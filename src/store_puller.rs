use crate::config::Config;
use crate::database::{cursor_to_vector, Database, StoreOwner};
use crate::log::{Log, LogFileSystem};
use crate::progress::Progress;
use crate::provider::Provider;
use crate::result::{BoxResult, UnexpectedError};
use crate::task_tracker::{GeneralTask, StoreTask, TaskTracker};
use crate::{error, info, store_service};
use chrono::Duration;
use mongodb::bson::doc;
use mongodb::error::Error;
use mongodb::options::FindOptions;
use mongodb::sync::{Collection, Cursor};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;

pub fn start(
    provider: &mut dyn Provider,
    database: &Database,
    config: &Config,
    log: &mut Log,
    log_fs: &LogFileSystem,
) -> BoxResult<()> {
    let mut local_log = Log::fork("store", log);
    let mut task: StoreTask = TaskTracker::read_task(database)?;
    if task.state().done {
        info!(local_log, "done");
        return Ok(());
    }
    force_start(
        &mut task,
        provider,
        database,
        config,
        &mut local_log,
        log_fs,
    )?;
    task.state_mut().done = true;
    TaskTracker::write_task(&task, database)?;
    info!(local_log, "done");
    return Ok(());
}

fn force_start(
    task: &mut StoreTask,
    provider: &mut dyn Provider,
    database: &Database,
    config: &Config,
    log: &mut Log,
    log_fs: &LogFileSystem,
) -> BoxResult<()> {
    let iterator = prepare_stores_for_pulling(task, database)?;
    pull_stores(task, iterator, provider, database, config, log, log_fs)?;
    return Ok(());
}

fn prepare_stores_for_pulling<'a>(
    task: &mut StoreTask,
    database: &'a Database,
) -> BoxResult<StoreOwnerIterator<'a>> {
    if task.state().store_owner_prepared == false {
        merge_owners_from_products(database)?;
        task.state_mut().store_owner_prepared = true;
        TaskTracker::write_task(task, database)?;
    }
    return StoreOwnerIterator::new(task.state().store_owner_identity, database);
}

fn merge_owners_from_products(database: &Database) -> BoxResult<()> {
    let pipeline = vec![
        doc! {
            "$project": {
                "_id": "$owner_identity"
            }
        },
        doc! {
            "$merge": {
                "into": "tmp_store_owner",
                "on": "_id",
                "whenMatched": "keepExisting",
                "whenNotMatched": "insert"
            }
        },
    ];
    database.product_collection.aggregate(pipeline, None)?;
    return Ok(());
}

fn pull_stores(
    task: &mut StoreTask,
    store_owner_it: StoreOwnerIterator,
    provider: &mut dyn Provider,
    database: &Database,
    config: &Config,
    log: &mut Log,
    log_fs: &LogFileSystem,
) -> BoxResult<()> {
    let mut progress = Progress::new(
        store_owner_it.finished_count,
        store_owner_it.total_count,
        Duration::seconds(20),
    )?;
    info!(
        log,
        "restored, owner={}, {:.2}%, {} minutes left",
        task.state().store_owner_identity,
        progress.percentage(),
        progress.estimated().num_minutes()
    );
    for owner_result in store_owner_it {
        let owner = match owner_result {
            Err(e) => return Err(e),
            Ok(v) => v,
        };
        let store_id = match pull_store(owner.identity, provider, database, config, log, log_fs)? {
            None => "skip".to_string(),
            Some(v) => v.to_string(),
        };
        task.state_mut().store_owner_identity = owner.identity;
        TaskTracker::write_task(task, database)?;
        progress.tick();
        info!(
            log,
            "new store={}, owner={}, {:.2}%, {} minutes left",
            store_id,
            owner.identity,
            progress.percentage(),
            progress.estimated().num_minutes()
        );
    }
    return Ok(());
}

fn pull_store(
    store_owner_id: u64,
    provider: &mut dyn Provider,
    database: &Database,
    config: &Config,
    log: &mut Log,
    log_fs: &LogFileSystem,
) -> BoxResult<Option<u64>> {
    loop {
        let store = match provider.get_store_by_owner_id(store_owner_id) {
            Err(e) => {
                let log_path = config.log_directory.join("pull_store_error");
                e.write(&log_path)?;
                error!(log, "check {}", log_path.to_string_lossy());
                if e.skip {
                    error!(
                        log,
                        "skip owner_identity={} because of empty page", store_owner_id
                    );
                    return Ok(None);
                }
                continue;
            }
            Ok(v) => v,
        };
        store_service::update(&store, database)?;
        return Ok(Some(store.identity));
    }
}

struct StoreOwnerIterator<'a> {
    collection: &'a Collection<StoreOwner>,
    from_owner_identity: u64,
    next_from_owner_identity: u64,
    items: VecDeque<StoreOwner>,
    total_count: u64,
    finished_count: u64,
}

impl<'a> StoreOwnerIterator<'a> {
    pub fn new(from_owner_id: u64, database: &'a Database) -> BoxResult<Self> {
        let (total_count, finished_count) =
            Self::count_items(from_owner_id, &database.tmp_store_owner_collection)?;
        let instance = Self {
            collection: &database.tmp_store_owner_collection,
            from_owner_identity: from_owner_id,
            next_from_owner_identity: from_owner_id,
            items: VecDeque::new(),
            total_count,
            finished_count,
        };
        return Ok(instance);
    }

    fn load_items(&mut self) -> BoxResult<Vec<StoreOwner>> {
        let filter = doc! {"_id": {"$gte": self.next_from_owner_identity as f64}};
        let sort = doc! {"_id": 1};
        let options = FindOptions::builder().sort(sort).limit(128).build();
        let mut cursor = self.collection.find(filter, options)?;
        let items = cursor_to_vector(&mut cursor)?;
        return Ok(items);
    }

    fn count_items(
        from_identity: u64,
        collection: &Collection<StoreOwner>,
    ) -> BoxResult<(u64, u64)> {
        let finished_filter = doc! {"_id": {"$lt": from_identity as f64}};
        let finished_count = collection.count_documents(finished_filter, None)?;
        let total_count = collection.count_documents(None, None)?;
        return Ok((total_count, finished_count));
    }
}

impl<'a> Iterator for StoreOwnerIterator<'a> {
    type Item = BoxResult<StoreOwner>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.items.len() == 0 {
            let next_items = match self.load_items() {
                Err(e) => return Some(Err(e)),
                Ok(v) => v,
            };
            match next_items.last() {
                None => {}
                Some(v) => self.next_from_owner_identity = v.identity + 1,
            }
            self.items = VecDeque::from_iter(next_items);
        }
        return match self.items.pop_front() {
            None => return None,
            Some(v) => Some(Ok(v)),
        };
    }
}

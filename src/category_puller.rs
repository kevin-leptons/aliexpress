use crate::config::Config;
use crate::database::{cursor_to_vector, Database};
use crate::log::Log;
use crate::progress::Progress;
use crate::provider::{Category, Provider};
use crate::result::{BoxResult, PullError, PullErrorKind, PullResult};
use crate::task_tracker::GeneralTask;
use crate::task_tracker::{CategoryTask, TaskTracker};
use crate::{category_service, error, info};
use chrono::Duration;
use mongodb::bson::doc;
use mongodb::options::FindOptions;

pub fn start(
    config: &Config,
    provider: &mut dyn Provider,
    database: &Database,
    log: &mut Log,
) -> BoxResult<()> {
    let mut local_log = Log::fork("category", log);
    let mut task: CategoryTask = TaskTracker::read_task(database)?;
    if task.state().done {
        info!(local_log, "done");
        return Ok(());
    }
    if config.pulling.category.len() == 0 {
        pull_categories_from_provider(&mut local_log, config, &mut task, provider, database)?;
    } else {
        put_categories_to_database(
            &mut local_log,
            config,
            &config.pulling.category,
            provider,
            database,
        )?;
    }

    task.state_mut().done = true;
    TaskTracker::write_task(&task, database)?;
    info!(local_log, "done");
    return Ok(());
}

fn put_categories_to_database(
    log: &mut Log,
    config: &Config,
    category_identities: &Vec<u64>,
    provider: &mut dyn Provider,
    database: &Database,
) -> BoxResult<()> {
    for id in category_identities {
        put_category_to_database(log, config, *id, provider, database)?;
    }
    info!(
        log,
        "put categories into database: {} items",
        category_identities.len()
    );
    return Ok(());
}

fn put_category_to_database(
    log: &mut Log,
    config: &Config,
    category_identity: u64,
    provider: &mut dyn Provider,
    database: &Database,
) -> BoxResult<()> {
    loop {
        let category = match provider.get_category(category_identity) {
            Err(e) => {
                if e.skip {
                    error!(log, "skip because no data, identity={}", category_identity);
                    return Ok(());
                }
                let directory_path = config.log_directory.join("pull_category_error");
                e.write(&directory_path)?;
                error!(log, "check: {}", directory_path.to_string_lossy());
                continue;
            }
            Ok(v) => v,
        };
        category_service::put(&category, database)?;
        info!(
            log,
            "new category, identity={}, name={}", category.identity, category.name
        );
        return Ok(());
    }
}

fn pull_categories_from_provider(
    log: &mut Log,
    config: &Config,
    task: &mut CategoryTask,
    provider: &mut dyn Provider,
    database: &Database,
) -> BoxResult<()> {
    if task.state().prepared == false {
        prepare_level_1_and_2_items(task, provider, database)?;
        info!(log, "prepare level 1 and 2 items: done");
    }
    pull_level_3_items(log, config, task, provider, database)?;
    return Ok(());
}

fn prepare_level_1_and_2_items(
    task: &mut CategoryTask,
    provider: &mut dyn Provider,
    database: &Database,
) -> PullResult<()> {
    static STEP: &str = "prepare_level_1_and_2_items";
    let categories = match provider.get_level_1_2_categories() {
        Err(e) => return e.stack_step(STEP).to_result(),
        Ok(v) => v,
    };
    match category_service::update_many(&categories, database) {
        Err(e) => {
            return PullError::from_step(STEP, PullErrorKind::Database)
                .set_message(e.to_string().as_str())
                .to_result()
        }
        Ok(_) => {}
    };
    task.state_mut().prepared = true;
    match TaskTracker::write_task(task, database) {
        Err(e) => {
            return PullError::from_step(STEP, PullErrorKind::Database)
                .set_message(e.to_string().as_str())
                .to_result()
        }
        Ok(_) => {}
    }
    return Ok(());
}

fn pull_level_3_items(
    log: &mut Log,
    config: &Config,
    task: &mut CategoryTask,
    provider: &mut dyn Provider,
    database: &Database,
) -> PullResult<()> {
    static STEP: &str = "pull_level_3_items";
    let (mut progress, parent_categories) = match restore_parent_categories(task, database) {
        Err(e) => return e.stack_step(STEP).to_result(),
        Ok(v) => v,
    };
    info!(
        log,
        "restored, {:.2}%, {} minutes left",
        progress.percentage(),
        progress.estimated().num_minutes()
    );
    for category in parent_categories {
        let items = pull_level_3_items_by_level_2(category.identity, log, config, provider);
        category_service::update_many(&items, database).unwrap();
        task.state_mut().from_identity = category.identity;
        TaskTracker::write_task(task, database).unwrap();
        progress.tick();
        info!(
            log,
            "new {} items, parent_identity={}, {:.2}%, {} minutes left",
            items.len(),
            category.identity,
            progress.percentage(),
            progress.estimated().num_minutes()
        );
    }
    return Ok(());
}

fn restore_parent_categories(
    task: &CategoryTask,
    database: &Database,
) -> PullResult<(Progress, Vec<Category>)> {
    static STEP: &str = "restore_parent_categories";
    let filter = doc! {
        "_id": {
            "$gt": task.state().from_identity as u32
        },
        "level": 2
    };
    let options = FindOptions::builder().sort(doc! {"_id": 1}).build();
    let mut cursor = match database.category_collection.find(filter, options) {
        Err(_) => {
            return PullError::from_step(STEP, PullErrorKind::Database)
                .set_message("can read load data")
                .to_result()
        }
        Ok(v) => v,
    };
    let items = match cursor_to_vector(&mut cursor) {
        Err(_) => {
            return PullError::from_step(STEP, PullErrorKind::Database)
                .set_message("can not deserialize data")
                .to_result()
        }
        Ok(v) => v,
    };
    let total_count = match database.category_collection.count_documents(doc! {}, None) {
        Err(_) => {
            return PullError::from_step(STEP, PullErrorKind::Database)
                .set_message("can not count data")
                .to_result()
        }
        Ok(v) => v,
    };
    let progress = Progress::new(
        total_count - (items.len() as u64),
        total_count,
        Duration::seconds(20),
    )
    .unwrap();
    return Ok((progress, items));
}

fn pull_level_3_items_by_level_2(
    level_2_identity: u64,
    log: &mut Log,
    config: &Config,
    provider: &mut dyn Provider,
) -> Vec<Category> {
    loop {
        let items = match provider.get_level_3_categories(level_2_identity) {
            Err(e) => {
                let log_path = config.log_directory.join("pull_category_error");
                e.write(&log_path).unwrap();
                error!(log, "check: {}", log_path.to_string_lossy());
                continue;
            }
            Ok(v) => v,
        };
        return items;
    }
}

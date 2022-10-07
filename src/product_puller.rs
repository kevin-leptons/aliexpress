use crate::category_service::CategoryIterator;
use crate::config::Config;
use crate::database::Database;
use crate::log::{Log, LogFileSystem};
use crate::progress::Progress;
use crate::provider::{Category, Product, Provider};
use crate::result::BoxResult;
use crate::task_tracker::{CategoryTask, GeneralTask, ProductTask, TaskTracker};
use crate::{category_service, error, info, product_service};
use chrono::Duration;
use mongodb::options::ReturnDocument;
use std::error::Error;
use std::path::Path;

pub fn start(
    provider: &mut dyn Provider,
    database: &Database,
    config: &Config,
    log: &mut Log,
    log_fs: &LogFileSystem,
) -> BoxResult<()> {
    let mut local_log = Log::fork("product", log);
    let mut task: ProductTask = TaskTracker::read_task(database)?;
    if task.state().done == false {
        force_start(
            &mut task,
            provider,
            database,
            config,
            &mut local_log,
            log_fs,
        )?;
    }
    task.state_mut().done = true;
    TaskTracker::write_task(&task, database)?;
    info!(local_log, "done");
    return Ok(());
}

pub fn force_start(
    task: &mut ProductTask,
    provider: &mut dyn Provider,
    database: &Database,
    config: &Config,
    log: &mut Log,
    log_fs: &LogFileSystem,
) -> BoxResult<()> {
    let category_iterator = category_service::iterator(database, task.state().category_identity)?;
    let mut progress = initialize_progress(config, &category_iterator, task)?;
    info!(
        log,
        "restored, identity={}, page={}, {:.2}%, {} minutes left",
        task.state().category_identity,
        task.state().category_page_index,
        progress.percentage(),
        progress.estimated().num_minutes()
    );
    for category_result in category_iterator {
        let category = match category_result {
            Err(e) => return Err(Box::new(e)),
            Ok(v) => v,
        };
        task.state_mut().category_identity = category.identity;
        pull_products_from_category(
            &category,
            task,
            provider,
            database,
            config,
            log,
            log_fs,
            &mut progress,
        )?;
        TaskTracker::write_task(task, database)?;
        task.state_mut().category_page_index = 1;
    }
    return Ok(());
}

fn initialize_progress(
    config: &Config,
    category_iterator: &CategoryIterator,
    task: &ProductTask,
) -> BoxResult<Progress> {
    let estimated_task_time = Duration::seconds(30);
    let category_pages = config.pulling.category_pages as u64;
    let finished_count = category_iterator.finished_count() * category_pages
        + task.state().category_page_index as u64;
    let total_count = category_iterator.total_count() * category_pages;
    return Progress::new(finished_count, total_count, estimated_task_time);
}

fn pull_products_from_category(
    category: &Category,
    task: &mut ProductTask,
    provider: &mut dyn Provider,
    database: &Database,
    config: &Config,
    log: &mut Log,
    log_fs: &LogFileSystem,
    progress: &mut Progress,
) -> BoxResult<()> {
    let begin_page_index = task.state().category_page_index;
    for page_index in begin_page_index..(config.pulling.category_pages + 1) {
        pull_products_from_category_page(
            category, page_index, provider, database, config, log, log_fs,
        )?;
        task.state_mut().category_page_index = page_index;
        TaskTracker::write_task(task, database)?;
        progress.tick();
        info!(
            log,
            "category={}, page_index={}, {:.2}%, {} minutes left",
            category.identity,
            task.state().category_page_index,
            progress.percentage(),
            progress.estimated().num_minutes()
        );
    }
    return Ok(());
}

fn pull_products_from_category_page(
    category: &Category,
    category_page_index: u32,
    provider: &mut dyn Provider,
    database: &Database,
    config: &Config,
    log: &mut Log,
    log_fs: &LogFileSystem,
) -> BoxResult<()> {
    let products = try_get_products(category, category_page_index, provider, config, log, log_fs)?;
    product_service::update_many(&products, database)?;
    return Ok(());
}
fn try_get_products(
    category: &Category,
    category_page_index: u32,
    provider: &mut dyn Provider,
    config: &Config,
    log: &mut Log,
    log_fs: &LogFileSystem,
) -> BoxResult<Vec<Product>> {
    loop {
        let mut products = match provider.get_products(category.identity, category_page_index) {
            Err(e) => {
                let log_path = config.log_directory.join("pull_product_error");
                e.write(log_path.as_path())?;
                if e.skip {
                    error!(
                        log,
                        "skip {}/{} because: {}",
                        category.identity,
                        category_page_index,
                        e.description()
                    );
                    return Ok(Vec::new());
                }
                error!(
                    log,
                    "error occurred, retry after few seconds; log: {}",
                    log_path.to_string_lossy()
                );
                continue;
            }
            Ok(v) => v,
        };
        return Ok(products);
    }
}

use crate::aliexpress_provider::AliexpressProvider;
use crate::config::Config;
use crate::database::Database;
use crate::log::{Log, LogFileSystem};
use crate::provider::{Category, Product, Provider};
use crate::result::{BoxResult, PullError, PullErrorKind, PullResult, UnexpectedError};
use crate::task_tracker::{
    CategoryState, CategoryTask, GeneralTask, ProductTask, StoreTask, TaskTracker,
};
use crate::{
    category_puller, category_service, error, info, product_puller, product_service, store_puller,
};
use mongodb::bson::doc;
use num_format::Locale::to;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

pub struct DataPuller<'a> {
    config: Config,
    database: Database,
    aliexpress_provider: AliexpressProvider,
    log: Log<'a>,
    log_fs: LogFileSystem,
}

impl<'a> DataPuller<'a> {
    pub fn new(config: Config, log: &'a mut Log<'a>, log_fs: LogFileSystem) -> BoxResult<Self> {
        let database = Database::new(config.mongo_endpoint.as_str())?;
        let aliexpress_provider = AliexpressProvider::new();
        let worker = DataPuller {
            config,
            database,
            aliexpress_provider,
            log: Log::fork("pull", log),
            log_fs,
        };
        return Ok(worker);
    }

    pub fn run(&mut self) -> BoxResult<()> {
        pull_data_from_provider(
            &mut self.aliexpress_provider,
            &self.database,
            &self.config,
            &mut self.log,
            &self.log_fs,
        )?;
        return Ok(());
    }

    pub fn clear(&mut self) -> BoxResult<()> {
        let query = doc! {};
        self.database
            .task_collection
            .delete_many(query.clone(), None)?;
        self.database
            .tmp_store_owner_collection
            .delete_many(query.clone(), None)?;
        self.database
            .store_collection
            .delete_many(query.clone(), None)?;
        self.database
            .product_collection
            .delete_many(query.clone(), None)?;
        self.database
            .category_collection
            .delete_many(query.clone(), None)?;
        info!(self.log, "cleared!");
        return Ok(());
    }
}

fn pull_data_from_provider(
    provider: &mut dyn Provider,
    database: &Database,
    config: &Config,
    log: &mut Log,
    log_fs: &LogFileSystem,
) -> BoxResult<()> {
    category_puller::start(config, provider, database, log)?;
    product_puller::start(provider, database, config, log, log_fs)?;
    store_puller::start(provider, database, config, log, log_fs)?;
    return Ok(());
}

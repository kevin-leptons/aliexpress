use crate::analyst::aggregator::Aggregator;
use crate::analyst::miner::Miner;
use crate::analyst::model::AnalysisModel;
use crate::analyst::reporter::Reporter;
use crate::config::Config;
use crate::database::Database;
use crate::info;
use crate::log::Log;
use crate::result::BoxResult;
use chrono::{DateTime, NaiveDateTime, Utc};
use std::io::stdout;

mod aggregator;
mod formatter;
mod miner;
mod model;
mod reporter;

#[derive()]
pub struct Analyst<'a> {
    log: Log<'a>,
    config: &'a Config,
    reporter: Reporter<'a>,
    aggregator: Aggregator<'a>,
    database: Database,
    miner: Miner<'a>,
}

impl<'a> Analyst<'a> {
    pub fn new(log: &'a mut Log, config: &'a Config) -> BoxResult<Self> {
        let instance = Self {
            log: Log::fork("analyst", log),
            config,
            database: Database::new(config.mongo_endpoint.as_str())?,
            reporter: Reporter::new(config)?,
            aggregator: Aggregator::new(config),
            miner: Miner::new(config),
        };
        return Ok(instance);
    }

    pub fn start(&mut self) -> BoxResult<()> {
        let started_at = Utc::now().naive_utc();
        make_distribution_report(
            &mut self.log,
            &self.aggregator,
            &self.reporter,
            &self.database,
        )?;
        make_timeline_report(
            &mut self.log,
            &self.aggregator,
            &self.reporter,
            &self.database,
        )?;
        make_top_report(
            &mut self.log,
            &self.aggregator,
            &self.reporter,
            &self.database,
        )?;
        make_prospect_reports(&mut self.log, &self.miner, &self.reporter, &self.database)?;
        make_index_page(
            &mut self.log,
            &self.aggregator,
            &self.reporter,
            &self.database,
            started_at,
        )?;
        return Ok(());
    }
}

fn make_index_page(
    log: &mut Log,
    aggregator: &Aggregator,
    reporter: &Reporter,
    database: &Database,
    started_at: NaiveDateTime,
) -> BoxResult<()> {
    let finished_at = Utc::now().naive_utc();
    let model = AnalysisModel {
        started_at: started_at,
        finished_in: finished_at - started_at,
        database: aggregator.get_database_summary(database)?,
        platform: format!(
            "{} {}",
            std::env::consts::OS.to_string(),
            std::env::consts::ARCH.to_string()
        ),
    };
    let report_path = reporter.make_index_report(&model)?;
    info!(log, "new index: {}", report_path.to_string_lossy());
    return Ok(());
}

fn make_timeline_report(
    log: &mut Log,
    aggregator: &Aggregator,
    reporter: &Reporter,
    database: &Database,
) -> BoxResult<()> {
    let store_online_model = aggregator.get_store_online_timeline(database)?;
    let report_path = reporter.make_timeline_report(&store_online_model)?;
    info!(log, "timeline report: {}", report_path.to_string_lossy());
    return Ok(());
}

fn make_top_report(
    log: &mut Log,
    aggregator: &Aggregator,
    reporter: &Reporter,
    database: &Database,
) -> BoxResult<()> {
    let products_by_rating = aggregator.get_top_products_by_rating(database)?;
    let products_by_orders = aggregator.get_top_products_by_orders(database)?;
    let products_by_revenue = aggregator.get_top_products_by_revenue(database)?;
    let products_by_highest_price = aggregator.get_top_products_by_highest_price(database)?;
    let products_by_lowest_price = aggregator.get_top_products_by_lowest_price(database)?;
    let stores_by_revenue = aggregator.get_top_stores_by_revenue(database)?;
    let stores_by_orders = aggregator.get_top_stores_by_orders(database)?;
    let stores_by_online_time = aggregator.get_top_stores_by_online_time(database)?;
    let level_1_categories_by_revenue = aggregator.get_top_categories_by_revenue(
        1,
        "Top Level 1 Categories by Revenue".to_string(),
        database,
    )?;
    let level_2_categories_by_revenue = aggregator.get_top_categories_by_revenue(
        2,
        "Top Level 2 Categories by Revenue".to_string(),
        database,
    )?;
    let report_path = reporter.make_top_report(
        &products_by_rating,
        &products_by_orders,
        &products_by_revenue,
        &products_by_highest_price,
        &products_by_lowest_price,
        &stores_by_revenue,
        &stores_by_orders,
        &stores_by_online_time,
        &level_1_categories_by_revenue,
        &level_2_categories_by_revenue,
    )?;
    info!(log, "top report: {}", report_path.to_string_lossy());
    return Ok(());
}

fn make_distribution_report(
    log: &mut Log,
    aggregator: &Aggregator,
    reporter: &Reporter,
    database: &Database,
) -> BoxResult<()> {
    let products_by_rating = aggregator.get_distribution_products_by_rating(database)?;
    let products_by_orders = aggregator.get_distribution_products_by_orders(database)?;
    let products_by_price = aggregator.get_distribution_products_by_price(database)?;
    let products_by_shipping_fee =
        aggregator.get_distribution_products_by_shipping_fee(database)?;
    let stores_by_rating = aggregator.get_distribution_stores_by_rating(database)?;
    let stores_by_rating45_count =
        aggregator.get_distribution_stores_by_rating45_count(database)?;
    let stores_by_rating12_count =
        aggregator.get_distribution_stores_by_rating12_count(database)?;
    let stores_by_rating3_count = aggregator.get_distribution_stores_by_rating3_count(database)?;
    let revenue_by_rating = aggregator.get_distribution_revenue_by_rating(database)?;
    let revenue_by_price = aggregator.get_distribution_revenue_by_price(database)?;
    let orders_by_price = aggregator.get_distribution_orders_by_price(database)?;
    let report_path = reporter.make_distribution_report(
        &products_by_rating,
        &products_by_orders,
        &products_by_price,
        &products_by_shipping_fee,
        &stores_by_rating,
        &stores_by_rating45_count,
        &stores_by_rating12_count,
        &stores_by_rating3_count,
        &revenue_by_rating,
        &revenue_by_price,
        &orders_by_price,
    )?;
    info!(
        log,
        "distribution report: {}",
        report_path.to_string_lossy()
    );
    return Ok(());
}

fn make_prospect_reports(
    log: &mut Log,
    miner: &Miner,
    reporter: &Reporter,
    database: &Database,
) -> BoxResult<()> {
    let products_by_points = miner.get_prospect_products_by_points(database)?;
    let report_path = reporter.make_prospect_reports(&products_by_points)?;
    info!(log, "prospect report: {}", report_path.to_string_lossy());
    return Ok(());
}

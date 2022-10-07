use crate::result::{BoxResult, PullResult};
use chrono::naive::serde::ts_seconds::deserialize as from_seconds_ts;
use chrono::naive::serde::ts_seconds::serialize as to_seconds_ts;
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use url::{ParseError, Url};

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr)]
#[repr(u16)]
pub enum ProviderIdentity {
    Aliexpress = 1,
    Ebay = 2,
    Amazon = 3,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Timestamp {
    pub value: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Category {
    #[serde(rename = "_id")]
    pub identity: u64,
    pub parent_identity: Option<u64>,
    pub name: String,
    pub name_url: String,
    pub level: u32,
}
pub type CompactProductIteratorResult = BoxResult<CompactProduct>;
pub type CategoryIteratorResult = PullResult<Category>;

impl Timestamp {
    pub fn now() -> BoxResult<Timestamp> {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Err(e) => return Err(Box::new(e)),
            Ok(v) => {
                let t = Timestamp { value: v.as_secs() };
                return Ok(t);
            }
        };
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    pub provider_identity: ProviderIdentity,

    #[serde(rename = "_id")]
    pub identity: u64,
    pub name: String,
    pub price: f64,
    pub cost: f64,
    pub image_url: Url,
    pub store_name: String,
    pub store_identity: u64,
    pub owner_identity: u64,
    pub category_identity: u64,

    pub orders: Option<u64>,
    pub shipping_fee: Option<f64>,
    pub rating: Option<f64>,
    pub revenue: Option<f64>,
}

impl PartialEq for Product {
    fn eq(&self, other: &Self) -> bool {
        return self.identity == other.identity && self.name == other.name;
    }

    fn ne(&self, other: &Self) -> bool {
        return self.identity != other.identity || self.name != other.name;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompactProduct {
    pub provider_identity: ProviderIdentity,
    pub identity: u64,
    pub name: String,
    pub price: f64,
    pub image_url: Url,
    pub store_name: String,
    pub store_identity: u64,

    pub orders: Option<u64>,
    pub shipping_fee: Option<f64>,
    pub rating: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Store {
    #[serde(rename = "_id")]
    pub identity: u64,
    pub owner_identity: u64,
    pub name: String,
    pub rating45_ratio: f64,
    pub rating45_count: u64,
    pub rating3_count: u64,
    pub rating12_count: u64,

    #[serde(serialize_with = "to_seconds_ts", deserialize_with = "from_seconds_ts")]
    pub online_at: NaiveDateTime,

    #[serde(serialize_with = "to_seconds_ts", deserialize_with = "from_seconds_ts")]
    pub modified_at: NaiveDateTime,
}

pub trait Provider {
    fn get_category(&mut self, identity: u64) -> PullResult<Category>;
    fn get_product(&mut self, link: String) -> BoxResult<Product>;
    fn get_store(&mut self, identity: u64) -> BoxResult<Store>;
    fn get_store_by_owner_id(&mut self, owner_id: u64) -> PullResult<Store>;
    fn get_products(&mut self, category_identity: u64, page_index: u32)
        -> PullResult<Vec<Product>>;
    fn get_level_1_2_categories(&mut self) -> PullResult<Vec<Category>>;
    fn get_level_3_categories(&mut self, parent_identity: u64) -> PullResult<Vec<Category>>;
}

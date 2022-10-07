use crate::analyst::formatter;
use crate::provider::{Product, ProviderIdentity, Store};
use chrono::naive::serde::ts_seconds::deserialize as from_seconds_ts;
use chrono::naive::serde::ts_seconds::serialize as to_seconds_ts;
use chrono::{Date, DateTime, Duration, FixedOffset, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

pub struct ProductScore {
    pub potential: u64,
    pub popular: u64,
}

#[derive(Debug, Serialize)]
pub struct ProductRatingModel {
    pub count_1_2: u64,
    pub count_2_3: u64,
    pub count_3_4: u64,
    pub count_4_5: u64,
    pub count_5: u64,
    pub unknown_count: u64,
}

#[derive(Debug)]
pub struct ProductOrdersModel {
    pub count_0: u64,
    pub count_1_9: u64,
    pub count_10_19: u64,
    pub count_20_49: u64,
    pub count_50_99: u64,
    pub count_100_499: u64,
    pub count_500_999: u64,
    pub count_1000_9999: u64,
    pub count_10000_n: u64,
    pub unknown_count: u64,
}

#[derive(Debug)]
pub struct ProductPriceModel {
    pub count_0_1: u64,
    pub count_1_5: u64,
    pub count_5_10: u64,
    pub count_10_20: u64,
    pub count_20_30: u64,
    pub count_30_50: u64,
    pub count_50_100: u64,
    pub count_100_1000: u64,
    pub count_1000_n: u64,
}

#[derive(Debug)]
pub struct ShippingFeeModel {
    pub count_0: u64,
    pub count_0_1: u64,
    pub count_1_5: u64,
    pub count_5_10: u64,
    pub count_10_20: u64,
    pub count_20_30: u64,
    pub count_30_50: u64,
    pub count_50_100: u64,
    pub count_100_n: u64,
    pub unknown_count: u64,
}

#[derive(Debug, Serialize)]
pub struct DatabaseSummaryModel {
    #[serde(with = "formatter::number")]
    pub product_count: u64,

    #[serde(with = "formatter::number")]
    pub store_count: u64,

    #[serde(with = "formatter::number")]
    pub category_count: u64,
}

impl Clone for DatabaseSummaryModel {
    fn clone(&self) -> Self {
        return Self {
            product_count: self.product_count,
            store_count: self.store_count,
            category_count: self.category_count,
        };
    }
}

pub struct StoreRatingModel {
    pub count_0_10: u64,
    pub count_10_30: u64,
    pub count_30_50: u64,
    pub count_50_80: u64,
    pub count_80_90: u64,
    pub count_90_95: u64,
    pub count_95_96: u64,
    pub count_96_97: u64,
    pub count_97_98: u64,
    pub count_98_99: u64,
    pub count_99_100: u64,
    pub unknown_count: u64,
}

pub struct StorePositiveRatingModel {
    pub count_0_10: u64,
    pub count_10_20: u64,
    pub count_20_30: u64,
    pub count_30_50: u64,
    pub count_50_100: u64,
    pub count_100_500: u64,
    pub count_500_1000: u64,
    pub count_1000_2000: u64,
    pub count_2000_3000: u64,
    pub count_3000_5000: u64,
    pub count_5000_n: u64,
}

pub struct StoreNegativeRatingModel {
    pub count_0_10: u64,
    pub count_10_20: u64,
    pub count_20_30: u64,
    pub count_30_50: u64,
    pub count_50_100: u64,
    pub count_100_500: u64,
    pub count_500_1000: u64,
    pub count_1000_2000: u64,
    pub count_2000_3000: u64,
    pub count_3000_5000: u64,
    pub count_5000_n: u64,
}

pub struct StoreNeutralRatingModel {
    pub count_0_10: u64,
    pub count_10_20: u64,
    pub count_20_30: u64,
    pub count_30_50: u64,
    pub count_50_100: u64,
    pub count_100_500: u64,
    pub count_500_1000: u64,
    pub count_1000_2000: u64,
    pub count_2000_3000: u64,
    pub count_3000_5000: u64,
    pub count_5000_n: u64,
}

pub type StoreOnlineModel = Vec<StoreOnlinePoint>;

pub struct StoreOnlinePoint {
    pub timestamp: NaiveDateTime,
    pub count: u64,
}

pub struct RevenueByRatingModel {
    pub rating_5: f64,
    pub rating_4_5: f64,
    pub rating_3_4: f64,
    pub rating_2_3: f64,
    pub rating_1_2: f64,
    pub unknown: f64,
}

pub struct RevenueByPriceModel {
    pub price_0_1: f64,
    pub price_1_5: f64,
    pub price_5_10: f64,
    pub price_10_20: f64,
    pub price_20_30: f64,
    pub price_30_50: f64,
    pub price_50_100: f64,
    pub price_100_200: f64,
    pub price_200_500: f64,
    pub price_500_700: f64,
    pub price_700_1000: f64,
    pub price_1000_n: f64,
}

pub struct OrdersByPriceModel {
    pub price_0_1: u64,
    pub price_1_5: u64,
    pub price_5_10: u64,
    pub price_10_20: u64,
    pub price_20_30: u64,
    pub price_30_50: u64,
    pub price_50_100: u64,
    pub price_100_200: u64,
    pub price_200_500: u64,
    pub price_500_700: u64,
    pub price_700_1000: u64,
    pub price_1000_n: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ProductModel {
    #[serde(rename = "_id")]
    pub identity: u64,
    pub name: String,
    pub price: f64,
    pub image_url: Url,
    pub store_name: String,
    pub store_identity: u64,
    pub owner_identity: u64,

    pub orders: Option<u64>,
    pub shipping_fee: Option<f64>,
    pub rating: Option<f64>,

    pub revenue: Option<f64>,
    pub points: f64,
}

impl ProductModel {
    pub fn new(product: &Product) -> Self {
        return Self {
            identity: product.identity,
            name: product.name.clone(),
            price: product.price,
            image_url: product.image_url.clone(),
            store_name: product.store_name.clone(),
            store_identity: product.store_identity,
            owner_identity: product.owner_identity,
            orders: product.orders,
            shipping_fee: product.shipping_fee,
            rating: product.rating,
            revenue: product.revenue,
            points: 0.0,
        };
    }
}

#[derive(Serialize)]
pub struct TopProductModel {
    pub items: Vec<ProductModel>,
    pub description: String,
}

#[derive(Serialize)]
pub struct AnalysisModel {
    pub database: DatabaseSummaryModel,

    #[serde(with = "formatter::datetime")]
    pub started_at: NaiveDateTime,

    #[serde(with = "formatter::duration")]
    pub finished_in: Duration,

    pub platform: String,
}

#[derive(Serialize, Deserialize)]
pub struct StoreModel {
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

    pub orders: u64,
    pub revenue: f64,
}

impl StoreModel {
    pub fn new(store: &Store, revenue: f64, orders: u64) -> Self {
        return Self {
            identity: store.identity,
            owner_identity: store.owner_identity,
            name: store.name.clone(),
            rating45_ratio: store.rating45_ratio,
            rating45_count: store.rating45_count,
            rating3_count: store.rating3_count,
            rating12_count: store.rating12_count,
            online_at: store.online_at,
            revenue: revenue,
            orders: orders,
        };
    }
}

#[derive(Serialize)]
pub struct TopStoreModel {
    pub items: Vec<StoreModel>,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct CategoryModel {
    #[serde(rename = "_id")]
    pub identity: u64,
    pub name: String,
    pub revenue: f64,
    pub orders: u64,
}

#[derive(Serialize, Deserialize)]
pub struct TopCategoryModel {
    pub items: Vec<CategoryModel>,
    pub description: String,
}

#[derive(Serialize)]
pub struct ProspectProductModel {
    pub items: Vec<ProductModel>,
    pub description: String,
}

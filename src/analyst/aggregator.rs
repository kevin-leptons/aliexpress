use crate::analyst::model::{
    CategoryModel, DatabaseSummaryModel, OrdersByPriceModel, ProductModel, ProductOrdersModel,
    ProductPriceModel, ProductRatingModel, ProspectProductModel, RevenueByPriceModel,
    RevenueByRatingModel, ShippingFeeModel, StoreModel, StoreNegativeRatingModel,
    StoreNeutralRatingModel, StoreOnlineModel, StoreOnlinePoint, StorePositiveRatingModel,
    StoreRatingModel, TopCategoryModel, TopProductModel, TopStoreModel,
};
use crate::config::Config;
use crate::database::{cursor_to_vector, deserialize_documents, Database};
use crate::log::Log;
use crate::provider::Product;
use crate::result::BoxResult;
use chrono::{Date, DateTime, Datelike, Months, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use mongodb::bson::doc;
use mongodb::options::{FindOneOptions, FindOptions};
use mongodb::sync::Cursor;
use serde::Deserialize;
use std::ops::Sub;

pub struct Aggregator<'a> {
    config: &'a Config,
}

impl<'a> Aggregator<'a> {
    pub fn new(config: &'a Config) -> Self {
        return Self { config };
    }

    pub fn get_database_summary(&self, database: &Database) -> BoxResult<DatabaseSummaryModel> {
        let product_count = database.product_collection.count_documents(doc! {}, None)?;
        let store_count = database.store_collection.count_documents(doc! {}, None)?;
        let category_count = database
            .category_collection
            .count_documents(doc! {}, None)?;
        let model = DatabaseSummaryModel {
            product_count,
            store_count,
            category_count,
        };
        return Ok(model);
    }

    pub fn get_store_online_timeline(&self, database: &Database) -> BoxResult<StoreOnlineModel> {
        let mut model = vec![];
        let mut to = first_day_next_month();
        let mut from = previous_month(&to);
        for _ in 0..60 {
            let count = count_online_store(from, to, database)?;
            let point = StoreOnlinePoint {
                timestamp: from,
                count: count,
            };
            model.push(point);
            to = from;
            from = previous_month(&to);
        }
        return Ok(model);
    }

    pub fn get_top_products_by_rating(&self, database: &Database) -> BoxResult<TopProductModel> {
        let filter = doc! {};
        let sort = doc! {"rating": -1, "revenue": -1, "orders": -1, "price": 1, "shipping_fee": 1};
        let options = FindOptions::builder().sort(sort).limit(128).build();
        let mut cursor = database.product_collection.find(filter, options)?;
        let products = cursor_to_vector(&mut cursor)?;
        let items: Vec<ProductModel> = products.iter().map(|p| ProductModel::new(p)).collect();
        let model = TopProductModel {
            items,
            description: "Top Products by Rating".to_string(),
        };
        return Ok(model);
    }

    pub fn get_top_products_by_orders(&self, database: &Database) -> BoxResult<TopProductModel> {
        let filter = doc! {};
        let sort = doc! {"orders": -1, "revenue": -1, "price": 1, "rating": -1, "shipping_fee": 1};
        let options = FindOptions::builder().sort(sort).limit(128).build();
        let mut cursor = database.product_collection.find(filter, options)?;
        let products = cursor_to_vector(&mut cursor)?;
        let items: Vec<ProductModel> = products.iter().map(|p| ProductModel::new(p)).collect();
        let model = TopProductModel {
            items,
            description: "Top Products by Orders".to_string(),
        };
        return Ok(model);
    }

    pub fn get_top_products_by_revenue(&self, database: &Database) -> BoxResult<TopProductModel> {
        let filter = doc! {};
        let sort = doc! {"revenue": -1, "orders": -1, "price": 1, "rating": -1, "shipping_fee": 1};
        let options = FindOptions::builder().sort(sort).limit(128).build();
        let mut cursor = database.product_collection.find(filter, options)?;
        let products = cursor_to_vector(&mut cursor)?;
        let items: Vec<ProductModel> = products.iter().map(|p| ProductModel::new(p)).collect();
        let model = TopProductModel {
            items,
            description: "Top Products by Revenue".to_string(),
        };
        return Ok(model);
    }

    pub fn get_top_products_by_highest_price(
        &self,
        database: &Database,
    ) -> BoxResult<TopProductModel> {
        let filter = doc! {};
        let sort = doc! {"price": -1, "revenue": -1, "orders": -1, "rating": -1, "shipping_fee": 1};
        let options = FindOptions::builder().sort(sort).limit(128).build();
        let mut cursor = database.product_collection.find(filter, options)?;
        let products = cursor_to_vector(&mut cursor)?;
        let items: Vec<ProductModel> = products.iter().map(|p| ProductModel::new(p)).collect();
        let model = TopProductModel {
            items,
            description: "Top Products by Highest Price".to_string(),
        };
        return Ok(model);
    }

    pub fn get_top_products_by_lowest_price(
        &self,
        database: &Database,
    ) -> BoxResult<TopProductModel> {
        let filter = doc! {};
        let sort = doc! {"price": 1, "revenue": -1, "orders": -1, "rating": -1, "shipping_fee": 1};
        let options = FindOptions::builder().sort(sort).limit(128).build();
        let mut cursor = database.product_collection.find(filter, options)?;
        let products = cursor_to_vector(&mut cursor)?;
        let items: Vec<ProductModel> = products.iter().map(|p| ProductModel::new(p)).collect();
        let model = TopProductModel {
            items,
            description: "Top Products by Lowest Price".to_string(),
        };
        return Ok(model);
    }

    pub fn get_top_stores_by_revenue(&self, database: &Database) -> BoxResult<TopStoreModel> {
        let pipeline = vec![
            doc! {
                "$match": {
                    "revenue": {
                        "$ne": null
                    }
                }
            },
            doc! {
                "$group": {
                    "_id": "$store_identity",
                    "revenue": {
                        "$sum": "$revenue"
                    },
                    "orders": {
                        "$sum": "$orders"
                    }
                }
            },
            doc! {
                "$sort": {
                    "revenue": -1
                }
            },
            doc! {
                "$limit": 128
            },
            doc! {
                "$lookup": {
                    "from": "store",
                    "localField": "_id",
                    "foreignField": "_id",
                    "as": "store"
                }
            },
            doc! {
                "$match": {
                    "store": {
                        "$size": 1
                    }
                }
            },
            doc! {
                "$replaceRoot": {
                    "newRoot": {
                        "$mergeObjects": [
                            {"revenue": "$revenue", "orders": "$orders"},
                            {"$arrayElemAt": ["$store", 0]}
                        ]
                    }
                }
            },
        ];
        let mut cursor = database.product_collection.aggregate(pipeline, None)?;
        let documents = cursor_to_vector(&mut cursor)?;
        let stores = deserialize_documents(documents)?;
        let model = TopStoreModel {
            items: stores,
            description: "Top Stores by Revenue".to_string(),
        };
        return Ok(model);
    }

    pub fn get_top_stores_by_orders(&self, database: &Database) -> BoxResult<TopStoreModel> {
        let pipeline = vec![
            doc! {
                "$match": {
                    "orders": {
                        "$ne": null
                    }
                }
            },
            doc! {
                "$group": {
                    "_id": "$store_identity",
                    "revenue": {
                        "$sum": "$revenue"
                    },
                    "orders": {
                        "$sum": "$orders"
                    }
                }
            },
            doc! {
                "$sort": {
                    "orders": -1
                }
            },
            doc! {
                "$limit": 128
            },
            doc! {
                "$lookup": {
                    "from": "store",
                    "localField": "_id",
                    "foreignField": "_id",
                    "as": "store"
                }
            },
            doc! {
                "$match": {
                    "store": {
                        "$size": 1
                    }
                }
            },
            doc! {
                "$replaceRoot": {
                    "newRoot": {
                        "$mergeObjects": [
                            {"revenue": "$revenue", "orders": "$orders"},
                            {"$arrayElemAt": ["$store", 0]}
                        ]
                    }
                }
            },
        ];
        let mut cursor = database.product_collection.aggregate(pipeline, None)?;
        let documents = cursor_to_vector(&mut cursor)?;
        let stores = deserialize_documents(documents)?;
        let model = TopStoreModel {
            items: stores,
            description: "Top Stores by Orders".to_string(),
        };
        return Ok(model);
    }

    pub fn get_top_stores_by_online_time(&self, database: &Database) -> BoxResult<TopStoreModel> {
        let filter = doc! {};
        let options = FindOptions::builder()
            .sort(doc! {"online_at": -1})
            .limit(128)
            .build();
        let mut cursor = database.store_collection.find(filter, options)?;
        let stores = cursor_to_vector(&mut cursor)?;
        let store_identities = stores
            .iter()
            .map(|s| s.identity as f64)
            .collect::<Vec<f64>>();
        let pipeline = vec![
            doc! {
                "$match": {
                    "store_identity": {
                        "$in": store_identities
                    }
                }
            },
            doc! {
                "$group": {
                    "_id": "$store_identity",
                    "revenue": {
                        "$sum": "$revenue"
                    },
                    "orders": {
                        "$sum": "$orders"
                    }
                }
            },
            doc! {
                "$lookup": {
                    "from": "store",
                    "localField": "_id",
                    "foreignField": "_id",
                    "as": "store"
                }
            },
            doc! {
                "$match": {
                    "store": {
                        "$size": 1
                    }
                }
            },
            doc! {
                "$replaceRoot": {
                    "newRoot": {
                        "$mergeObjects": [
                            {"revenue": "$revenue", "orders": "$orders"},
                            {"$arrayElemAt": ["$store", 0]}
                        ]
                    }
                }
            },
            doc! {
                "$sort": {
                    "online_at": -1
                }
            },
        ];
        let mut cursor = database.product_collection.aggregate(pipeline, None)?;
        let documents = cursor_to_vector(&mut cursor)?;
        let items = deserialize_documents(documents)?;
        let model = TopStoreModel {
            items,
            description: "Top Store by Online Time".to_string(),
        };
        return Ok(model);
    }

    pub fn get_top_categories_by_revenue(
        &self,
        level: u32,
        description: String,
        database: &Database,
    ) -> BoxResult<TopCategoryModel> {
        let pipeline = vec![
            doc! {
              "$match": {
                    "revenue": {"$ne": null}
                }
            },
            doc! {
                "$group": {
                      "_id": "$category_identity",
                      "revenue": {
                        "$sum": "$revenue"
                      },
                      "orders": {
                        "$sum": "$orders"
                      }
                }
            },
            doc! {
                "$lookup": {
                  "from": "category",
                  "localField": "_id",
                  "foreignField": "_id",
                  "as": "category"
                }
            },
            doc! {
                "$match": {
                    "category": {"$size": 1},
                }
            },
            doc! {
                "$replaceRoot": {
                  "newRoot": {
                    "$mergeObjects": [
                      {"revenue": "$revenue", "orders": "$orders"},
                      {"$arrayElemAt": ["$category", 0]}
                    ]
                  }
                }
            },
            doc! {
                "$match": {
                    "level": level
                }
            },
            doc! {
                "$sort": {
                    "revenue": -1
                }
            },
            doc! {
                "$limit": 256
            },
        ];
        let mut cursor = database.product_collection.aggregate(pipeline, None)?;
        let documents = cursor_to_vector(&mut cursor)?;
        let items: Vec<CategoryModel> = deserialize_documents(documents)?;
        let model = TopCategoryModel {
            items: items,
            description: description,
        };
        return Ok(model);
    }

    pub fn get_distribution_products_by_rating(
        &self,
        database: &Database,
    ) -> BoxResult<ProductRatingModel> {
        let collection = &database.product_collection;
        let total_count = collection.count_documents(doc! {}, None)?;
        let count_1_2 = collection.count_documents(doc! {"rating": {"$gte": 1, "$lt": 2}}, None)?;
        let count_2_3 = collection.count_documents(doc! {"rating": {"$gte": 2, "$lt": 3}}, None)?;
        let count_3_4 = collection.count_documents(doc! {"rating": {"$gte": 3, "$lt": 4}}, None)?;
        let count_4_5 = collection.count_documents(doc! {"rating": {"$gte": 4, "$lt": 5}}, None)?;
        let count_5 = collection.count_documents(doc! {"rating": 5}, None)?;
        let unknown_count = total_count - count_1_2 - count_2_3 - count_3_4 - count_4_5 - count_5;
        let model = ProductRatingModel {
            count_1_2,
            count_2_3,
            count_3_4,
            count_4_5,
            count_5,
            unknown_count,
        };
        return Ok(model);
    }

    pub fn get_distribution_products_by_orders(
        &self,
        database: &Database,
    ) -> BoxResult<ProductOrdersModel> {
        let collection = &database.product_collection;
        let total_count = collection.count_documents(doc! {}, None)?;
        let count_0 = collection.count_documents(doc! {"orders": 0}, None)?;
        let count_1_9 =
            collection.count_documents(doc! {"orders": {"$gte": 1, "$lte": 9}}, None)?;
        let count_10_19 =
            collection.count_documents(doc! {"orders": {"$gte": 10, "$lte": 19}}, None)?;
        let count_20_49 =
            collection.count_documents(doc! {"orders": {"$gte": 20, "$lte": 49}}, None)?;
        let count_50_99 =
            collection.count_documents(doc! {"orders": {"$gte": 50, "$lte": 99}}, None)?;
        let count_100_499 =
            collection.count_documents(doc! {"orders": {"$gte": 100, "$lte": 499}}, None)?;
        let count_500_999 =
            collection.count_documents(doc! {"orders": {"$gte": 500, "$lte": 999}}, None)?;
        let count_1000_9999 =
            collection.count_documents(doc! {"orders": {"$gte": 1000, "$lte": 9999}}, None)?;
        let count_10000_n = collection.count_documents(doc! {"orders": {"$gte": 10000}}, None)?;
        let unknown_count = total_count
            - count_0
            - count_1_9
            - count_10_19
            - count_20_49
            - count_50_99
            - count_100_499
            - count_500_999
            - count_1000_9999
            - count_10000_n;
        let model = ProductOrdersModel {
            count_0,
            count_1_9,
            count_10_19,
            count_20_49,
            count_50_99,
            count_100_499,
            count_500_999,
            count_1000_9999,
            count_10000_n,
            unknown_count,
        };
        return Ok(model);
    }

    pub fn get_distribution_products_by_price(
        &self,
        database: &Database,
    ) -> BoxResult<ProductPriceModel> {
        let collection = &database.product_collection;
        let count_0_1 = collection.count_documents(doc! {"price": {"$gte": 0, "$lt": 1}}, None)?;
        let count_1_5 = collection.count_documents(doc! {"price": {"$gte": 1, "$lt": 5}}, None)?;
        let count_5_10 =
            collection.count_documents(doc! {"price": {"$gte": 5, "$lt": 10}}, None)?;
        let count_10_20 =
            collection.count_documents(doc! {"price": {"$gte": 10, "$lt": 20}}, None)?;
        let count_20_30 =
            collection.count_documents(doc! {"price": {"$gte": 20, "$lt": 30}}, None)?;
        let count_30_50 =
            collection.count_documents(doc! {"price": {"$gte": 30, "$lt": 50}}, None)?;
        let count_50_100 =
            collection.count_documents(doc! {"price": {"$gte": 50, "$lt": 100}}, None)?;
        let count_100_1000 =
            collection.count_documents(doc! {"price": {"$gte": 100, "$lt": 1000}}, None)?;
        let count_1000_n = collection.count_documents(doc! {"price": {"$gte": 1000}}, None)?;
        let model = ProductPriceModel {
            count_0_1,
            count_1_5,
            count_5_10,
            count_10_20,
            count_20_30,
            count_30_50,
            count_50_100,
            count_100_1000,
            count_1000_n,
        };
        return Ok(model);
    }

    pub fn get_distribution_products_by_shipping_fee(
        &self,
        database: &Database,
    ) -> BoxResult<ShippingFeeModel> {
        let collection = &database.product_collection;
        let total_count = collection.count_documents(doc! {}, None)?;
        let count_0 = collection.count_documents(doc! {"shipping_fee": 0}, None)?;
        let count_0_1 =
            collection.count_documents(doc! {"shipping_fee": {"$gt": 0, "$lte": 1}}, None)?;
        let count_1_5 =
            collection.count_documents(doc! {"shipping_fee": {"$gt": 1, "$lte": 5}}, None)?;
        let count_5_10 =
            collection.count_documents(doc! {"shipping_fee": {"$gt": 5, "$lte": 10}}, None)?;
        let count_10_20 =
            collection.count_documents(doc! {"shipping_fee": {"$gt": 10, "$lte": 20}}, None)?;
        let count_20_30 =
            collection.count_documents(doc! {"shipping_fee": {"$gt": 20, "$lte": 30}}, None)?;
        let count_30_50 =
            collection.count_documents(doc! {"shipping_fee": {"$gt": 30, "$lte": 50}}, None)?;
        let count_50_100 =
            collection.count_documents(doc! {"shipping_fee": {"$gt": 50, "$lte": 100}}, None)?;
        let count_100_n = collection.count_documents(doc! {"shipping_fee": {"$gt": 100}}, None)?;
        let unknown_count = total_count
            - count_0
            - count_1_5
            - count_5_10
            - count_10_20
            - count_20_30
            - count_30_50
            - count_50_100
            - count_100_n;
        let model = ShippingFeeModel {
            count_0,
            count_0_1,
            count_1_5,
            count_5_10,
            count_10_20,
            count_20_30,
            count_30_50,
            count_50_100,
            count_100_n,
            unknown_count,
        };
        return Ok(model);
    }

    pub fn get_distribution_stores_by_rating(
        &self,
        database: &Database,
    ) -> BoxResult<StoreRatingModel> {
        let collection = &database.store_collection;
        let total_count = collection.count_documents(doc! {}, None)?;
        let count_0_10 =
            collection.count_documents(doc! {"rating45_ratio": {"$gte": 0, "$lt": 10}}, None)?;
        let count_10_30 =
            collection.count_documents(doc! {"rating45_ratio": {"$gte": 10, "$lt": 30}}, None)?;
        let count_30_50 =
            collection.count_documents(doc! {"rating45_ratio": {"$gte": 30, "$lt": 50}}, None)?;
        let count_50_80 =
            collection.count_documents(doc! {"rating45_ratio": {"$gte": 50, "$lt": 80}}, None)?;
        let count_80_90 =
            collection.count_documents(doc! {"rating45_ratio": {"$gte": 80, "$lt": 90}}, None)?;
        let count_90_95 =
            collection.count_documents(doc! {"rating45_ratio": {"$gte": 90, "$lt": 95}}, None)?;
        let count_95_96 =
            collection.count_documents(doc! {"rating45_ratio": {"$gte": 95, "$lt": 96}}, None)?;
        let count_96_97 =
            collection.count_documents(doc! {"rating45_ratio": {"$gte": 96, "$lt": 97}}, None)?;
        let count_97_98 =
            collection.count_documents(doc! {"rating45_ratio": {"$gte": 97, "$lt": 98}}, None)?;
        let count_98_99 =
            collection.count_documents(doc! {"rating45_ratio": {"$gte": 98, "$lt": 99}}, None)?;
        let count_99_100 =
            collection.count_documents(doc! {"rating45_ratio": {"$gte": 99}}, None)?;
        let unknown_count = total_count
            - count_0_10
            - count_10_30
            - count_30_50
            - count_50_80
            - count_80_90
            - count_90_95
            - count_95_96
            - count_96_97
            - count_97_98
            - count_98_99
            - count_99_100;
        let model = StoreRatingModel {
            count_0_10,
            count_10_30,
            count_30_50,
            count_50_80,
            count_80_90,
            count_90_95,
            count_95_96,
            count_96_97,
            count_97_98,
            count_98_99,
            count_99_100,
            unknown_count,
        };
        return Ok(model);
    }

    pub fn get_distribution_stores_by_rating45_count(
        &self,
        database: &Database,
    ) -> BoxResult<StorePositiveRatingModel> {
        let collection = &database.store_collection;
        let count_0_10 =
            collection.count_documents(doc! {"rating45_count": {"$gte": 0, "$lt": 10}}, None)?;
        let count_10_20 =
            collection.count_documents(doc! {"rating45_count": {"$gte": 10, "$lt": 20}}, None)?;
        let count_20_30 =
            collection.count_documents(doc! {"rating45_count": {"$gte": 20, "$lt": 30}}, None)?;
        let count_30_50 =
            collection.count_documents(doc! {"rating45_count": {"$gte": 30, "$lt": 50}}, None)?;
        let count_50_100 =
            collection.count_documents(doc! {"rating45_count": {"$gte": 50, "$lt": 100}}, None)?;
        let count_100_500 =
            collection.count_documents(doc! {"rating45_count": {"$gte": 100, "$lt": 500}}, None)?;
        let count_500_1000 = collection
            .count_documents(doc! {"rating45_count": {"$gte": 500, "$lt": 1000}}, None)?;
        let count_1000_2000 = collection
            .count_documents(doc! {"rating45_count": {"$gte": 1000, "$lt": 2000}}, None)?;
        let count_2000_3000 = collection
            .count_documents(doc! {"rating45_count": {"$gte": 2000, "$lt": 3000}}, None)?;
        let count_3000_5000 = collection
            .count_documents(doc! {"rating45_count": {"$gte": 3000, "$lt": 5000}}, None)?;
        let count_5000_n =
            collection.count_documents(doc! {"rating45_count": {"$gte": 5000}}, None)?;
        let model = StorePositiveRatingModel {
            count_0_10,
            count_10_20,
            count_20_30,
            count_30_50,
            count_50_100,
            count_100_500,
            count_500_1000,
            count_1000_2000,
            count_2000_3000,
            count_3000_5000,
            count_5000_n,
        };
        return Ok(model);
    }

    pub fn get_distribution_stores_by_rating12_count(
        &self,
        database: &Database,
    ) -> BoxResult<StoreNegativeRatingModel> {
        let collection = &database.store_collection;
        let count_0_10 =
            collection.count_documents(doc! {"rating12_count": {"$gte": 0, "$lt": 10}}, None)?;
        let count_10_20 =
            collection.count_documents(doc! {"rating12_count": {"$gte": 10, "$lt": 20}}, None)?;
        let count_20_30 =
            collection.count_documents(doc! {"rating12_count": {"$gte": 20, "$lt": 30}}, None)?;
        let count_30_50 =
            collection.count_documents(doc! {"rating12_count": {"$gte": 30, "$lt": 50}}, None)?;
        let count_50_100 =
            collection.count_documents(doc! {"rating12_count": {"$gte": 50, "$lt": 100}}, None)?;
        let count_100_500 =
            collection.count_documents(doc! {"rating12_count": {"$gte": 100, "$lt": 500}}, None)?;
        let count_500_1000 = collection
            .count_documents(doc! {"rating12_count": {"$gte": 500, "$lt": 1000}}, None)?;
        let count_1000_2000 = collection
            .count_documents(doc! {"rating12_count": {"$gte": 1000, "$lt": 2000}}, None)?;
        let count_2000_3000 = collection
            .count_documents(doc! {"rating12_count": {"$gte": 2000, "$lt": 3000}}, None)?;
        let count_3000_5000 = collection
            .count_documents(doc! {"rating12_count": {"$gte": 3000, "$lt": 5000}}, None)?;
        let count_5000_n =
            collection.count_documents(doc! {"rating12_count": {"$gte": 5000}}, None)?;
        let model = StoreNegativeRatingModel {
            count_0_10,
            count_10_20,
            count_20_30,
            count_30_50,
            count_50_100,
            count_100_500,
            count_500_1000,
            count_1000_2000,
            count_2000_3000,
            count_3000_5000,
            count_5000_n,
        };
        return Ok(model);
    }

    pub fn get_distribution_stores_by_rating3_count(
        &self,
        database: &Database,
    ) -> BoxResult<StoreNeutralRatingModel> {
        let collection = &database.store_collection;
        let count_0_10 =
            collection.count_documents(doc! {"rating3_count": {"$gte": 0, "$lt": 10}}, None)?;
        let count_10_20 =
            collection.count_documents(doc! {"rating3_count": {"$gte": 10, "$lt": 20}}, None)?;
        let count_20_30 =
            collection.count_documents(doc! {"rating3_count": {"$gte": 20, "$lt": 30}}, None)?;
        let count_30_50 =
            collection.count_documents(doc! {"rating3_count": {"$gte": 30, "$lt": 50}}, None)?;
        let count_50_100 =
            collection.count_documents(doc! {"rating3_count": {"$gte": 50, "$lt": 100}}, None)?;
        let count_100_500 =
            collection.count_documents(doc! {"rating3_count": {"$gte": 100, "$lt": 500}}, None)?;
        let count_500_1000 =
            collection.count_documents(doc! {"rating3_count": {"$gte": 500, "$lt": 1000}}, None)?;
        let count_1000_2000 = collection
            .count_documents(doc! {"rating3_count": {"$gte": 1000, "$lt": 2000}}, None)?;
        let count_2000_3000 = collection
            .count_documents(doc! {"rating3_count": {"$gte": 2000, "$lt": 3000}}, None)?;
        let count_3000_5000 = collection
            .count_documents(doc! {"rating3_count": {"$gte": 3000, "$lt": 5000}}, None)?;
        let count_5000_n =
            collection.count_documents(doc! {"rating3_count": {"$gte": 5000}}, None)?;
        let model = StoreNeutralRatingModel {
            count_0_10,
            count_10_20,
            count_20_30,
            count_30_50,
            count_50_100,
            count_100_500,
            count_500_1000,
            count_1000_2000,
            count_2000_3000,
            count_3000_5000,
            count_5000_n,
        };
        return Ok(model);
    }

    pub fn get_distribution_revenue_by_rating(
        &self,
        database: &Database,
    ) -> BoxResult<RevenueByRatingModel> {
        let model = RevenueByRatingModel {
            rating_1_2: get_revenue_by_rating(1, 2, database)?,
            rating_2_3: get_revenue_by_rating(2, 3, database)?,
            rating_3_4: get_revenue_by_rating(3, 4, database)?,
            rating_4_5: get_revenue_by_rating(4, 5, database)?,
            rating_5: get_revenue_by_rating(5, 6, database)?,
            unknown: get_revenue_by_unknown_rating(database)?,
        };
        return Ok(model);
    }

    pub fn get_distribution_revenue_by_price(
        &self,
        database: &Database,
    ) -> BoxResult<RevenueByPriceModel> {
        let model = RevenueByPriceModel {
            price_0_1: get_revenue_by_price(0.0, 1.0, database)?,
            price_1_5: get_revenue_by_price(1.0, 5.0, database)?,
            price_5_10: get_revenue_by_price(5.0, 10.0, database)?,
            price_10_20: get_revenue_by_price(10.0, 20.0, database)?,
            price_20_30: get_revenue_by_price(20.0, 30.0, database)?,
            price_30_50: get_revenue_by_price(30.0, 50.0, database)?,
            price_50_100: get_revenue_by_price(50.0, 100.0, database)?,
            price_100_200: get_revenue_by_price(100.0, 200.0, database)?,
            price_200_500: get_revenue_by_price(200.0, 500.0, database)?,
            price_500_700: get_revenue_by_price(500.0, 700.0, database)?,
            price_700_1000: get_revenue_by_price(700.0, 1000.0, database)?,
            price_1000_n: get_revenue_by_price_from(1000.0, database)?,
        };
        return Ok(model);
    }

    pub fn get_distribution_orders_by_price(
        &self,
        database: &Database,
    ) -> BoxResult<OrdersByPriceModel> {
        let model = OrdersByPriceModel {
            price_0_1: get_orders_by_price(0.0, 1.0, database)?,
            price_1_5: get_orders_by_price(1.0, 5.0, database)?,
            price_5_10: get_orders_by_price(5.0, 10.0, database)?,
            price_10_20: get_orders_by_price(10.0, 20.0, database)?,
            price_20_30: get_orders_by_price(20.0, 30.0, database)?,
            price_30_50: get_orders_by_price(30.0, 50.0, database)?,
            price_50_100: get_orders_by_price(50.0, 100.0, database)?,
            price_100_200: get_orders_by_price(100.0, 200.0, database)?,
            price_200_500: get_orders_by_price(200.0, 500.0, database)?,
            price_500_700: get_orders_by_price(500.0, 700.0, database)?,
            price_700_1000: get_orders_by_price(700.0, 1000.0, database)?,
            price_1000_n: get_orders_by_price_from(1000.0, database)?,
        };
        return Ok(model);
    }
}

fn first_day_next_month() -> NaiveDateTime {
    let mut now = Utc::now();
    let this_month = NaiveDate::from_ymd(now.year(), now.month(), 1);
    let next_month = this_month.checked_add_months(Months::new(1)).unwrap();
    return NaiveDateTime::new(next_month, NaiveTime::from_hms(0, 0, 0));
}

fn previous_month(now: &NaiveDateTime) -> NaiveDateTime {
    return NaiveDateTime::new(
        now.date().checked_sub_months(Months::new(1)).unwrap(),
        NaiveTime::from_hms(0, 0, 0),
    );
}

// result is online store in time range [from, to)
fn count_online_store(
    from: NaiveDateTime,
    to: NaiveDateTime,
    database: &Database,
) -> BoxResult<u64> {
    let filter = doc! {
        "online_at": {
            "$gte": from.timestamp(),
            "$lt": to.timestamp()
        }
    };
    let count = database.store_collection.count_documents(filter, None)?;
    return Ok(count);
}

fn get_revenue_by_rating(lower: u32, upper: u32, database: &Database) -> BoxResult<f64> {
    let match_stage = doc! {
        "$match": {
            "rating": {
                "$gte": lower,
                "$lt": upper
            }
        }
    };
    let group_stage = doc! {
        "$group": {
            "_id": null,
            "output": {
                "$sum": {
                    "$multiply": ["$orders", "$price"]
                }
            }
        }
    };
    let pipeline = vec![match_stage, group_stage];
    let mut cursor = database.product_collection.aggregate(pipeline, None)?;
    if cursor.advance()? == false {
        return Ok(0.0);
    }
    let document = cursor.deserialize_current()?;
    let output = document.get("output").unwrap();
    return match output.as_f64() {
        None => return Ok(0.0),
        Some(v) => Ok(v),
    };
}

fn get_revenue_by_unknown_rating(database: &Database) -> BoxResult<f64> {
    let match_stage = doc! {
        "$match": {
            "rating": null
        }
    };
    let group_stage = doc! {
        "$group": {
            "_id": null,
            "output": {
                "$sum": {
                    "$multiply": ["$orders", "$price"]
                }
            }
        }
    };
    let pipeline = vec![match_stage, group_stage];
    let mut cursor = database.product_collection.aggregate(pipeline, None)?;
    if cursor.advance()? == false {
        return Ok(0.0);
    }
    let document = cursor.deserialize_current()?;
    let output = document.get("output").unwrap();
    return match output.as_f64() {
        None => return Ok(0.0),
        Some(v) => Ok(v),
    };
}

fn get_revenue_by_price(lower: f64, upper: f64, database: &Database) -> BoxResult<f64> {
    let match_stage = doc! {
        "$match": {
            "price": {
                "$gte": lower,
                "$lt": upper
            }
        }
    };
    let group_stage = doc! {
        "$group": {
            "_id": null,
            "output": {
                "$sum": {
                    "$multiply": ["$orders", "$price"]
                }
            }
        }
    };
    let pipeline = vec![match_stage, group_stage];
    let mut cursor = database.product_collection.aggregate(pipeline, None)?;
    if cursor.advance()? == false {
        return Ok(0.0);
    }
    let document = cursor.deserialize_current()?;
    let output = document.get("output").unwrap();
    return match output.as_f64() {
        None => return Ok(0.0),
        Some(v) => Ok(v),
    };
}

fn get_revenue_by_price_from(lower: f64, database: &Database) -> BoxResult<f64> {
    let pipeline = vec![
        doc! {
            "$match": {
                "price": {
                    "$gte": lower
                }
            }
        },
        doc! {
            "$group": {
                "_id": null,
                "output": {
                    "$sum": {
                        "$multiply": ["$orders", "$price"]
                    }
                }
            }
        },
    ];
    let mut cursor = database.product_collection.aggregate(pipeline, None)?;
    if cursor.advance()? == false {
        return Ok(0.0);
    }
    let document = cursor.deserialize_current()?;
    let output = document.get("output").unwrap();
    return match output.as_f64() {
        None => return Ok(0.0),
        Some(v) => Ok(v),
    };
}

fn get_orders_by_price(lower: f64, upper: f64, database: &Database) -> BoxResult<u64> {
    let match_stage = doc! {
        "$match": {
            "price": {
                "$gte": lower,
                "$lt": upper
            }
        }
    };
    let group_stage = doc! {
        "$group": {
            "_id": null,
            "output": {
                "$sum": "$orders"
            }
        }
    };
    let pipeline = vec![match_stage, group_stage];
    let mut cursor = database.product_collection.aggregate(pipeline, None)?;
    if cursor.advance()? == false {
        return Ok(0);
    }
    let document = cursor.deserialize_current()?;
    let output = document.get("output").unwrap();
    return match output.as_i64() {
        None => return Ok(0),
        Some(v) => Ok(v as u64),
    };
}

fn get_orders_by_price_from(lower: f64, database: &Database) -> BoxResult<u64> {
    let match_stage = doc! {
        "$match": {
            "price": {
                "$gte": lower
            }
        }
    };
    let group_stage = doc! {
        "$group": {
            "_id": null,
            "output": {
                "$sum": "$orders"
            }
        }
    };
    let pipeline = vec![match_stage, group_stage];
    let mut cursor = database.product_collection.aggregate(pipeline, None)?;
    if cursor.advance()? == false {
        return Ok(0);
    }
    let document = cursor.deserialize_current()?;
    let output = document.get("output").unwrap();
    return match output.as_i64() {
        None => return Ok(0),
        Some(v) => Ok(v as u64),
    };
}

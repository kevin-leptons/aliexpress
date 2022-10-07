use crate::analyst::model::{ProductModel, ProspectProductModel};
use crate::config::Config;
use crate::database::{cursor_to_vector, deserialize_documents, Database};
use crate::log::Log;
use crate::provider::Product;
use crate::result::BoxResult;
use mongodb::bson::{bson, doc};
use mongodb::options::FindOptions;

pub struct Miner<'a> {
    config: &'a Config,
}

impl<'a> Miner<'a> {
    pub fn new(config: &'a Config) -> Self {
        return Self { config };
    }

    pub fn get_prospect_products_by_points(
        &self,
        database: &Database,
    ) -> BoxResult<ProspectProductModel> {
        score_products(self.config, database)?;
        score_stores(database)?;
        let items = get_scored_products(database)?;
        let model = ProspectProductModel {
            items,
            description: "Prospect Products by Points".to_string(),
        };
        return Ok(model);
    }
}

fn score_products(config: &Config, database: &Database) -> BoxResult<()> {
    let cost_delta = config.analyst.miner_cost_upper - config.analyst.miner_cost_lower;
    let orders_delta = config.analyst.miner_orders_upper - config.analyst.miner_orders_lower;
    let pipeline = vec![
        doc! {
            "$project": {
                "store_identity": 1,
                "cost_points": {
                    "$cond": {
                        "if": {
                            "$or": [
                                {"$lt": ["$cost", config.analyst.miner_cost_lower]},
                                {"$gt": ["$cost", config.analyst.miner_cost_upper]}
                            ]
                        },
                        "then": 0,
                        "else": {
                            "$multiply": [
                                100,
                                {"$subtract": [config.analyst.miner_cost_upper, "$cost"]},
                                {"$divide": [1, cost_delta]}
                            ]
                        }
                    }
                },
                "orders_points": {
                    "$cond": {
                        "if": {"$eq": ["$orders", null]},
                        "then": 0,
                        "else": {
                            "$cond": {
                                "if": {
                                    "$or": [
                                        {"$lt": ["$orders", config.analyst.miner_orders_lower as f64]},
                                        {"$gt": ["$orders", config.analyst.miner_orders_upper as f64]}
                                    ]
                                },
                                "then": 0,
                                "else": {
                                    "$multiply": [
                                        100,
                                        {"$subtract": ["$orders", config.analyst.miner_orders_lower as f64]},
                                        {"$divide": [1, (orders_delta as f64)]}
                                    ]
                                },
                            }
                        }
                    }
                },
                "rating_points": {
                    "$cond": {
                        "if": {"$eq": ["$rating", null]},
                        "then": 0,
                        "else": {
                            "$cond": {
                                "if": {"$lt": ["$rating", 4.5]},
                                "then": 0,
                                "else": {
                                    "$multiply": [
                                        100,
                                        {"$subtract": ["$rating", 4.5]},
                                        {"$divide": [1, 0.5]}
                                    ]
                                }
                            }
                        }
                    }
                }
            }
        },
        doc! {
            "$addFields": {
            "cost_and_orders_points": {
                "$cond": {
                    "if": {"$gt": [{"$abs": {"$subtract": ["$cost_points", "$orders_points"]}}, 10]},
                    "then": {"$min": ["$cost_points", "$orders_points"]},
                    "else": {
                        "$divide": [
                        {"$add": ["$cost_points", "$orders_points"]},
                        2
                        ]
                    }
                }
            }
        }
        },
        doc! {
            "$addFields": {
            "points": {
                "$cond": {
                    "if": {"$gt": [{"$abs": {"$subtract": ["$cost_and_orders_points", "$rating_points"]}}, 10]},
                    "then": {"$min": ["$cost_and_orders_points", "$rating_points"]},
                    "else": {
                        "$divide": [
                        {"$add": ["$cost_and_orders_points", "$rating_points"]},
                        2
                        ]
                    }
                }
            }
        }
        },
        doc! {
            "$out": "tmp_scored_product"
        },
    ];
    database.product_collection.aggregate(pipeline, None)?;
    return Ok(());
}

fn score_stores(database: &Database) -> BoxResult<()> {
    let pipeline = vec![
        doc! {
            "$project": {
                "points": {
                    "$cond": {
                        "if": {"$lt": ["$rating45_ratio", 95]},
                        "then": 0,
                        "else": "$rating45_ratio"
                    }
                }
            }
        },
        doc! {
            "$out": "tmp_scored_store"
        },
    ];
    database.store_collection.aggregate(pipeline, None)?;
    return Ok(());
}

fn get_scored_products(database: &Database) -> BoxResult<Vec<ProductModel>> {
    let pipeline = vec![
        doc! {
            "$lookup": {
                "from": "tmp_scored_store",
                "localField": "store_identity",
                "foreignField": "_id",
                "as": "store"
            }
        },
        doc! {
            "$addFields": {
                "store_points": {
                    "$cond": {
                        "if": {"$eq": [{"$size": "$store"}, 0]},
                        "then": 30,
                        "else": {
                            "$getField": {
                                "input": {"$first": "$store"},
                                "field": "points"
                            }
                        }
                    }
                }
            }
        },
        doc! {
            "$addFields": {
                "points": {
                "$cond": {
                        "if": {"$gt": [{"$abs": {"$subtract": ["$points", "$store_points"]}}, 10]},
                        "then": {"$min": ["$points", "$store_points"]},
                        "else": {
                            "$divide": [
                                {"$add": ["$points", "$store_points"]},
                                2
                            ]
                        }
                    }
                }
            }
        },
        doc! {
            "$sort": {
                "points": -1
            }
        },
        doc! {
            "$limit": 256
        },
        doc! {
            "$lookup": {
                "from": "product",
                "localField": "_id",
                "foreignField": "_id",
                "as": "product"
            }
        },
        doc! {
            "$replaceRoot": {
                "newRoot": {
                    "$mergeObjects": [
                        {"points": "$points"},
                        {"$arrayElemAt": ["$product", 0]}
                    ]
                }
            }
        },
    ];
    let mut cursor = database
        .tmp_scored_product_collection
        .aggregate(pipeline, None)?;
    let documents = cursor_to_vector(&mut cursor)?;
    let items: Vec<ProductModel> = deserialize_documents(documents)?;
    return Ok(items);
}

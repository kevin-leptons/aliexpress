use crate::provider::{Category, Product, Store};
use crate::result::{BoxResult, UnexpectedError};
use mongodb;
use mongodb::bson::{bson, doc, Array, Bson, Document};
use mongodb::error::Error;
use mongodb::options::{
    CreateIndexOptions, FindOneAndReplaceOptions, IndexOptions, ReturnDocument,
};
use mongodb::sync::Cursor;
use mongodb::{sync as mongo, IndexModel};
use serde::{Deserialize, Serialize};

pub struct Database {
    pub product_collection: mongodb::sync::Collection<Product>,
    pub category_collection: mongodb::sync::Collection<Category>,
    pub store_collection: mongodb::sync::Collection<Store>,
    pub task_collection: mongodb::sync::Collection<Task>,
    pub tmp_store_owner_collection: mongodb::sync::Collection<StoreOwner>,
    pub tmp_scored_product_collection: mongodb::sync::Collection<Document>,
    pub mongo: mongo::Database,
}

impl Database {
    pub fn new(url: &str) -> BoxResult<Database> {
        let client = match mongodb::sync::Client::with_uri_str(url) {
            Err(e) => return Err(Box::new(e)),
            Ok(v) => v,
        };
        let database = match client.default_database() {
            None => return UnexpectedError::new_as_box_result("can not select database"),
            Some(v) => v,
        };
        let product_collection = database.collection::<Product>("product");
        let category_collection = database.collection::<Category>("category");
        let store_collection = database.collection::<Store>("store");
        let task_collection = database.collection::<Task>("task");
        let tmp_store_owner_collection = database.collection::<StoreOwner>("tmp_store_owner");
        let tmp_scored_product_collection = database.collection::<Document>("tmp_scored_product");
        match database_init_indexes(&product_collection, &store_collection, &category_collection) {
            Err(e) => return Err(e),
            Ok(_) => {}
        }
        let instance = Database {
            product_collection,
            category_collection,
            store_collection,
            task_collection,
            tmp_store_owner_collection,
            tmp_scored_product_collection,
            mongo: database,
        };
        return Ok(instance);
    }

    pub fn save_store(&self, item: Store) -> BoxResult<()> {
        let filter = doc! {"_id": item.identity as f64};
        let options = FindOneAndReplaceOptions::builder().upsert(true).build();
        match self
            .store_collection
            .find_one_and_replace(filter, item, options)
        {
            Err(e) => {
                if is_conflict_document_error(&e) {
                    return Ok(());
                }
                return Err(Box::new(e));
            }
            Ok(_) => return Ok(()),
        }
    }

    pub fn read_state() {}
    pub fn write_state() {}
}

fn database_init_indexes(
    product_collection: &mongo::Collection<Product>,
    store_collection: &mongo::Collection<Store>,
    category_collection: &mongo::Collection<Category>,
) -> BoxResult<()> {
    create_product_indexes(product_collection)?;
    create_store_indexes(store_collection)?;
    // create_category_indexes(category_collection)?;
    return Ok(());
}

fn create_product_indexes(collection: &mongo::Collection<Product>) -> BoxResult<()> {
    let top_rating_index = IndexModel::builder()
        .keys(doc! {"rating": -1, "revenue": -1, "orders": -1, "price": 1, "shipping_fee": 1})
        .build();
    ensure_index(collection, top_rating_index, None)?;
    let top_orders_index = IndexModel::builder()
        .keys(doc! {"orders": -1, "revenue": -1, "price": 1, "rating": -1, "shipping_fee": 1})
        .build();
    ensure_index(collection, top_orders_index, None)?;
    let top_revenue_index = IndexModel::builder()
        .keys(doc! {"revenue": -1, "orders": -1, "price": 1, "rating": -1, "shipping_fee": 1})
        .build();
    ensure_index(collection, top_revenue_index, None)?;
    let top_highest_price_index = IndexModel::builder()
        .keys(doc! {"price": -1, "revenue": -1, "orders": -1, "rating": -1, "shipping_fee": 1})
        .build();
    ensure_index(collection, top_highest_price_index, None)?;
    let top_lowest_price_index = IndexModel::builder()
        .keys(doc! {"price": -1, "revenue": -1, "orders": -1, "rating": -1, "shipping_fee": 1})
        .build();
    ensure_index(collection, top_lowest_price_index, None)?;
    let rating_index = IndexModel::builder().keys(doc! {"rating": 1}).build();
    ensure_index(collection, rating_index, None)?;
    let shipping_fee_index = IndexModel::builder().keys(doc! {"shipping_fee": 1}).build();
    ensure_index(collection, shipping_fee_index, None)?;
    let online_at_index = IndexModel::builder().keys(doc! {"online_at": 1}).build();
    ensure_index(collection, online_at_index, None)?;
    return Ok(());
}
fn create_store_indexes(collection: &mongo::Collection<Store>) -> BoxResult<()> {
    let rating45_ratio_index = IndexModel::builder()
        .keys(doc! {"rating45_ratio": 1})
        .build();
    ensure_index(collection, rating45_ratio_index, None)?;
    let rating45_count_index = IndexModel::builder()
        .keys(doc! {"rating45_count": 1})
        .build();
    ensure_index(collection, rating45_count_index, None)?;
    let rating3_count_index = IndexModel::builder()
        .keys(doc! {"rating3_count": 1})
        .build();
    ensure_index(collection, rating3_count_index, None)?;
    let rating12_count_index = IndexModel::builder()
        .keys(doc! {"rating12_count": 1})
        .build();
    ensure_index(collection, rating12_count_index, None)?;
    return Ok(());
}

fn create_category_indexes(collection: &mongo::Collection<Category>) -> BoxResult<()> {
    let keys = doc! {"identity": 1};
    let options = IndexOptions::builder().unique(true).build();
    let index = IndexModel::builder().keys(keys).options(options).build();
    ensure_index(collection, index, None)?;
    return Ok(());
}

fn ensure_index<T>(
    collection: &mongo::Collection<T>,
    index: IndexModel,
    options: impl Into<Option<CreateIndexOptions>>,
) -> BoxResult<()> {
    let e = match collection.create_index(index, options) {
        Ok(_) => return Ok(()),
        Err(e) => e,
    };
    if is_conflict_index_error(&e) {
        return Ok(());
    }
    return Err(Box::new(e));
}

pub fn is_conflict_index_error(e: &mongodb::error::Error) -> bool {
    let kind = e.kind.clone();
    let command_error = match *kind {
        mongodb::error::ErrorKind::Command(v) => v,
        _ => return false,
    };
    return command_error.code == 86;
}

pub fn is_conflict_document_error(e: &mongodb::error::Error) -> bool {
    let kind = e.kind.clone();
    let write_failure = match *kind {
        mongodb::error::ErrorKind::Write(v) => v,
        _ => return false,
    };
    let error = match write_failure {
        mongodb::error::WriteFailure::WriteError(v) => v,
        _ => return false,
    };
    return error.code == 11000;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    #[serde(rename = "_id")]
    pub identity: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoreOwner {
    #[serde(rename = "_id")]
    pub identity: u64,
}

pub fn cursor_to_vector<T>(cursor: &mut Cursor<T>) -> Result<Vec<T>, mongodb::error::Error>
where
    T: for<'a> Deserialize<'a>,
{
    let mut items = Vec::new();
    while cursor.advance()? {
        let item: T = cursor.deserialize_current()?;
        items.push(item);
    }
    return Ok(items);
}

pub fn deserialize_documents<T>(documents: Vec<Document>) -> Result<Vec<T>, mongodb::error::Error>
where
    T: for<'a> Deserialize<'a>,
{
    let mut items = Vec::new();
    for document in documents {
        let item: T = mongodb::bson::from_bson(Bson::Document(document))?;
        items.push(item);
    }
    return Ok(items);
}

pub fn is_bulk_write_conflict_error(e: &Error) -> bool {
    let bulk_write_error = match *e.clone().kind {
        mongodb::error::ErrorKind::BulkWrite(e) => e,
        _ => return false,
    };
    let write_errors = match bulk_write_error.write_errors {
        None => return false,
        Some(v) => v,
    };
    for write_error in write_errors {
        if write_error.code != 11000 {
            return false;
        }
    }
    return true;
}

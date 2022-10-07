use crate::database::{is_bulk_write_conflict_error, is_conflict_document_error, Database};
use crate::provider::Product;
use crate::result::BoxResult;
use mongodb::bson::{doc, to_document};
use mongodb::error::Error;
use mongodb::options::InsertManyOptions;

pub fn put(product: &Product, database: &Database) -> BoxResult<()> {
    match database.product_collection.insert_one(product, None) {
        Err(e) => {
            if is_conflict_document_error(&e) {
                return Ok(());
            }
            return Err(Box::new(e));
        }
        Ok(_) => return Ok(()),
    };
}

pub fn put_many(products: &Vec<Product>, database: &Database) -> BoxResult<()> {
    let options = InsertManyOptions::builder().ordered(false).build();
    match database.product_collection.insert_many(products, options) {
        Err(e) => {
            if is_bulk_write_conflict_error(&e) {
                return Ok(());
            }
            return Err(Box::new(e));
        }
        Ok(_) => return Ok(()),
    };
}

pub fn update_many(items: &Vec<Product>, database: &Database) -> BoxResult<()> {
    if items.len() == 0 {
        return Ok(());
    }
    let mut updates = Vec::new();
    for item in items {
        let update = doc! {
            "q": {"_id": item.identity as f64},
            "u": to_document(item)?,
            "upsert": true,
            "multi": false
        };
        updates.push(update);
    }
    let command = doc! {
        "update": database.product_collection.name(),
        "updates": updates
    };
    database.mongo.run_command(command, None)?;
    return Ok(());
}

use crate::database::{is_conflict_document_error, Database};
use crate::provider::Store;
use crate::result::BoxResult;
use mongodb::bson::{doc, to_document};
use mongodb::options::UpdateOptions;

pub fn put(item: &Store, database: &Database) -> BoxResult<()> {
    match database.store_collection.insert_one(item, None) {
        Err(e) => {
            if is_conflict_document_error(&e) {
                return Ok(());
            }
            return Err(Box::new(e));
        }
        Ok(_) => return Ok(()),
    }
}

pub fn update(item: &Store, database: &Database) -> BoxResult<()> {
    return update_many(&vec![item], database);
}

pub fn update_many(items: &Vec<&Store>, database: &Database) -> BoxResult<()> {
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
        "update": database.store_collection.name(),
        "updates": updates
    };
    database.mongo.run_command(command, None)?;
    return Ok(());
}

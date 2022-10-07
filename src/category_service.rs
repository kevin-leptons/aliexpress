use crate::database::{
    cursor_to_vector, is_bulk_write_conflict_error, is_conflict_document_error, Database,
};
use crate::provider::Category;
use crate::result::BoxResult;
use mongodb::bson::{doc, to_document, Document};
use mongodb::options::{FindOptions, InsertManyOptions};
use mongodb::sync::{Collection, Cursor};
use serde::Serialize;
use std::collections::VecDeque;

pub struct CategoryIterator<'a> {
    total_count: u64,
    finished_count: u64,
    items: VecDeque<Category>,
    next_identity: u64,
    collection: &'a Collection<Category>,
}

impl<'a> Iterator for CategoryIterator<'a> {
    type Item = Result<Category, mongodb::error::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.items.len() == 0 {
            let items = match self.load_items() {
                Err(e) => return Some(Err(e)),
                Ok(v) => v,
            };
            match items.last() {
                None => {}
                Some(v) => {
                    self.next_identity = v.identity + 1;
                }
            };
            self.items = VecDeque::from_iter(items);
        }
        let item = match self.items.pop_front() {
            None => return None,
            Some(v) => v,
        };
        return Some(Ok(item));
    }
}

impl<'a> CategoryIterator<'a> {
    pub fn new(collection: &'a Collection<Category>, from_identity: u64) -> BoxResult<Self> {
        let (total_count, finished_count) = Self::count_documents(from_identity, collection)?;
        let items = VecDeque::new();
        let next_identity = from_identity;
        let iterator = Self {
            total_count,
            finished_count,
            items,
            next_identity,
            collection,
        };
        return Ok(iterator);
    }

    fn load_items(&mut self) -> Result<Vec<Category>, mongodb::error::Error> {
        let filter = doc! {
            "_id": {
                "$gte": self.next_identity as f64
            },
            "level": {
                "$lte": 2
            }
        };
        let sort = doc! {"level": 1, "_id": 1};
        let options = FindOptions::builder().sort(sort).limit(128).build();
        let mut cursor = self.collection.find(filter, options)?;
        let items = cursor_to_vector(&mut cursor)?;
        return Ok(items);
    }

    pub fn total_count(&self) -> u64 {
        return self.total_count;
    }

    pub fn finished_count(&self) -> u64 {
        return self.finished_count;
    }

    fn count_documents(
        from_identity: u64,
        collection: &Collection<Category>,
    ) -> BoxResult<(u64, u64)> {
        let finished_filter = doc! {"_id": {"$lt": from_identity as u32}};
        let finished_count = collection.count_documents(finished_filter, None)?;
        let total_count = collection.count_documents(None, None)?;
        return Ok((total_count, finished_count));
    }
}

pub fn put(item: &Category, database: &Database) -> BoxResult<()> {
    match database.category_collection.insert_one(item, None) {
        Err(e) => {
            if is_conflict_document_error(&e) {
                return Ok(());
            }
            return Err(Box::new(e));
        }
        Ok(_) => return Ok(()),
    }
}

pub fn put_many(items: &Vec<Category>, database: &Database) -> BoxResult<()> {
    if items.len() == 0 {
        return Ok(());
    }
    let options = InsertManyOptions::builder().ordered(false).build();
    match database.category_collection.insert_many(items, options) {
        Err(e) => {
            if is_bulk_write_conflict_error(&e) {
                return Ok(());
            }
            return Err(Box::new(e));
        }
        Ok(_) => return Ok(()),
    };
}

pub fn update_many(items: &Vec<Category>, database: &Database) -> BoxResult<()> {
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
        "update": database.category_collection.name(),
        "updates": updates
    };
    database.mongo.run_command(command, None)?;
    return Ok(());
}

pub fn iterator(database: &Database, from_identity: u64) -> BoxResult<CategoryIterator> {
    let iterator = CategoryIterator::new(&database.category_collection, from_identity)?;
    return Ok(iterator);
}

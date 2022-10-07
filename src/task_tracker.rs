use crate::database::{Database, Task};
use crate::result::BoxResult;
use mongodb::bson::datetime::DateTimeBuilder;
use mongodb::bson::doc;
use mongodb::options::IndexVersion::Custom;
use mongodb::options::{FindOneAndReplaceOptions, ReplaceOptions};
use mongodb::sync::Collection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub trait GeneralTask<S>
where
    S: for<'a> Deserialize<'a> + for<'a> Serialize,
{
    fn identity() -> String;
    fn new(state: Option<S>) -> Self;
    fn state(&self) -> &S;
    fn state_mut(&mut self) -> &mut S;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CategoryState {
    pub done: bool,
    pub prepared: bool,
    pub from_identity: u64,
}

pub struct CategoryTask {
    current_state: CategoryState,
}

impl GeneralTask<CategoryState> for CategoryTask {
    fn identity() -> String {
        return "pull.category".to_string();
    }

    fn new(state: Option<CategoryState>) -> Self {
        return match state {
            None => Self {
                current_state: CategoryState {
                    done: false,
                    from_identity: 0,
                    prepared: false,
                },
            },
            Some(v) => Self { current_state: v },
        };
    }

    fn state(&self) -> &CategoryState {
        return &self.current_state;
    }

    fn state_mut(&mut self) -> &mut CategoryState {
        return &mut self.current_state;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductState {
    pub category_identity: u64,
    pub category_page_index: u32,
    pub done: bool,
}

impl ProductState {
    pub fn new() -> Self {
        return ProductState {
            category_identity: 0,
            category_page_index: 0,
            done: false,
        };
    }
}

pub struct ProductTask {
    current_state: ProductState,
}

impl GeneralTask<ProductState> for ProductTask {
    fn identity() -> String {
        return "pull.product".to_string();
    }

    fn new(state: Option<ProductState>) -> Self {
        return match state {
            None => Self {
                current_state: ProductState::new(),
            },
            Some(v) => Self { current_state: v },
        };
    }

    fn state(&self) -> &ProductState {
        return &self.current_state;
    }

    fn state_mut(&mut self) -> &mut ProductState {
        return &mut self.current_state;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoreState {
    pub store_owner_identity: u64,
    pub store_owner_prepared: bool,
    pub done: bool,
}

impl StoreState {
    fn new() -> Self {
        return Self {
            store_owner_identity: 0,
            store_owner_prepared: false,
            done: false,
        };
    }
}

pub struct StoreTask {
    current_state: StoreState,
}

impl GeneralTask<StoreState> for StoreTask {
    fn identity() -> String {
        return "pull.store".to_string();
    }

    fn new(state: Option<StoreState>) -> Self {
        return match state {
            None => Self {
                current_state: StoreState::new(),
            },
            Some(v) => Self { current_state: v },
        };
    }

    fn state(&self) -> &StoreState {
        return &self.current_state;
    }

    fn state_mut(&mut self) -> &mut StoreState {
        return &mut self.current_state;
    }
}

pub struct TaskTracker {
    steps: HashMap<String, Task>,
}

impl TaskTracker {
    pub fn read_task<T, S>(database: &Database) -> BoxResult<T>
    where
        T: GeneralTask<S>,
        S: for<'a> Deserialize<'a> + for<'a> Serialize,
    {
        let filter = doc! {
            "_id": T::identity().as_str()
        };
        let task = match database.task_collection.find_one(filter, None)? {
            None => T::new(None),
            Some(v) => Self::deserialize_task(&v)?,
        };
        return Ok(task);
    }

    pub fn write_task<T, S>(task: &T, database: &Database) -> BoxResult<()>
    where
        T: GeneralTask<S>,
        S: for<'a> Deserialize<'a> + for<'a> Serialize,
    {
        let identity = T::identity();
        let state = serde_json::to_string(task.state())?;
        let database_task = Task {
            identity: identity.clone(),
            state: state,
        };
        let query = doc! {"_id": identity};
        let options = FindOneAndReplaceOptions::builder().upsert(true).build();
        database
            .task_collection
            .find_one_and_replace(query, database_task, options)?;
        return Ok(());
    }

    fn deserialize_task<T, S>(database_task: &Task) -> BoxResult<T>
    where
        T: GeneralTask<S>,
        S: for<'a> Deserialize<'a> + for<'a> Serialize,
    {
        let state: S = serde_json::from_str(database_task.state.as_str())?;
        let task = T::new(Some(state));
        return Ok(task);
    }
}

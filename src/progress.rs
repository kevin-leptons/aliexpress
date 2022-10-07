use crate::result::{BoxResult, UnexpectedError};
use chrono::{DateTime, Duration, Utc};

pub struct Progress {
    total_tasks: u64,
    finished_tasks: u64,
    ticked_tasks: u64,
    estimated_task_time: Duration,
    started_at: DateTime<Utc>,
}

impl Progress {
    pub fn new(
        finished_tasks: u64,
        total_tasks: u64,
        estimated_task_time: Duration,
    ) -> BoxResult<Self> {
        if estimated_task_time.is_zero() {
            return UnexpectedError::new_as_box_result("estimated_task_time must be > 0");
        }
        let started_at = Utc::now();
        let instance = Self {
            total_tasks,
            estimated_task_time,
            finished_tasks,
            started_at,
            ticked_tasks: 0,
        };
        return Ok(instance);
    }

    pub fn tick(&mut self) {
        if self.finished_tasks >= self.total_tasks {
            return;
        }
        self.finished_tasks = self.finished_tasks + 1;
        self.ticked_tasks = self.ticked_tasks + 1;
    }

    pub fn percentage(&self) -> f64 {
        return (self.finished_tasks as f64) * 100.0 / (self.total_tasks as f64);
    }

    pub fn estimated(&self) -> Duration {
        let task_time = self.estimate_task_time();
        let remain_tasks = self.total_tasks - self.finished_tasks;
        let remain_seconds = task_time * (remain_tasks as i32);
        return remain_seconds;
    }

    fn estimate_task_time(&self) -> Duration {
        if self.ticked_tasks == 0 {
            return self.estimated_task_time;
        }
        let now = Utc::now();
        let elapsed = now - self.started_at;
        let task_time = elapsed / (self.ticked_tasks as i32);
        if task_time.is_zero() {
            return self.estimated_task_time;
        }
        return task_time;
    }
}

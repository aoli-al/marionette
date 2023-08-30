use rand::{RngCore, seq::SliceRandom};
use crate::gtid::Gtid;

use super::Scheduler;

pub struct RandomScheduler {
    rand: Box<dyn RngCore>,
}


impl RandomScheduler {
    pub fn new() -> Self {
        Self {
            rand: Box::new(rand::thread_rng()),
        }
    }
}

impl Scheduler for RandomScheduler {
    fn new_execution(&mut self) {
    }

    fn next_task(&mut self, runnable_tasks: &Vec<Gtid>, _current_task: Option<Gtid>) -> Option<Gtid> {
        if runnable_tasks.is_empty() {
            return None;
        }
        runnable_tasks.choose(&mut self.rand).cloned()
    }
}

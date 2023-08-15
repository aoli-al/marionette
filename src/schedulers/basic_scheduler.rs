use std::collections::HashSet;

use crate::gtid::Gtid;

use super::Scheduler;

pub struct BaseScheduler {
    run_queue: HashSet<Gtid>,
}


impl BaseScheduler {
    pub fn new() -> Self {
        Self {
            run_queue: HashSet::new(),
        }
    }
    pub fn add_task(&mut self, id: &Gtid) {
        self.run_queue.insert(*id);
    }

    pub fn get_next(&mut self) -> Option<Gtid> {
        let elem = self.run_queue.iter().next()?.clone();
        self.run_queue.remove(&elem);
        Some(elem)
    }

    pub fn remove_task(&mut self, id: &Gtid) {
        self.run_queue.retain(|&x| x != *id);
    }
}

impl Scheduler for BaseScheduler {
    fn new_execution(&mut self) {
    }

    fn next_task(
        &mut self,
        runnable_tasks: Vec<Gtid>,
        current_task: Option<Gtid>,
    ) -> Option<Gtid> {
        if !runnable_tasks.is_empty() {
            Some(runnable_tasks[0])
        } else {
            None
        }
    }
}
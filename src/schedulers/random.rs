use rand::{RngCore, seq::SliceRandom};
use crate::{gtid::Gtid, random::random_source::RandomSource};

use super::Scheduler;

pub struct RandomScheduler {
    rand: RandomSource,
}


impl RandomScheduler {
    pub fn new() -> Self {
        Self {
            // rand: RandomSource::new(),
            rand: RandomSource::from_choices(vec![0, 0, 0, 1, 2, 1, 2, 2, 1, 1, 0]),
        }
    }
}

impl Scheduler for RandomScheduler {
    fn new_execution(&mut self) {
        self.rand.index = 0;
    }

    fn next_task(&mut self, runnable_tasks: &Vec<Gtid>, _current_task: Option<Gtid>) -> (Option<Gtid>, bool) {
        if runnable_tasks.is_empty() {
            return (None, false);
        }
        let idx = self.rand.next_choice() % runnable_tasks.len();
        log::info!("Schedule from {} tasks, index: {}.", runnable_tasks.len(), idx);
        (Some(runnable_tasks[idx]), true)
    }

    fn dump_schedules(&self) {
    }

    fn revert_last_choice(&mut self) {
        log::info!("Last choice is reverted.");
        self.rand.index -= 1;
    }

}

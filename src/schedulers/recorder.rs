use std::{collections::HashSet, ops::Index};

use crate::gtid::{Gtid, NULL_GTID};

use super::Scheduler;

pub struct ScheduleRecorder<S: Scheduler> {
    run_queue: Vec<Gtid>,
    choices: Vec<usize>,
    scheduler: S,
}
impl<S: Scheduler> ScheduleRecorder<S> {
    pub fn new(scheduler: S) -> Self {
        Self {
            run_queue: Vec::new(),
            choices: Vec::new(),
            scheduler
        }
    }

}

impl<S: Scheduler> Scheduler for ScheduleRecorder<S> {
    fn new_execution(&mut self) {
        self.scheduler.new_execution();
        self.run_queue.clear();
        self.choices.clear();
    }

    fn next_task(
        &mut self,
        runnable_tasks: &Vec<Gtid>,
        current_task: Option<Gtid>,
    ) -> (Option<Gtid>, bool) {
        let (task, res) = self.scheduler.next_task(runnable_tasks, current_task);
        if let Some(selected_task) = task {
            self.run_queue.push(selected_task);
            self.choices.push(runnable_tasks.iter().position(|x| *x == selected_task).unwrap());
        } else {
            self.run_queue.push(NULL_GTID);
        }
        (task, res)
    }

    fn dump_schedules(&self) {
        log::info!("Schedule: {}", self.run_queue.iter().map(|it| it.to_string()).collect::<Vec<_>>().join(", "));
        log::info!("Choices: {}", self.choices.iter().map(|it| it.to_string()).collect::<Vec<_>>().join(", "));
    }

    fn revert_last_choice(&mut self) {
        self.run_queue.pop();
        self.choices.pop();
        self.scheduler.revert_last_choice();
    }
}

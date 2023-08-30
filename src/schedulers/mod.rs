use crate::gtid::Gtid;

pub mod basic_scheduler;
pub mod pct;
pub mod random;



pub trait Scheduler {
    fn new_execution(&mut self);

    fn next_task(
        &mut self,
        runnable_tasks: &Vec<Gtid>,
        current_task: Option<Gtid>,
    ) -> Option<Gtid>;
}

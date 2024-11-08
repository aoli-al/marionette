use std::collections::HashSet;

use crate::gtid::Gtid;

use rand::seq::SliceRandom;
use rand::RngCore;

use super::Scheduler;

pub struct PctScheduler {
    max_depth: usize,
    priorities: Vec<Gtid>,
    change_points: HashSet<usize>,
    max_steps: usize,
    steps: usize,
    rand: Box<dyn RngCore>,
}


impl PctScheduler {
    pub fn new(max_depth: usize) -> Self {
        Self {
            max_depth,
            rand: Box::new(rand::thread_rng()),
            priorities: Vec::new(),
            change_points: HashSet::new(),
            steps: 0,
            max_steps: 0
        }
    }
}

impl Scheduler for PctScheduler {
    fn new_execution(&mut self) {
        self.max_steps = std::cmp::max(self.steps, self.max_depth);
        self.steps = 0;
        self.change_points.clear();
        self.priorities.clear();

        let mut res: Vec<usize> = (0..self.max_steps).collect();
        res.shuffle(&mut self.rand);
        for i in 0..self.max_depth {
            self.change_points.insert(i);
        }
    }

    fn next_task(&mut self, runnable_tasks: &Vec<Gtid>, _current_task: Option<Gtid>) -> (Option<Gtid>, bool) {
        if runnable_tasks.is_empty() {
            return (None, false);
        }
        let priorities = &mut self.priorities;
        let new_tasks: Vec<Gtid> = runnable_tasks
            .iter()
            .filter(|&&gtid| !priorities.contains(&gtid))
            .cloned()
            .collect();
        for gtid in new_tasks {
            let loc = self.rand.next_u32() as usize % (priorities.len() + 1);
            priorities.insert(loc, gtid);
        }
        if self.change_points.contains(&self.steps) {
            if runnable_tasks.len() == 1 {
                self.move_forward_switch_point();
            } else {
                let gtid = *priorities.first().unwrap();
                priorities.remove(0);
                priorities.push(gtid);
            }
        }
        let task = *self.priorities.iter().find(|it| runnable_tasks.contains(it)).unwrap();
        self.steps += 1;
        (Some(task), true)
    }

    fn dump_schedules(&self) {
    }


    fn revert_last_choice(&mut self) {
    }

}


impl PctScheduler {
    fn move_forward_switch_point(&mut self) {
        self.change_points.remove(&self.steps);
        let mut new_step = self.steps + 1;
        while self.change_points.contains(&new_step) {
            new_step += 1;
        }
        self.change_points.insert(new_step);
    }
}
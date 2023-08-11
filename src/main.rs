use std::{rc::Rc, cell::RefCell};

use marionette::{enclave::{self, Enclave}, ghost, topology::Topology, scheduler::{self, Scheduler}};

pub fn main() {

    let topology = Topology::new();
    let mut enclave = Enclave::new(topology);

    let scheduler = Scheduler::new(&mut enclave);
    scheduler.start();

}

use std::{rc::Rc, cell::RefCell};

use marionette::{enclave::{self, Enclave}, ghost, topology::Topology, scheduler::{self, AgentManager}};

pub fn main() {

    let topology = Topology::new();
    let mut enclave = Enclave::new(topology);

    let scheduler = AgentManager::new(&mut enclave);
    scheduler.start();

}

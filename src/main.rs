use std::{cell::RefCell, rc::Rc};

use marionette::{
    enclave::{self, Enclave},
    ghost,
    scheduler::{self, AgentManager},
    topology::Topology,
};

pub fn main() {
    let topology = Topology::new();
    let mut enclave = Enclave::new(topology);

    let scheduler = AgentManager::new(&mut enclave);
    scheduler.start();
}

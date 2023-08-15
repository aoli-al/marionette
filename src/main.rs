

use marionette::{
    enclave::{Enclave},
    scheduler::{AgentManager},
    topology::Topology,
};

pub fn main() {
    let topology = Topology::new();
    let mut enclave = Enclave::new(topology);

    let scheduler = AgentManager::new(&mut enclave);
    scheduler.start();
}

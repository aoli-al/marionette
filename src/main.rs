

use std::{fs::{OpenOptions, File}, path::PathBuf, io::{BufRead, BufReader}, ptr::null};

use marionette::{
    enclave::{Enclave},
    scheduler::{AgentManager},
    topology::Topology,
};
use simple_logger::SimpleLogger;

fn ghost_is_mounted() -> bool {
    let mounts = File::open("/proc/self/mounts").unwrap();
    let reader = BufReader::new(mounts);
    for line in reader.lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let mnt_dir = parts[1];
        let mnt_type = parts[2];
        if "/sys/fs/ghost" == mnt_dir && "ghost" == mnt_type {
            return true;
        }
    }
    false
}

fn mount_ghost() {
    unsafe {
        let ghost = "ghost".as_ptr() as *const i8;
        let res = libc::mount(ghost, "/sys/fs/ghost".as_ptr() as *const i8, ghost, 0, null());
        if res != 0  {
            let error = std::io::Error::last_os_error();
            log::error!("mount failed: {}", error);
        }
    }
}

pub fn main() {
    if !ghost_is_mounted() {
        mount_ghost();
    }

    SimpleLogger::new().with_level(log::LevelFilter::Info).init().unwrap();
    let topology = Topology::new();
    let mut enclave = Enclave::new(topology);

    let scheduler = AgentManager::new(&mut enclave);
    scheduler.start();
}

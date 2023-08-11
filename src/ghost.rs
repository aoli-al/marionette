use libc::SIGINT;
use signal_hook::iterator::Signals;

use super::*;
use std::collections::HashMap;
use std::io::Write;
use std::{
    fs::{File, OpenOptions},
    os::fd::{AsFd, AsRawFd},
    path::PathBuf,
};

pub struct Ghost {
    word_stable: StatusWordTable
}

pub struct StatusWordTable {
    pub f: File,
    pub map_size: usize,
    pub header: *mut ghost_sw_region_header,
    pub table: *mut ghost_status_word,
}

impl StatusWordTable {
    pub fn new(dir_path: &PathBuf, id: i32, numa_node: i32) -> Self {
        let mut ctl = OpenOptions::new()
            .read(true)
            .write(true)
            .open(dir_path.join("ctl"))
            .expect("Failed to open ctl");

        let cmd = format!("create sw_region {} {}", id, numa_node);
        ctl.write(cmd.as_bytes())
            .expect("Unable to write sw_region command");
        let f = OpenOptions::new()
            .read(true)
            .open(dir_path.join(format!("sw_regions/sw_{}", id)))
            .expect(format!("Failed to open sw region {}", id).as_str());
        let map_size = f.metadata().unwrap().len() as usize;
        unsafe {
            let header = libc::mmap(
                std::ptr::null_mut(),
                map_size as usize,
                libc::PROT_READ,
                libc::MAP_SHARED,
                f.as_raw_fd(),
                0,
            ) as *mut ghost_sw_region_header;
            assert!((*header).capacity > 0);
            assert!((*header).id == id as u32);
            assert!((*header).numa_node == numa_node as u32);
            let table =
                (header as *const u8).offset((*header).start as isize) as *mut ghost_status_word;

            Self {
                f, map_size, header, table
            }
        }
    }
}


pub struct GhostSignals<'a> {
    handlers: HashMap<i32, &'a fn(i32) -> bool>
}

impl <'a> GhostSignals<'a> {
    pub fn new() -> Self {
        let signal = Self {
            handlers: HashMap::new()
        };

        let mut signals = Signals::new(&[SIGINT]).expect("Can not register signals");

        signal
    }
}
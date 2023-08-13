use libc::SIGINT;
use signal_hook::iterator::Signals;

use crate::external::safe_ghost_status_word;

use super::*;
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::Ordering;
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
    pub table: *mut safe_ghost_status_word,
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
                (header as *const u8).offset((*header).start as isize) as *mut safe_ghost_status_word;

            Self {
                f, map_size, header, table
            }
        }
    }

    pub unsafe fn for_each_task_status_word<F>(&self, mut f: F)
    where
        F: FnMut(*mut safe_ghost_status_word, u32, u32),
    {
        for i in 0..(*self.header).capacity {
            let sw = self.table.add(i as usize);
            if (*sw).flags.load(Ordering::SeqCst) & GHOST_SW_F_INUSE == 0 {
                continue;
            }
            if (*sw).flags.load(Ordering::SeqCst) & GHOST_SW_TASK_IS_AGENT != 0 {
                continue;
            }
            f(sw, (*self.header).id, i);
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

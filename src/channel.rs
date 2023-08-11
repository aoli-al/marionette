use std::os::fd::{AsFd, AsRawFd};

use libc::MAP_FAILED;

use super::*;
use crate::enclave::{Enclave, SafeEnclave};

pub struct Channel {
    map_size: usize,
    pub fd: i32,
    header: *mut ghost_queue_header,
    elems: u32,
}

impl Channel {
    pub fn new(elems: usize, node: usize, cpu: usize, enclave: &SafeEnclave) -> Self {
        unsafe {
            let mut data = ghost_ioc_create_queue {
                elems: elems as i32,
                node: node as i32,
                flags: 0,
                mapsize: 0,
            };
            let ctl_fd = enclave.ctl_file.lock().unwrap().as_raw_fd();
            let fd = libc::ioctl(ctl_fd, GHOST_IOC_CREATE_QUEUE_C, &mut data as *mut _);
            assert!(fd > 0);
            let map_size = data.mapsize as usize;
            let header = libc::mmap(
                std::ptr::null_mut(),
                map_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            ) as *mut ghost_queue_header;
            assert!(header != MAP_FAILED as *mut _);
            let elems = (*header).nelems;

            let mut wakeup = ghost_agent_wakeup {
                cpu: cpu as i32,
                prio: 0
            };
            let mut data = ghost_ioc_config_queue_wakeup {
                qfd: fd,
                w: &mut wakeup as *mut _,
                ninfo: 1,
                flags: 0
            };
            let wfd = libc::ioctl(ctl_fd, GHOST_IOC_CONFIG_QUEUE_WAKEUP_C, &mut data as *mut _);
            assert_eq!(wfd, 0);
            Self { map_size, fd, header, elems }
        }
    }

    pub fn set_default_queue(&self, ctl_fd: i32) {
        let mut data = ghost_ioc_set_default_queue {
            fd: self.fd
        };

        unsafe {
            let res = libc::ioctl(ctl_fd, GHOST_IOC_SET_DEFAULT_QUEUE_C, &mut data as *mut _);
            assert_eq!(res, 0);
        }
    }
}
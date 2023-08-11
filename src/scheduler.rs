use super::*;
use std::{
    collections::HashMap,
    os::fd::AsRawFd,
    sync::{Arc, Mutex}, thread,
};

use crate::{
    agent::{Agent, Notification, NotificationTrait},
    channel::{self, Channel},
    enclave::{Enclave, SafeEnclave},
    ghost::StatusWordTable,
    ghost_ioc_sw_get_info, ghost_msg_src, ghost_type_GHOST_AGENT,
    gtid::Gtid,
    GHOST_MAX_QUEUE_ELEMS,
};

pub struct StatusWord {
    sw_info: ghost_sw_info,
    sw: *mut ghost_status_word,
    owner: Gtid,
}

impl StatusWord {
    pub fn new(enclave: &SafeEnclave, word_table: &StatusWordTable) -> Self {
        let ctl_fd = enclave.ctl_file.lock().unwrap().as_raw_fd();
        unsafe {
            let mut req = ghost_ioc_sw_get_info {
                request: ghost_msg_src {
                    type_: ghost_type_GHOST_AGENT,
                    arg: libc::sched_getcpu() as u64,
                },
                response: std::mem::zeroed(),
            };
            let res = libc::ioctl(ctl_fd, GHOST_IOC_SW_GET_INFO_C, &mut req as *mut _);
            assert_eq!(res, 0);
            let sw_info = req.response;
            assert_eq!((*word_table.header).id, sw_info.id);
            let sw = word_table.table.add(sw_info.index as usize);
            let owner = gtid::current();
            Self { sw_info, sw, owner }
        }
    }
}

pub struct Task {
    gtid: Gtid,
    status_word: StatusWord,
    seqnum: u64,
}

struct TaskAllocator {
    task_map: HashMap<i64, Task>,
}

impl TaskAllocator {
    pub fn get_task(&mut self, gtid: Gtid) -> Option<&mut Task> {
        self.task_map.get_mut(&gtid.gtid_raw)
    }
}

pub struct Scheduler<'a> {
    pub tasks: HashMap<usize, Vec<Task>>,
    pub enclave: &'a Enclave,
}

unsafe impl Send for StatusWordTable {}
unsafe impl Sync for StatusWordTable {}

impl<'a> Scheduler<'a> {
    pub fn new(enclave: &'a mut Enclave) -> Self {
        let mut channels = HashMap::<usize, Channel>::new();
        let mut tasks = HashMap::<usize, Vec<Task>>::new();
        let scheduler = Self {
            tasks: HashMap::new(),
            enclave,
        };
        scheduler
    }

    pub fn start(&self) {
        let safe_e = &self.enclave.safe_e;
        let word_table = &self.enclave.unsafe_e.word_table;
        thread::scope(|s| {
            let agent_notifications: Vec<Notification> =
                vec![Default::default(); self.enclave.safe_e.cpus.len()];
            let init_flag = Arc::new(Mutex::new(false));
            for i in 0..self.enclave.safe_e.cpus.len() {
                let cpu = self.enclave.safe_e.cpus[i];
                let mut notification = agent_notifications[i].clone();
                let flag_clone = Arc::clone(&init_flag);
                s.spawn(move || {
                    let mut agent = Agent::new(cpu, safe_e, word_table);
                    notification.notify();

                    let mut has_inited = flag_clone.lock().unwrap();
                    if !*has_inited {



                        *has_inited = true;
                    }

                    agent.run();
                });
            }

            for notification in agent_notifications {
                notification.wait();
            }
        });
    }

    pub fn tasks_of(&self, cpu: usize) -> &Vec<Task> {
        self.tasks.get(&cpu).unwrap()
    }
}

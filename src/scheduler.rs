use super::*;
use std::{
    collections::HashMap,
    os::fd::AsRawFd,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Barrier, Mutex,
    },
    thread,
};

use crate::{
    agent::Agent,
    channel::Channel,
    enclave::Enclave,
    external::safe_ghost_status_word,
    ghost::StatusWordTable,
    ghost_ioc_sw_get_info, ghost_msg_src, ghost_type_GHOST_AGENT,
    gtid::Gtid,
    requester::RunRequest,
};

pub struct StatusWord {
    pub sw: *mut safe_ghost_status_word,
}

impl StatusWord {
    pub fn new(_owner: Gtid, word_table: &StatusWordTable, sw_info: ghost_sw_info) -> Self {
        unsafe {
            let sw = word_table.table.add(sw_info.index as usize);
            Self { sw }
        }
    }
    pub fn from_world_table(ctl_fd: i32, word_table: &StatusWordTable) -> Self {
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
            let owner = gtid::current();
            StatusWord::new(owner, word_table, sw_info)
        }
    }

    pub fn boosted_priority(&self) -> bool {
        let res = unsafe { (*self.sw).flags.load(Ordering::SeqCst) };
        res & GHOST_SW_BOOST_PRIO != 0
    }
}


#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TaskState {
    Runnable,
    Blocked,
}


pub struct Task {
    pub gtid: Gtid,
    pub status_word: StatusWord,
    pub seqnum: AtomicU32,
    pub cpu: i32,
    pub state: TaskState
}


pub struct AgentManager<'a> {
    pub tasks: HashMap<usize, Vec<Task>>,
    pub enclave: &'a Enclave,
}

unsafe impl Send for StatusWordTable {}
unsafe impl Sync for StatusWordTable {}
unsafe impl Send for RunRequest {}
unsafe impl Sync for RunRequest {}

impl<'a> AgentManager<'a> {
    pub fn new(enclave: &'a mut Enclave) -> Self {
        let _channels = HashMap::<usize, Channel>::new();
        let _tasks = HashMap::<usize, Vec<Task>>::new();
        let scheduler = Self {
            tasks: HashMap::new(),
            enclave,
        };
        scheduler
    }

    pub fn start(&self) {
        let safe_e = &self.enclave.safe_e;
        let unsafe_e = &self.enclave.unsafe_e;
        let word_table = &self.enclave.unsafe_e.word_table;
        let ctl_fd = self.enclave.safe_e.ctl_file.lock().unwrap().as_raw_fd();
        thread::scope(|s| {
            let cpus = &self.enclave.safe_e.cpus;
            let init_flag = Arc::new(Mutex::new(false));
            let init_barrier = Arc::new(Barrier::new(cpus.len()));
            let discovery_flag = Arc::new(Mutex::new(0));

            for i in 0..self.enclave.safe_e.cpus.len() {
                let cpu = self.enclave.safe_e.cpus[i];
                let request = unsafe_e.build_cpu_reps(cpu);
                let flag_clone = Arc::clone(&init_flag);
                let init_clone = init_barrier.clone();
                let discovery_clone = discovery_flag.clone();
                s.spawn(move || {
                    let mut agent = Agent::new(cpu, safe_e, word_table, ctl_fd, request);

                    init_clone.wait();

                    let mut has_inited = flag_clone.lock().unwrap();
                    if !*has_inited {
                        agent.channel.set_default_queue(ctl_fd);
                        *has_inited = true;
                    }

                    {
                        let mut res = discovery_clone.lock().unwrap();
                        agent.discover_tasks();
                        *res += 1;
                    }

                    init_clone.wait();

                    println!("Init done");
                    log::info!("Agent initialization done.");
                    agent.run();
                });
            }
        });
    }

    pub fn tasks_of(&self, cpu: usize) -> &Vec<Task> {
        self.tasks.get(&cpu).unwrap()
    }
}

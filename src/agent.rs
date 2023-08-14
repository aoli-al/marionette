use std::{
    collections::HashMap,
    ffi::CString,
    io::Error,
    mem::size_of,
    os::fd::AsRawFd,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Condvar, Mutex,
    },
};

use libc::{ENOENT, ESTALE, ENODEV};
use nix::sys::ptrace::Request;

use super::*;
use crate::{
    channel::Channel,
    enclave::{Enclave, SafeEnclave},
    external::{safe_ghost_status_word, safe_ghost_ring},
    ghost::StatusWordTable,
    gtid::{self, Gtid},
    message::{payload, payload_task_new_msg, Message},
    scheduler::{AgentManager, Scheduler, StatusWord, Task}, requester::{RunRequest, RunRequestOption},
};

pub type Notification = Arc<(Mutex<bool>, Condvar)>;

pub trait NotificationTrait {
    fn notify(&mut self);
    fn wait(&self);
    fn has_been_notified(&self) -> bool;
}

impl NotificationTrait for Notification {
    fn notify(&mut self) {
        *self.0.lock().unwrap() = true;
        self.1.notify_one();
    }

    fn wait(&self) {
        let (lock, cvar) = &**self;
        let mut value = lock.lock().unwrap();
        while !*value {
            value = cvar.wait(value).unwrap();
        }
    }

    fn has_been_notified(&self) -> bool {
        *self.0.lock().unwrap()
    }
}

pub struct Agent<'a> {
    safe_e: &'a SafeEnclave,
    status_word: StatusWord,
    cpu: i32,
    pub channel: Channel,
    pub finished: Notification,
    gtid: gtid::Gtid,
    pub word_table: &'a StatusWordTable,
    ctl_fd: i32,
    tasks: HashMap<Gtid, Task>,
    scheduler: Scheduler,
    run_request: RunRequest,
}

unsafe impl<'a> Sync for Agent<'a> {}
unsafe impl<'a> Send for Agent<'a> {}

impl<'a> Agent<'a> {
    pub fn new(
        cpu: i32,
        safe_e: &'a SafeEnclave,
        word_table: &'a StatusWordTable,
        ctl_fd: i32,
        run_request: RunRequest
    ) -> Self {
        let channel = Channel::new(GHOST_MAX_QUEUE_ELEMS as usize, 0, cpu, safe_e);
        unsafe {
            let s = CString::new(format!("ap_task_{}", cpu)).unwrap().as_ptr();
            let ret = libc::prctl(libc::PR_SET_NAME, s, 0);
            assert_eq!(ret, 0);
        }
        let gtid = gtid::current();
        safe_e.wait_for_agent_online_value(0);
        safe_e.sched_agent_enter_ghost(cpu, channel.fd);
        let status_word = StatusWord::from_world_table(
            (*safe_e.ctl_file.lock().unwrap()).as_raw_fd(),
            word_table,
        );
        let agent = Self {
            safe_e,
            status_word,
            cpu,
            channel,
            finished: Default::default(),
            gtid,
            word_table,
            ctl_fd,
            tasks: HashMap::new(),
            scheduler: Scheduler::new(),
            run_request
        };
        agent
    }

    pub fn run(&mut self) {
        while !self.finished.has_been_notified() || !self.tasks.is_empty() {
            let agent_barrier = unsafe { (*self.status_word.sw).barrier.load(Ordering::SeqCst) };

            while let Some(m) = self.peek() {
                println!("start dispatch");
                self.dispatch(&m);
                self.consume(m);
            }
            self.schedule(agent_barrier);
        }
    }

    fn schedule(&mut self, agent_barrier: u32) {
        let boosted_priority =self.status_word.boosted_priority();
        let next = if  !boosted_priority {
            self.scheduler.get_next()
        } else {
            None
        };
        if let Some(gtid) = next {
            println!("Scheduled! {}", gtid);
            self.run_request.open(RunRequestOption {
                target: gtid,
                target_barrier: self.tasks[&gtid].seqnum.load(Ordering::SeqCst),
                agent_barrier: agent_barrier,
                commit_flags: COMMIT_AT_TXN_COMMIT as u8,
                ..Default::default()
            });

            if !self.run_request.commit(self.ctl_fd) {
                self.scheduler.add_task(&gtid);
            }

        } else {
            let mut flags = 0;
            if boosted_priority {
                flags |= RTLA_ON_IDLE;
            }
            self.local_yield(agent_barrier, flags as i32)
        }
    }

    fn local_yield(&self, agent_barrier: u32,  run_flags: i32) {
        let mut data = ghost_ioc_run {
            gtid: 0,
            agent_barrier: agent_barrier,
            task_barrier: 0,
            run_cpu: self.cpu as i32,
            run_flags
        };
        let res = unsafe {
            libc::ioctl(self.ctl_fd, GHOST_IOC_RUN_C, &mut data as *mut _)
        };
        if res != 0 {
            let errno = Error::last_os_error().raw_os_error().unwrap();
            assert!(errno == ESTALE || errno == ENODEV);
        }

    }

    fn consume(&self, m: Message) {
        let header = self.channel.header;
        unsafe {
            let r = (header as *mut u8).add((*header).start as usize) as *mut safe_ghost_ring;
            let slot_size = std::mem::size_of::<ghost_msg>();
            let mut tail = (*r).tail.load(Ordering::Acquire);
            tail += (roundup2!(m.length(), slot_size) / slot_size) as u32;
            (*r).tail.store(tail, Ordering::SeqCst);
        }
    }

    fn dispatch(&mut self, msg: &Message) {
        println!("new message: {}", msg.get_type());
        if msg.get_type() == MSG_NOP {
            return;
        }
        if msg.is_cpu_msg() {
            return;
        }
        let gtid = msg.get_gtid();
        if msg.get_type() == MSG_TASK_NEW {
            let payload = msg.get_payload_as::<ghost_msg_payload_task_new>();
            if !self.create_task(gtid, unsafe { payload.read_unaligned().sw_info }) {
                println!("Error: task exists: {}", gtid);
                return;
            }
        }


        let mut update_seqnum = true;
        match msg.get_type() {
            MSG_TASK_NEW => {
                self.task_new(gtid, msg);
                update_seqnum = false;
            }

            MSG_TASK_PREEMPT => {
                self.task_preempted(gtid, msg);
            }
            MSG_TASK_DEAD => {
                self.task_dead(gtid, msg);
                update_seqnum = false;
            }
            MSG_TASK_WAKEUP => {
                self.task_runnable(gtid, msg);
            }
            MSG_TASK_BLOCKED => {
                self.task_blocked(gtid, msg);
            }
            MSG_TASK_YIELD => {
                self.task_yield(gtid, msg);
            }
            MSG_TASK_DEPARTED => {
                self.task_departed(gtid, msg);
                update_seqnum = false;
            }
            MSG_TASK_SWITCHTO => {
                self.task_switchto(gtid, msg);
            }
            MSG_TASK_AFFINITY_CHANGED => {
                // todo!("impl this");
            }
            MSG_TASK_PRIORITY_CHANGED => {
                // todo!("impl this");
            }
            MSG_TASK_ON_CPU => {
            }
            _ => {}
        }

        if update_seqnum {
            (*self.tasks.get_mut(&gtid).unwrap().seqnum.get_mut()) += 1;
        }
    }

    fn peek(&self) -> Option<Message> {
        let header = self.channel.header;
        unsafe {
            let r = (header as *mut u8).add((*header).start as usize) as *mut safe_ghost_ring;
            let nelems = (*header).nelems;
            let head = (*r).head.load(Ordering::Acquire);
            let tail = (*r).tail.load(Ordering::Acquire);
            let overflow = (*r).overflow.load(Ordering::SeqCst);
            if tail == head {
                return None;
            }
            assert_eq!(overflow, 0);
            let tidx = tail & (nelems - 1);
            let msg = ((*r).msgs).as_mut_ptr().add(tidx as usize);
            Some(Message::from_raw(msg))
        }
    }

    pub fn task_preempted(&mut self, gtid: Gtid, msg: &Message) {
        self.scheduler.add_task(&gtid);
        let payload = msg.get_payload_as::<ghost_msg_payload_task_preempt>();
        if unsafe { payload.read_unaligned().from_switchto } != 0 {
            let cpu = unsafe { payload.read_unaligned().cpu };
            self.ping(cpu);
        }
    }

    pub fn task_switchto(&mut self, gtid: Gtid, msg: &Message) {
        self.scheduler.remove_task(&gtid);
    }

    pub fn task_blocked(&mut self, gtid: Gtid, msg: &Message) {
        self.scheduler.remove_task(&gtid);
        let payload = msg.get_payload_as::<ghost_msg_payload_task_blocked>();
        if unsafe { payload.read_unaligned().from_switchto } != 0 {
            let cpu = unsafe { payload.read_unaligned().cpu };
            self.ping(cpu);
        }
    }

    pub fn task_yield(&mut self, gtid: Gtid, msg: &Message) {
        self.scheduler.add_task(&gtid);
        let payload = msg.get_payload_as::<ghost_msg_payload_task_yield>();
        if unsafe { payload.read_unaligned().from_switchto } != 0 {
            let cpu = unsafe { payload.read_unaligned().cpu };
            self.ping(cpu);
        }
    }

    pub fn task_departed(&mut self, gtid: Gtid, msg: &Message) {
        self.scheduler.remove_task(&gtid);
        let payload = msg.get_payload_as::<ghost_msg_payload_task_departed>();
        if unsafe { payload.read_unaligned().from_switchto } != 0 {
            let cpu = unsafe { payload.read_unaligned().cpu };
            self.ping(cpu);
        }
        self.tasks.remove(&gtid);
    }

    pub fn task_dead(&mut self, gtid: Gtid, msg: &Message) {
        self.scheduler.remove_task(&gtid);
        self.tasks.remove(&gtid);
    }

    pub fn discover_tasks(&mut self) {
        unsafe {
            let word_table_ref = self.word_table;
            let closure = |global_sw: *mut safe_ghost_status_word, region_id: u32, idx: u32| {
                self.process_each_task_status_word(global_sw, region_id, idx);
            };
            word_table_ref.for_each_task_status_word(closure);
        }
    }

    unsafe fn process_each_task_status_word(
        &mut self,
        global_sw: *mut safe_ghost_status_word,
        region_id: u32,
        idx: u32,
    ) {
        let mut has_estale = false;
        let (sw_barrier, sw_flags, sw_runtime, sw_gtid, swi, id, assoc_status) = loop {
            let sw_barrier = (*global_sw).barrier.load(Ordering::SeqCst);
            let sw_flags = (*global_sw).flags.load(Ordering::SeqCst);
            let sw_runtime = (*global_sw).flags.load(Ordering::SeqCst);
            let sw_gtid = (*global_sw).flags.load(Ordering::SeqCst);
            let mut swi = ghost_sw_info {
                id: region_id,
                index: idx,
            };

            let id = Gtid::new(sw_gtid as i64);
            let assoc_status = self.channel.associate_task(id, sw_barrier, self.ctl_fd);
            if assoc_status < 0 {
                let err = Error::last_os_error().raw_os_error().unwrap();
                match err {
                    ENOENT => {
                        if self.tasks.contains_key(&id) {
                            self.tasks.remove(&id);
                        } else {
                            println!("Error: Already had task for gtid {}", id);
                        }
                        return;
                    }
                    ESTALE => {
                        if sw_flags & GHOST_SW_F_CANFREE != 0 {
                            assert_eq!(
                                libc::ioctl(
                                    self.ctl_fd,
                                    GHOST_IOC_SW_FREE_C,
                                    &mut swi as *mut ghost_sw_info
                                ),
                                0
                            );
                            return;
                        }

                        if has_estale {
                            println!("Got repeated ESTAL: {}, {}", id, sw_flags);
                        }
                        has_estale = true;
                        continue;
                    }
                    _ => {}
                }
            }
            break (
                sw_barrier,
                sw_flags,
                sw_runtime,
                sw_gtid,
                swi,
                id,
                assoc_status,
            );
        };
        if assoc_status & (GHOST_ASSOC_SF_ALREADY | GHOST_ASSOC_SF_BRAND_NEW) as i32 != 0 {
            return;
        }

        assert_eq!(
            size_of::<payload_task_new_msg>(),
            size_of::<ghost_msg>() + size_of::<ghost_msg_payload_task_new>()
        );
        let mut synth = payload_task_new_msg {
            header: ghost_msg {
                type_: MSG_TASK_NEW as u16,
                length: size_of::<payload_task_new_msg>() as u16,
                seqnum: sw_barrier,
                payload: __IncompleteArrayField::new(),
            },
            payload: ghost_msg_payload_task_new {
                gtid: sw_gtid as u64,
                parent_gtid: 0,
                runtime: sw_runtime as u64,
                runnable: ((sw_flags & GHOST_SW_TASK_RUNNABLE) != 0) as u16,
                nice: 0,
                sw_info: swi,
            },
        };

        let msg = Message::from_raw(&mut synth as *mut payload_task_new_msg as *mut ghost_msg);
        if !self.create_task(id, swi) {
            println!("Error: task exists: {}", id);
        }
        self.task_new(id, &msg);
    }

    fn create_task(&mut self, id: Gtid, swi: ghost_sw_info) -> bool {
        if self.tasks.contains_key(&id) {
            return false;
        }
        self.tasks.insert(
            id,
            Task {
                gtid: id,
                status_word: StatusWord::new(id, self.word_table, swi),
                seqnum: AtomicU32::new(0),
                cpu: -1
            },
        );
        true
    }

    pub fn task_new(&mut self, gtid: Gtid, msg: &Message) {
        let task = self.tasks.get_mut(&gtid).unwrap();
        let synth = msg.get_payload_as::<ghost_msg_payload_task_new>();
        task.seqnum.store(msg.get_seqnum(), Ordering::SeqCst);
        if unsafe { synth.read_unaligned().runnable } != 0 {
            let res = self.channel.associate_task(
                task.gtid,
                task.seqnum.load(Ordering::SeqCst),
                self.ctl_fd,
            );
            assert!(res >= 0);
            task.cpu = self.cpu;
            self.scheduler.add_task(&task.gtid);
            self.ping(self.cpu as i32);
        }
    }

    pub fn task_runnable(&mut self, gtid: Gtid, msg: &Message) {
        let task = self.tasks.get_mut(&gtid).unwrap();
        self.scheduler.add_task(&gtid);
        println!("msg: {}, task: {}", msg.get_seqnum(), task.seqnum.load(Ordering::SeqCst));
        if task.cpu < 0 {
            let res = self.channel.associate_task(
                task.gtid,
                msg.get_seqnum(),
                self.ctl_fd,
            );
            println!("error: {}, {}", res, Error::last_os_error());
            assert!(res >= 0);
            task.cpu = self.cpu;
            self.ping(self.cpu as i32);
        }
    }


    pub fn ping(&self, cpu: i32) -> i32 {
        let mut data = ghost_ioc_run {
            gtid: gtid::AGENT_GTID.gtid_raw,
            agent_barrier: 0,
            task_barrier: 0,
            run_cpu: cpu,
            run_flags: 0,
        };
        unsafe {
            libc::ioctl(self.ctl_fd, GHOST_IOC_RUN_C, &mut data as *mut _)
        }
    }
}

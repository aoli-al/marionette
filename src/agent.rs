use std::{
    ffi::CString,
    sync::{Arc, Condvar, Mutex},
    thread::{self, Thread},
};

use super::*;
use crate::{channel::Channel, enclave::{Enclave, SafeEnclave}, gtid, scheduler::{StatusWord, Scheduler}, ghost::StatusWordTable};

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
    cpu: usize,
    channel: Channel,
    enclave_ready: Notification,
    pub finished: Notification,
    gtid: gtid::Gtid,

}

unsafe impl <'a> Sync for Agent<'a> {}
unsafe impl <'a> Send for Agent<'a> {}

impl <'a> Agent<'a> {
    pub fn new(cpu: usize, safe_e: &'a SafeEnclave, word_table: &StatusWordTable) -> Self {
        let channel = Channel::new(GHOST_MAX_QUEUE_ELEMS as usize, 0, cpu, safe_e);
        unsafe {
            let s = CString::new(format!("ap_task_{}", cpu)).unwrap().as_ptr();
            let ret = libc::prctl(libc::PR_SET_NAME, s, 0);
            assert_eq!(ret, 0);
        }
        let gtid = gtid::current();
        safe_e.wait_for_agent_online_value(0);

        safe_e.sched_agent_enter_ghost(cpu, channel.fd);
        let status_word = StatusWord::new(safe_e, word_table);
        let agent = Self {
            safe_e,
            status_word,
            cpu,
            channel,
            enclave_ready: Default::default(),
            finished: Default::default(),
            gtid
        };
        agent
    }

    pub fn run(&mut self) {
        self.enclave_ready.wait();

        // while !self.finished.has_been_notified() || !self.safe_e.tasks_of(self.cpu).is_empty() {
        // }
    }


}


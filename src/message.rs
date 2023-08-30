use std::fmt::Display;

use crate::{
    ghost_msg, ghost_msg_payload_task_new, gtid::Gtid, _MSG_CPU_FIRST, _MSG_CPU_LAST,
    _MSG_TASK_FIRST, _MSG_TASK_LAST, MSG_TASK_NEW, MSG_TASK_PREEMPT, MSG_TASK_DEAD, MSG_TASK_BLOCKED, MSG_TASK_YIELD, MSG_TASK_WAKEUP,
};

#[repr(C)]
#[repr(align(8))]
pub struct payload<T> {
    pub header: ghost_msg,
    pub payload: T,
}

pub type payload_task_new_msg = payload<ghost_msg_payload_task_new>;
pub struct Message {
    pub msg: *mut ghost_msg,
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = match self.get_type() {
            MSG_TASK_NEW => "MSG_TASK_NEW".to_string(),
            MSG_TASK_PREEMPT => "MSG_TASK_PREEMPT".to_string(),
            MSG_TASK_DEAD => "MSG_TASK_DEAD".to_string(),
            MSG_TASK_WAKEUP => "MSG_TASK_WAKEUP".to_string(),
            MSG_TASK_BLOCKED => "MSG_TASK_BLOCK".to_string(),
            MSG_TASK_YIELD => "MSG_TASK_YIELD".to_string(),
            it => format!("MSG({})", it)
        };
        write!(f, "{}", t)
    }
}

impl Message {
    pub fn from_raw(msg: *mut ghost_msg) -> Self {
        Self { msg }
    }

    pub fn get_type(&self) -> u32 {
        unsafe { (*self.msg).type_ as u32 }
    }

    pub fn length(&self) -> usize {
        unsafe { (*self.msg).length as usize }
    }

    pub fn is_cpu_msg(&self) -> bool {
        self.get_type() >= _MSG_CPU_FIRST && self.get_type() <= _MSG_CPU_LAST
    }

    pub fn is_task_msg(&self) -> bool {
        self.get_type() >= _MSG_TASK_FIRST && self.get_type() <= _MSG_TASK_LAST
    }

    pub fn get_gtid(&self) -> Gtid {
        let gtid = unsafe { ((*self.msg).payload.as_ptr() as *const i64).read_unaligned() };
        Gtid::new(gtid)
    }

    pub fn get_payload_as<T>(&self) -> *mut T {
        unsafe { (*self.msg).payload.as_mut_ptr() as *mut _ }
    }

    pub fn get_seqnum(&self) -> u32 {
        unsafe { (*self.msg).seqnum }
    }
}

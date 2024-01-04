use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU64};

use crate::{__IncompleteArrayField, ghost_msg};

extern "C" {
    pub fn numa_node_of_cpu(cpu: libc::c_int) -> libc::c_int;
}

pub const TASK_KILLABLE: i64 = 0x00000100 | 0x00000002;

#[repr(C)]
#[repr(align(32))]
#[derive(Debug)]
pub struct safe_ghost_status_word {
    pub barrier: AtomicU32,
    pub flags: AtomicU32,
    pub gtid: AtomicU64,
    pub switch_time: AtomicI64,
    pub runtime: AtomicU64,
}

type _ghost_ring_index_t = AtomicU32;
#[repr(C)]
#[derive(Debug)]
pub struct safe_ghost_ring {
    pub head: _ghost_ring_index_t,
    pub tail: _ghost_ring_index_t,
    pub overflow: _ghost_ring_index_t,
    pub msgs: __IncompleteArrayField<ghost_msg>,
}

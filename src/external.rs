use std::sync::atomic::{AtomicU32, AtomicU64, AtomicI64};

extern "C" {
    pub fn numa_node_of_cpu(cpu: libc::c_int) -> libc::c_int;
}

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
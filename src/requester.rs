use std::hint;

use libc::{system, CPU_ZERO, CPU_SET};

use crate::{ghost_txn, gtid::Gtid, ghost_txn_state, ghost_txn_state_GHOST_TXN_READY, cpu_set_t, ghost_ioc_commit_txn, GHOST_IOC_COMMIT_TXN_C, ghost_txn_state_GHOST_TXN_NO_AGENT, ghost_txn_state_GHOST_TXN_TARGET_ONCPU, ghost_txn_state_GHOST_TXN_COMPLETE};

pub struct RunRequestOption {
    pub target: Gtid,
    pub target_barrier: u32,
    pub agent_barrier: u32,
    pub commit_flags: u8,
    pub run_flags: u16,
    pub sync_group_owner: i32,
    pub allow_txn_target_on_cpu: bool,
}

static SYNC_GROUP_NOT_OWNED: i32 = -1;

impl Default for RunRequestOption {
    fn default() -> Self {
        Self {
            target: Gtid { gtid_raw: 0 },
            target_barrier: 0,
            agent_barrier: 0,
            commit_flags: 0,
            run_flags: 0,
            sync_group_owner: SYNC_GROUP_NOT_OWNED,
            allow_txn_target_on_cpu: false,
        }
    }
}

pub struct RunRequest {
    pub txn: *mut ghost_txn,
    pub allow_txn_target_on_cpu: bool,
    pub cpu: i32
}

impl RunRequest {
    pub fn open(&mut self, option: RunRequestOption) {
        if option.sync_group_owner != SYNC_GROUP_NOT_OWNED {
            // while unsafe { sync_group_owned}
            while self.sync_group_owned() {
                hint::spin_loop();
            }
        } else {
            assert!(!self.sync_group_owned());
        }

        assert!(self.is_committed());

        unsafe {
            (*self.txn).agent_barrier = option.agent_barrier;
            (*self.txn).gtid = option.target.gtid_raw;
            (*self.txn).task_barrier = option.target_barrier;
            (*self.txn).run_flags = option.run_flags;
            (*self.txn).commit_flags = option.commit_flags;
            if option.sync_group_owner != SYNC_GROUP_NOT_OWNED {
                (*self.txn).u.sync_group_owner = option.sync_group_owner;
            }
            self.allow_txn_target_on_cpu = option.allow_txn_target_on_cpu;
            if self.allow_txn_target_on_cpu {
                assert!(self.sync_group_owned());
            }
            (*self.txn).state = ghost_txn_state_GHOST_TXN_READY;
        }
    }

    pub fn sync_group_owner_get(&self) -> i32 {
        unsafe {(*self.txn).u.sync_group_owner}
    }

    pub fn sync_group_owned(&self) -> bool {
        self.sync_group_owner_get() != SYNC_GROUP_NOT_OWNED
    }

    pub fn state(&self) -> ghost_txn_state {
        unsafe { (*self.txn).state }
    }

    pub fn is_committed(&self) -> bool {
        self.state() < 0
    }

    pub fn is_open(&self) -> bool {
        self.state() == ghost_txn_state_GHOST_TXN_READY
    }

    pub fn commit(&self, ctl_fd: i32) -> bool {
        if self.is_open() {
            let res = unsafe {

                let mut cpuset: libc::cpu_set_t = std::mem::zeroed();
                CPU_ZERO(&mut cpuset);
                CPU_SET(self.cpu as usize, &mut cpuset);

                let mut data = ghost_ioc_commit_txn {
                    mask_ptr: &mut cpuset as *mut _ as *mut cpu_set_t,
                    mask_len: std::mem::size_of_val(&cpuset) as u32,
                    flags: 0
                };
                libc::ioctl(ctl_fd, GHOST_IOC_COMMIT_TXN_C, &mut data as *mut _)
            };
            assert_eq!(res, 0);
        }

        while !self.is_committed() {
            hint::spin_loop();
        }
        let state = self.state();
        state == ghost_txn_state_GHOST_TXN_COMPLETE
    }
}

use crate::ghost::StatusWordTable;
use crate::requester::RunRequest;
use crate::topology::CpuList;
use crate::topology::Topology;

use super::*;
use atoi::atoi;
use libc::epoll_ctl;
use libc::epoll_event;
use libc::sched_getaffinity;
use libc::MAP_FAILED;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Seek;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

static GOAST_FS_MOUNT: &str = "/sys/fs/ghost";
pub static GHOST_VERSION: i32 = 83;

pub struct CpuDel {
    pub requester: RunRequest,
}

pub struct Enclave {
    pub safe_e: SafeEnclave,
    pub unsafe_e: UnsafeEnclave,
}

pub struct SafeEnclave {
    pub dir_path: PathBuf,
    pub ctl_file: Mutex<File>,
    pub topology: Topology,
    pub cpus: Vec<i32>,
}

pub struct UnsafeEnclave {
    pub data_region: *mut ghost_cpu_data,
    pub word_table: StatusWordTable,
}

pub fn create_and_attach_to_enclave() -> Result<(PathBuf, File), ()> {
    let ctl_path = PathBuf::from(&GOAST_FS_MOUNT).join("ctl");
    let mut top_ctl = OpenOptions::new()
        .write(true)
        .open(ctl_path)
        .expect("Failed to open ctl file.");

    let mut e_id = 1;
    loop {
        let cmd = format!("create {} {}", e_id, GHOST_VERSION);
        match top_ctl.write(cmd.as_bytes()) {
            Ok(len) if len == cmd.len() => {
                break;
            }
            Err(ref e) if e.kind() == ErrorKind::AlreadyExists => {
                e_id += 1;
            }
            _ => {
                return Err(());
            }
        }
    }
    let ctl_path = PathBuf::from(&GOAST_FS_MOUNT).join(format!("enclave_{}/ctl", e_id));
    let mut ctl_file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(ctl_path)
        .expect("Failed to open enclave");
    ctl_file.seek(std::io::SeekFrom::Start(0)).expect("Failed to seek");
    const U64_IN_ASCII_BYTES: usize = 20 + 2 + 1 + 9;
    let mut buf = [0u8; U64_IN_ASCII_BYTES];
    ctl_file.read_exact(&mut buf).expect("Failed to read");
    let id = atoi::<u64>(&buf).expect("Failed to parse int");
    let dir_path = PathBuf::from(&GOAST_FS_MOUNT).join(format!("enclave_{}", id));

    assert!(dir_path.exists(), "Failed to find enclave dir");
    Ok((dir_path, ctl_file))
}

pub fn get_abi_version(dir_path: &PathBuf) -> i32 {
    let mut version_file = OpenOptions::new()
        .read(true)
        .open(dir_path.join("abi_version"))
        .expect("Failed to open version file");
    let mut contents = String::new();
    version_file
        .read_to_string(&mut contents)
        .expect("Failed to read version");
    contents.trim().parse().expect("Failed to parse version")
}

pub fn get_cpu_data_region(dir_path: &PathBuf) -> *mut ghost_cpu_data {
    let cpu_data = OpenOptions::new()
        .read(true)
        .write(true)
        .open(dir_path.join("cpu_data"))
        .expect("Failed to open CPU data");
    let data_region_size = cpu_data.metadata().unwrap().len();
    let fd = cpu_data.as_raw_fd();
    unsafe {
        let res = libc::mmap(
            std::ptr::null_mut(),
            data_region_size as usize,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            0,
        ) as *mut ghost_cpu_data;
        assert_ne!(res, MAP_FAILED as *mut _);
        res
    }
}

pub fn set_cpu_mask(dir_path: &PathBuf, enclave_cpus: &CpuList) {
    let mut cpu_mask_file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(dir_path.join("cpumask"))
        .expect("Failed to open CPU mask");
    let cpu_mask = enclave_cpus.cpu_mask_string();
    for _ in 0..10 {
        match cpu_mask_file.write(cpu_mask.as_bytes()) {
            Ok(len) if len == cpu_mask.len() => break,
            _ => {
                log::error!("Failed to write cpu mask");
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

impl Enclave {
    pub fn new(topology: Topology) -> Self {
        let (dir_path, ctl_file) = enclave::create_and_attach_to_enclave().expect("msg");
        // let abi_version = enclave::get_abi_version(&dir_path);
        let data_region = enclave::get_cpu_data_region(&dir_path);
        let word_table = ghost::StatusWordTable::new(&dir_path, 0, 0);
        let cpus = vec![0i32];
        let enclave_cpus = CpuList::from_vec(topology.num_cpus as i32, &cpus);
        set_cpu_mask(&dir_path, &enclave_cpus);

        Self {
            safe_e: SafeEnclave {
                dir_path,
                ctl_file: Mutex::new(ctl_file),
                topology,
                cpus,
            },
            unsafe_e: UnsafeEnclave {
                data_region,
                word_table,
            },
        }
    }
}

impl UnsafeEnclave {
    pub fn build_cpu_reps(&self, cpu: i32) -> RunRequest {
        unsafe {
            RunRequest {
                txn: &mut ((*self.data_region.add(cpu as usize)).txn) as *mut _,
                allow_txn_target_on_cpu: false,
                cpu,
            }
        }
    }
}
impl SafeEnclave {
    pub fn sched_agent_enter_ghost(&self, cpu: i32, queue_fd: i32) {
        let cmd = format!("become agent {} {}", cpu, queue_fd);
        loop {
            match self.ctl_file.lock().unwrap().write(cmd.as_bytes()) {
                Ok(len) if len == cmd.len() => {
                    break;
                }
                Err(e) => {
                    log::error!("Failed to assign agent to channel {}, {}: {}", cpu, queue_fd, e);
                    assert_eq!(Error::last_os_error().raw_os_error().unwrap(), libc::EBUSY);
                }
                _ => {}
            }
        }
        unsafe {
            assert_eq!(
                sched_getscheduler(0),
                SCHED_GHOST as i32 | libc::SCHED_RESET_ON_FORK
            );
        }

        let gtid = gtid::current();
        let allowed_cpus = unsafe {
            let mut allowed_cpus: libc::cpu_set_t = std::mem::zeroed();
            assert_eq!(
                sched_getaffinity(
                    gtid.gtid_raw as i32,
                    std::mem::size_of_val(&allowed_cpus),
                    &mut allowed_cpus as *mut _
                ),
                0
            );
            assert!(libc::CPU_COUNT(&allowed_cpus) <= 512);
            assert!(libc::CPU_COUNT(&allowed_cpus) > 0);
            allowed_cpus
        };

        let agent_affinity = CpuList::from_cpuset(self.topology.num_cpus as i32, &allowed_cpus);

        assert_eq!(agent_affinity.size(), 1);
        assert!(agent_affinity.is_set(cpu));
        assert_eq!(unsafe { libc::sched_getcpu() }, cpu as i32);
    }

    pub fn wait_for_agent_online_value(&self, until: i32) {
        let mut fd = OpenOptions::new()
            .read(true)
            .open(self.dir_path.join("agent_online"))
            .expect("Failed to open agent_online");
        let epfd = unsafe { libc::epoll_create(1) };
        let mut ep_ev = epoll_event {
            events: (libc::EPOLLIN | libc::EPOLLET | libc::EPOLLPRI) as u32,
            u64: 0,
        };
        let res = unsafe {
            epoll_ctl(
                epfd,
                libc::EPOLL_CTL_ADD,
                fd.as_raw_fd(),
                &mut ep_ev as *mut _,
            )
        };
        assert_eq!(res, 0);

        while unsafe {
            libc::epoll_wait(
                epfd,
                &mut ep_ev as *mut _,
                std::mem::size_of_val(&ep_ev) as i32,
                -1,
            )
        } != 1
        {
            let err = Error::last_os_error();
            assert_eq!(err.raw_os_error().unwrap(), libc::EINTR);
        }

        let mut buf = [0u8; 20];
        let len = fd.read(&mut buf).expect("Failed to read from agent_online");
        assert!(len > 0);
        let one_or_zero = atoi::<i32>(&buf).expect("Failed to parse int");
        if one_or_zero != until {
            while unsafe {
                libc::epoll_wait(
                    epfd,
                    &mut ep_ev as *mut _,
                    std::mem::size_of_val(&ep_ev) as i32,
                    -1,
                )
            } != -1
            {
                let err = Error::last_os_error();
                assert_eq!(err.raw_os_error().unwrap(), libc::EINTR);
            }
        }
    }
}

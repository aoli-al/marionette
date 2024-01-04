use crate::external;

use nix::sched::CpuSet;
use regex::Regex;


use std::collections::HashMap;

use std::io::{BufRead, BufReader};
use std::{fs::OpenOptions, path::PathBuf};

const MAX_CPUS: usize = 512;
const INTS_BITS: i32 = (8 * std::mem::size_of::<u64>()) as i32;
const MAP_CAPACITY: usize = MAX_CPUS / INTS_BITS as usize;

#[derive(Clone, PartialEq, Debug)]
pub struct CpuList {
    num_cpus: i32,
    map_size: i32,
    bitmap: [u64; MAP_CAPACITY],
}

pub struct CpuListIter<'a> {
    map: &'a CpuList,
    id: i32,
}

impl<'a> CpuListIter<'a> {
    pub fn new(map: &'a CpuList, id: i32) -> Self {
        let mut iter = Self { map, id };
        iter.find_next_set_bit();
        iter
    }

    pub fn find_next_set_bit(&mut self) {
        let mut map_idx = self.id / INTS_BITS;
        let bit_offset = self.id & (INTS_BITS - 1);

        if map_idx >= self.map.map_size {
            self.id = self.map.num_cpus;
            return;
        }
        let mut word = self.map.bitmap[map_idx as usize];
        word &= !0u64 << bit_offset;

        while map_idx < self.map.map_size {
            if word != 0 {
                self.id = map_idx * INTS_BITS + word.trailing_zeros() as i32;
                return;
            }
            map_idx += 1;
            if map_idx < self.map.map_size {
                word = self.map.bitmap[map_idx as usize];
            }
        }
        self.id = self.map.num_cpus;
    }
}

impl<'a> Iterator for CpuListIter<'a> {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.id == self.map.num_cpus {
            return None;
        }
        let current = self.id;
        self.id += 1;
        self.find_next_set_bit();
        Some(current)
    }
}

impl CpuList {
    pub fn new(num_cpus: i32) -> Self {
        let map_size = (num_cpus + INTS_BITS - 1) / INTS_BITS;
        Self {
            num_cpus,
            map_size,
            bitmap: [0; MAP_CAPACITY],
        }
    }

    pub fn from_vec(num_cpus: i32, cpus: &Vec<i32>) -> Self {
        let mut cpu_list = CpuList::new(num_cpus);
        for cpu in cpus {
            cpu_list.set(*cpu);
        }
        cpu_list
    }

    pub fn from_cpuset(num_cpus: i32, cpus: &libc::cpu_set_t) -> Self {
        let mut cpu_list = CpuList::new(num_cpus);
        for cpu in 0..libc::CPU_SETSIZE {
            if unsafe { libc::CPU_ISSET(cpu as usize, cpus) } {
                cpu_list.set(cpu);
            }
        }
        cpu_list
    }

    pub fn size(&self) -> usize {
        let mut size = 0usize;
        for byte in self.bitmap {
            size += byte.count_ones() as usize;
        }
        size
    }

    pub fn set(&mut self, id: i32) {
        self.bitmap[(id / INTS_BITS) as usize] |= 1u64 << (id % INTS_BITS);
    }
    pub fn is_set(&self, id: i32) -> bool {
        (self.bitmap[(id / INTS_BITS) as usize] & (1u64 << (id % INTS_BITS))) != 0
    }

    pub fn begin(&self) -> CpuListIter {
        CpuListIter::new(self, 0)
    }

    pub fn end(&self) -> CpuListIter {
        CpuListIter::new(self, self.map_size)
    }

    pub fn to_cpu_set(&self) -> CpuSet {
        let mut set = CpuSet::new();
        for cpu in self {
            set.set(cpu as usize).expect("Failed to create CPU set");
        }
        set
    }

    pub fn cpu_mask_string(&self) -> String {
        let mut s = String::new();
        let mut emitted_nibble = false;
        let u64_size = std::mem::size_of::<u64>() as i32;
        let byte_size = 8;
        for i in 0..self.map_size * u64_size {
            let idx = i / u64_size;
            let value = self.bitmap[idx as usize];
            let offset = byte_size - (i % u64_size) - 1;
            let byte = ((value >> (byte_size * offset)) & 0xff) as u8;
            let hi = byte >> 4;
            let lo = byte & 0xf;

            if hi != 0 || emitted_nibble {
                s.push_str(&format!("{:x}", hi));
                emitted_nibble = true;
            }

            if lo != 0 || emitted_nibble {
                s.push_str(&format!("{:x}", lo));
                emitted_nibble = true;
            }
        }

        let mut ret = String::new();
        let mut nibble_til_next_comma = s.len() % 8;
        if nibble_til_next_comma == 0 {
            nibble_til_next_comma = 8;
        }
        for c in s.chars() {
            if nibble_til_next_comma == 0 {
                nibble_til_next_comma = 8;
                ret.push(',');
            }
            nibble_til_next_comma -= 1;
            ret.push(c);
        }
        if ret.is_empty() {
            "0".to_owned()
        } else {
            ret
        }
    }
}

impl<'a> IntoIterator for &'a CpuList {
    type Item = i32;

    type IntoIter = CpuListIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        return self.begin();
    }
}

pub struct Cpu {}

pub struct CpuRep {
    siblings: CpuList,
    l3_siblings: CpuList,
    numa_node: usize,
}

pub struct Topology {
    pub num_cpus: usize,
    pub all_cpus: CpuList,
    pub cpus: Vec<CpuRep>,
    pub highest_node_idx: usize,
    pub cpus_on_node: Vec<CpuList>,
}

fn get_all_siblings(prefix: &str, suffix: &str, num_cpus: i32) -> HashMap<i32, CpuList> {
    let mut sibling_map: HashMap<String, Vec<i32>> = HashMap::new();
    for i in 0..num_cpus {
        let path = PathBuf::from(prefix).join(format!("cpu{}", i)).join(suffix);
        let f = OpenOptions::new()
            .read(true)
            .open(path)
            .unwrap_or_else(|_| panic!("Failed to open CPU {}", i));
        let mut buf_reader = BufReader::new(f);
        let mut sibling_list = String::new();
        buf_reader
            .read_line(&mut sibling_list)
            .expect("Failed to read sibling list");
        if sibling_list.is_empty() {
            assert_eq!(i, 0);
            break;
        }
        sibling_map
            .entry(sibling_list)
            .or_insert(Vec::new())
            .push(i);
    }

    let mut siblings = HashMap::<i32, CpuList>::new();
    for (_, cpus) in sibling_map {
        for cpu in &cpus {
            let cpu_list = CpuList::from_vec(num_cpus, &cpus);
            siblings.insert(*cpu, cpu_list);
        }
    }
    siblings
}

fn get_highest_node_idx(path: PathBuf) -> usize {
    let f = OpenOptions::new()
        .read(true)
        .open(path)
        .expect("Unable to open path");
    let mut buf_reader = BufReader::new(f);
    let mut node_possible_str = String::new();
    buf_reader
        .read_line(&mut node_possible_str)
        .expect("Failed to read sibling list");
    let seperator = Regex::new(r"[,-]").expect("Invalid regex");
    let mut highest_idx = 0;
    for node_idx in seperator.split(&node_possible_str) {
        highest_idx = std::cmp::max(
            highest_idx,
            node_idx.trim().parse().expect("Failed to parse node_idx"),
        );
    }
    highest_idx
}

impl Topology {
    pub fn new() -> Self {
        let num_cpus = std::thread::available_parallelism().unwrap().get() as i32;
        let siblings = get_all_siblings(
            "/sys/devices/system/cpu",
            "topology/thread_siblings",
            num_cpus,
        );
        let l3_siblings = get_all_siblings(
            "/sys/devices/system/cpu",
            "cache/index3/shared_cpu_list",
            num_cpus,
        );

        let cpus: Vec<CpuRep> = (0..num_cpus)
            .map(|i| {
                let cpu = i;
                let sls = &siblings[&i];
                let l3_sls = &l3_siblings[&i];
                let numa_node = unsafe { external::numa_node_of_cpu(cpu as libc::c_int) as usize };
                CpuRep {
                    siblings: sls.clone(),
                    l3_siblings: l3_sls.clone(),
                    numa_node,
                }
            })
            .collect();
        let all_cpus: CpuList = CpuList::from_vec(num_cpus, &(0..num_cpus).collect());
        let highest_node_idx =
            get_highest_node_idx(PathBuf::from("/sys/devices/system/node/possible"));

        // Assertions
        for c in all_cpus.begin() {
            let cpu = c as usize;
            assert!(cpus[cpu].siblings.is_set(c));
            for sibling in &cpus[cpu].siblings {
                assert_eq!(&cpus[sibling as usize].siblings, &cpus[cpu].siblings);
            }
            for l3_sibling in &cpus[cpu].l3_siblings {
                assert_eq!(
                    &cpus[l3_sibling as usize].l3_siblings,
                    &cpus[cpu].l3_siblings
                );
            }
        }

        let mut cpus_on_node = vec![CpuList::new(num_cpus); highest_node_idx + 1];
        for cpu in &all_cpus {
            let node = cpus[cpu as usize].numa_node;
            cpus_on_node[node].set(cpu);
        }
        Self {
            num_cpus: num_cpus as usize,
            all_cpus,
            cpus,
            highest_node_idx,
            cpus_on_node,
        }
    }
}

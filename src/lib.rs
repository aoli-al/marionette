#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub mod external;
pub mod agent;
pub mod enclave;
pub mod ghost;
pub mod global;
pub mod topology;
pub mod requester;
pub mod gtid;
pub mod scheduler;
pub mod channel;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

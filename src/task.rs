use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{ghost_sw_info, gtid::Gtid};

pub struct Task {
    gtid: Gtid,
    sw_info: ghost_sw_info,
}

pub struct Allocator<'a> {
    tasks: Mutex<HashMap<Gtid, Task>>,
    _phantom: std::marker::PhantomData<&'a str>,
}

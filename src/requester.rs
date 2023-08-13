use crate::ghost_txn;

pub struct RunRequest {
    pub txn: *const ghost_txn,
}

impl RunRequest {
}
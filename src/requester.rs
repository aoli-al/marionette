use crate::ghost_txn;

pub struct Requester {
    pub txn: *const ghost_txn,
}
use std::{
    fmt,
    fs::OpenOptions,
    hash::Hash,
    hash::Hasher,
    io::{self, Read},
    path::PathBuf,
};

#[derive(Clone, Copy)]
pub struct Gtid {
    pub gtid_raw: i64,
}

pub static NULL_GTID: Gtid = Gtid::new(0);
pub static AGENT_GTID: Gtid = Gtid::new(-1);
pub static IDLE_GTID: Gtid = Gtid::new(-2);

impl fmt::Display for Gtid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.gtid_raw)
    }
}

impl Eq for Gtid {}

impl PartialEq for Gtid {
    fn eq(&self, other: &Self) -> bool {
        self.gtid_raw == other.gtid_raw
    }
}

impl Hash for Gtid {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.gtid_raw.hash(state);
    }
}

impl Gtid {
    pub const fn new(gtid_raw: i64) -> Self {
        Self { gtid_raw }
    }
}

pub struct GtidStore {}

pub fn current() -> Gtid {
    let gtid_raw = nix::unistd::gettid().as_raw() as i64;
    Gtid { gtid_raw }
}

pub fn gtid(pid: i64) -> io::Result<i64> {
    let proc = PathBuf::from("/proc")
        .join(pid.to_string())
        .join("ghost/gtid");
    let mut stream = OpenOptions::new().read(true).open(proc)?;
    let mut content = String::new();
    stream.read_to_string(&mut content)?;
    Ok(content.trim().parse().expect("Failed to parse"))
}

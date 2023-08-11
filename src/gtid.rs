use std::{fs::OpenOptions, path::PathBuf, thread, io::{BufReader, BufRead, Read, self}};

pub struct Gtid {
    pub gtid_raw: i64,
}

pub struct GtidStore {
}


pub fn current() -> Gtid {
    let gtid_raw= nix::unistd::gettid().as_raw() as i64;
    Gtid {
        gtid_raw
    }
}

pub fn gtid(pid: i64) -> io::Result<i64> {
    let proc = PathBuf::from("/proc")
        .join(pid.to_string())
        .join("ghost/gtid");
    let mut stream = OpenOptions::new()
        .read(true)
        .open(proc)?;
    let mut content = String::new();
    stream.read_to_string(&mut content)?;
    Ok(content.trim().parse().expect("Failed to parse"))
}

extern "C" {
    pub fn numa_node_of_cpu(cpu: libc::c_int) -> libc::c_int;
}

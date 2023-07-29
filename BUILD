# Note: If you modify this BUILD file, please contact jhumphri@ first to ensure
# that you are not breaking the Copybara script.

load("//:bpf/bpf.bzl", "bpf_skeleton")

compiler_flags = [
    "-Wno-sign-compare",
]

bpf_linkopts = [
    "-lelf",
    "-lz",
]

cc_library(
    name = "agent",
    srcs = [
        "bpf/user/agent.c",
        "lib/agent.cc",
        "lib/channel.cc",
        "lib/enclave.cc",
    ],
    hdrs = [
        "bpf/user/agent.h",
        "bpf/user/schedghostidle_bpf.skel.h",
        "lib/agent.h",
        "lib/channel.h",
        "lib/enclave.h",
        "lib/scheduler.h",
        "//third_party:iovisor_bcc/trace_helpers.h",
    ],
    copts = compiler_flags,
    linkopts = bpf_linkopts + ["-lnuma"],
    deps = [
        ":base",
        ":ghost",
        ":shared",
        ":topology",
        ":trivial_status",
        "@com_google_absl//absl/base:core_headers",
        "@com_google_absl//absl/container:flat_hash_map",
        "@com_google_absl//absl/container:flat_hash_set",
        "@com_google_absl//absl/flags:flag",
        "@com_google_absl//absl/status",
        "@com_google_absl//absl/status:statusor",
        "@com_google_absl//absl/strings",
        "@com_google_absl//absl/strings:str_format",
        "@com_google_absl//absl/synchronization",
        "@linux//:libbpf",
    ],
    visibility = ["//visibility:public"],

)

cc_library(
    name = "trivial_status",
    srcs = ["lib/trivial_status.cc"],
    hdrs = ["lib/trivial_status.h"],
    copts = compiler_flags,
    deps = [
        "@com_google_absl//absl/log:check",
        "@com_google_absl//absl/status",
        "@com_google_absl//absl/status:statusor",
        "@com_google_absl//absl/strings:str_format",
    ],
)



exports_files(glob([
    "kernel/vmlinux_ghost_*.h",
]) + [
    "lib/queue.bpf.h",
])

cc_library(
    name = "topology",
    srcs = [
        "lib/topology.cc",
    ],
    hdrs = [
        "lib/topology.h",
    ],
    copts = compiler_flags,
    linkopts = ["-lnuma"],
    deps = [
        ":base",
        "@com_google_absl//absl/container:flat_hash_map",
        "@com_google_absl//absl/container:flat_hash_set",
        "@com_google_absl//absl/strings:str_format",
    ],
)

cc_library(
    name = "base",
    srcs = [
        "lib/base.cc",
    ],
    hdrs = [
        "kernel/ghost_uapi.h",
        "lib/base.h",
        "lib/logging.h",
        "//third_party:util/util.h",
    ],
    copts = compiler_flags,
    linkopts = ["-lcap"],
    deps = [
        "@com_google_absl//absl/base",
        "@com_google_absl//absl/base:core_headers",
        "@com_google_absl//absl/container:flat_hash_map",
        "@com_google_absl//absl/container:node_hash_map",
        "@com_google_absl//absl/debugging:stacktrace",
        "@com_google_absl//absl/debugging:symbolize",
        "@com_google_absl//absl/flags:flag",
        "@com_google_absl//absl/log",
        "@com_google_absl//absl/log:check",
        "@com_google_absl//absl/memory",
        "@com_google_absl//absl/status",
        "@com_google_absl//absl/status:statusor",
        "@com_google_absl//absl/strings:str_format",
        "@com_google_absl//absl/time",
    ],
)



cc_binary(
    name = "fdcat",
    srcs = [
        "util/fdcat.cc",
    ],
    copts = compiler_flags,
    deps = [
        ":shared",
    ],
)

cc_binary(
    name = "fdsrv",
    srcs = [
        "util/fdsrv.cc",
    ],
    copts = compiler_flags,
    deps = [
        ":shared",
    ],
)


cc_binary(
    name = "test_scheduler",
    srcs = [
        "schedulers/fifo/per_cpu/fifo_agent.cc",
    ],
    copts = compiler_flags,
    deps = [
        ":agent",
        ":fifo_per_cpu_scheduler",
        "@com_google_absl//absl/debugging:symbolize",
        "@com_google_absl//absl/flags:parse",
    ],
)

cc_library(
    name = "fifo_per_cpu_scheduler",
    srcs = [
        "schedulers/fifo/per_cpu/fifo_scheduler.cc",
        "schedulers/fifo/per_cpu/fifo_scheduler.h",
    ],
    hdrs = [
        "schedulers/fifo/per_cpu/fifo_scheduler.h",
    ],
    copts = compiler_flags,
    deps = [
        ":agent",
    ],
)


cc_library(
    name = "ghost",
    srcs = [
        "lib/ghost.cc",
    ],
    hdrs = [
        "kernel/ghost_uapi.h",
        "lib/ghost.h",
    ],
    copts = compiler_flags,
    linkopts = ["-lnuma"],
    deps = [
        ":base",
        ":topology",
        "@com_google_absl//absl/container:flat_hash_map",
        "@com_google_absl//absl/container:flat_hash_set",
        "@com_google_absl//absl/flags:flag",
        "@com_google_absl//absl/log",
        "@com_google_absl//absl/strings",
        "@com_google_absl//absl/strings:str_format",
    ],
)

cc_library(
    name = "shared",
    srcs = [
        "shared/fd_server.cc",
        "shared/prio_table.cc",
        "shared/shmem.cc",
    ],
    hdrs = [
        "shared/fd_server.h",
        "shared/prio_table.h",
        "shared/shmem.h",
    ],
    copts = compiler_flags,
    deps = [
        ":base",
        "@com_google_absl//absl/cleanup",
        "@com_google_absl//absl/status",
        "@com_google_absl//absl/status:statusor",
        "@com_google_absl//absl/strings",
        "@com_google_absl//absl/synchronization",
    ],
)

cc_binary(
    name = "enclave_watcher",
    srcs = [
        "util/enclave_watcher.cc",
    ],
    copts = compiler_flags,
    deps = [
        ":agent",
        ":ghost",
        "@com_google_absl//absl/flags:parse",
    ],
)

cc_binary(
    name = "pushtosched",
    srcs = [
        "util/pushtosched.cc",
    ],
    copts = compiler_flags,
    deps = [
        "@com_google_absl//absl/strings",
        "@com_google_absl//absl/strings:str_format",
    ],
)

bpf_skeleton(
    name = "schedghostidle_bpf_skel",
    bpf_object = "//third_party/bpf:schedghostidle_bpf",
    skel_hdr = "bpf/user/schedghostidle_bpf.skel.h",
)

cc_binary(
    name = "schedghostidle",
    srcs = [
        "bpf/user/schedghostidle.c",
        "bpf/user/schedghostidle_bpf.skel.h",
        "//third_party:iovisor_bcc/trace_helpers.h",
    ],
    copts = compiler_flags,
    linkopts = bpf_linkopts,
    deps = [
        "@linux//:libbpf",
    ],
)

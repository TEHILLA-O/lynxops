# eBPF

eBPF is the **advanced layer**. 0.1 ships types and a capability probe so the rest of LynxOps can ship without a BPF toolchain.

## Why it is last

Process, cgroup, net, and journal collectors already cover most incident questions:

- What is using CPU and RSS?
- Which unit owns that cgroup?
- Who is listening, and is the unit failed?
- What did the journal say this boot?

eBPF answers questions *procfs cannot*: short-lived execs, connect storms, openat paths that never show up in `fd/`. Those probes need BTF, privileges, and a build story (Aya + `bpf-linker`). They should not gate a usable CLI.

## What 0.1 contains

| Crate | Role |
| --- | --- |
| `lynx-ebpf-common` | `#![no_std]` `repr(C)` `ExecEvent` / `ConnectEvent` |
| `lynx-ebpf` | `probe_capability()`, placeholder `attach_default_probes()` |
| CLI | `lynxops doctor` prints BTF / bpf fs / build feature |

`lynxops doctor` checks:

- `/sys/kernel/btf/vmlinux` (CO-RE)
- `/sys/fs/bpf`
- whether the binary was compiled with `--features probes`
- `target_os = linux`

## Planned probes (not loaded today)

1. **exec** — `sched_process_exec` / `sys_enter_execve`: pid, uid, comm, filename.
2. **connect / bind** — `tcp_v4_connect`, `inet_bind`: 5-tuple + comm.
3. **open** (later) — noisy; gated by pid or cgroup filter.

Events are fixed-size so a ring buffer can move them without serialization in the kernel. Userspace maps them into the TUI “events” view and into snapshots as a time-ordered appendix.

## Privileges

Loading probes needs `CAP_BPF` and `CAP_PERFMON` (or `CAP_SYS_ADMIN` on older kernels), plus an unprivileged-bpf policy that allows it. LynxOps will refuse to load when `probe_capability()` is not `Available`. That is a product decision: this tool observes the host you already administer; it does not try to bypass LSM or hide from audit.

## Build (future)

```text
cargo build -p lynx-ebpf --features probes
```

The `probes` feature is reserved. When it lands it will compile Aya programs in-tree and load them from `lynx-ebpf`. Until then `attach_default_probes()` returns `LynxError::Unsupported`.

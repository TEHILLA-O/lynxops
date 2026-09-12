# Architecture

LynxOps is a Rust workspace of small collectors behind one CLI/TUI. The binary does not talk to the kernel directly; crates do, and they all speak `lynx-core` types.

```text
                 ┌──────────── lynx-cli ────────────┐
                 │  clap · tables · --json · doctor │
                 └──────┬─────────────┬─────────────┘
                        │             │
                   lynx-tui      lynx-snapshot
                        │             │
        ┌───────────────┼─────────────┼───────────────┐
        │               │             │               │
   lynx-proc      lynx-cgroup   lynx-network    lynx-systemd
        │               │             │               │
        └───────────────┴──────┬──────┴───────────────┘
                               │
                          lynx-core
                     (Pid, Socket, Unit, …)

   lynx-ebpf ──► lynx-ebpf-common   (advanced; not on the hot path)
```

## Design rules

1. **Inject the filesystem.** `SysPaths` names `proc`, `cgroup`, `passwd`, and `os-release`. Production uses `/`. Tests and `lynxops --sysroot` use a captured tree. This is why parsers can run in CI on a non-Linux runner.
2. **Types live in `lynx-core`.** No crate returns ad-hoc maps when a struct will do. Snapshots are just `serde` of those structs.
3. **Correlate, do not just list.** A process summary already carries `cgroup` and a derived systemd `unit`. `inspect` attaches sockets by inode. Snapshots keep failed units next to the process table.
4. **Privilege-aware.** Missing `/proc/<pid>/io` or `fd` is a gap, not a crash. Journal and systemd are optional: if `systemctl` is absent, those views stay empty.
5. **eBPF last.** Userspace collectors are enough for a serious first release. `lynx-ebpf` exposes capability detection and `repr(C)` event layouts so probes can land without rewriting the rest.

## Data flow for `lynxops inspect <pid>`

1. `lynx-proc` reads `stat`, `status`, `cmdline`, `environ`, `io`, `limits`, `fd`, `ns`, `cgroup`, `task`.
2. `unit_from_cgroup_path` turns `/system.slice/nginx.service` into `nginx.service`.
3. `lynx-network` loads `/proc/net/{tcp,tcp6,udp,udp6,unix}` and joins `socket:[inode]` from `fd`.
4. The CLI prints one record. `--json` emits the same `ProcessDetail`.

## CPU accounting

`ps` and the TUI take two `/proc/stat` + `/proc/<pid>/stat` samples (~150–200 ms). Process CPU% is:

```text
(delta(utime+stime) / delta(system total ticks)) * logical_cpus * 100
```

`CLK_TCK` and page size come from `sysconf` on Linux and fall back to 100 / 4096 elsewhere.

## Snapshot contract

`IncidentSnapshot` version `1` is a single JSON document:

- host (load, memory, kernel, os-release)
- process table (CPU-sorted)
- listening sockets
- hot cgroups (high memory or PSI)
- failed systemd units
- journal priority `err` from the current boot

The file is the hand-off between “box is on fire” and “read this later on a laptop.”

## Release sequencing

```text
process → cgroup → net → systemd/journal → TUI → snapshot → eBPF
```

Each layer is a crate with its own tests. The CLI only composes them.

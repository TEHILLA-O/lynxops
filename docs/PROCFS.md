# procfs

LynxOps treats `/proc` as the source of truth for processes and host counters. This note is the map we implement, not a reprint of `proc(5)`.

## Host files

| File | Use |
| --- | --- |
| `/proc/stat` | Aggregate CPU ticks (`cpu` line) and logical CPU count (`cpuN`) |
| `/proc/meminfo` | `MemTotal`, `MemAvailable`, swap |
| `/proc/loadavg` | 1/5/15, runnable/total, last pid |
| `/proc/uptime` | Seconds since boot (process start-time math) |
| `/proc/version` | Kernel banner |
| `/proc/sys/kernel/hostname` | Hostname without `uname` |
| `/proc/net/tcp{,6}`, `udp{,6}`, `unix` | Socket tables |
| `/proc/net/dev` | Interface counters |

`MemAvailable` is preferred over `MemTotal - MemFree`. Cached + reclaimable is already in that field on modern kernels.

## Per-process files

| File | Use |
| --- | --- |
| `stat` | pid, comm (may contain spaces), state, ppid, utime/stime, threads, vsize, rss pages, starttime, nice |
| `status` | Uid, VmRSS (kB), capabilities, umask |
| `cmdline` | NUL-separated argv; empty for kernel threads |
| `environ` | NUL-separated `KEY=VALUE`; secrets redacted in the CLI |
| `io` | `read_bytes` / `write_bytes` (needs root or `CAP_SYS_PTRACE` on many distros) |
| `limits` | Soft/hard rlimits (`Max open files` is the usual incident tell) |
| `fd/` | Symlinks; `socket:[inode]` joins the net table |
| `ns/` | Namespace inode identities (`pid:[4026531836]`) |
| `cgroup` | v2 `0::/path` or v1 `id:controller:/path` |
| `exe`, `cwd` | Realpath via `readlink` |
| `task/` | Threads (`tid`, `comm`, `stat`) |

## Parsing `stat`

`comm` is between the first `(` and the last `)`. Never `split_whitespace` the whole line — a comm like `Isolated Web Co` will shift every field.

RSS in `stat` is pages. Multiply by `sysconf(_SC_PAGESIZE)` (usually 4096). `status` `VmRSS` is already kB and is preferred in `inspect`.

## Races

Processes exit between `readdir(/proc)` and `open(stat)`. Collectors treat `NotFound` / `PermissionDenied` as “skip this pid.” The TUI refreshes; it does not retry a single vanished task.

## Offline / tests

`ProcCollector::new(SysPaths { proc: fixture, .. })` reads the same paths. See `tests/fixtures/sysroot`.

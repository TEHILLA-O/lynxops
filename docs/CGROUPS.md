# cgroups

LynxOps reads cgroupfs directly. It does not require `systemd-cgtop` or libcgroup.

## Versions

| Detect | Meaning |
| --- | --- |
| `$CGROUP/cgroup.controllers` exists | **v2** (unified) |
| `$CGROUP/memory` or `$CGROUP/cpu` is a directory | **v1** |
| v1 mounts plus `$CGROUP/unified/cgroup.controllers` | **hybrid** |

`lynxops doctor` prints the detected version.

## v2 files we use

| File | Meaning |
| --- | --- |
| `cgroup.procs` | PIDs in this node (not the whole subtree unless it is a leaf) |
| `cgroup.controllers` | Controllers available here |
| `memory.current` / `memory.max` / `memory.high` | Working set vs hard/soft cap (`max` = unlimited) |
| `memory.stat` | `anon`, `file`, `kernel`, `slab`, … |
| `memory.pressure` | PSI `some` / `full` averages |
| `cpu.stat` | `usage_usec` |
| `pids.current` / `pids.max` | Fork bombs and `TasksMax=` |
| `io.stat` | Per-device rbytes/wbytes |

## v1

On a v1 host the hierarchy is per-controller (`/sys/fs/cgroup/memory/...`). LynxOps walks `memory`, `cpu`, and `pids` from the root and reads `memory.usage_in_bytes` / `memory.limit_in_bytes`.

## Incident signals

A cgroup is marked **hot** in snapshots when:

- `memory.current / memory.max > 0.80`, or
- current memory > 256 MiB with no max, or
- PSI `some avg10 >= 5` or `full avg10 >= 1`

Those thresholds are conservative on purpose: snapshots should be small and still catch the unit that is thrashing.

## systemd mapping

On a systemd host the v2 path *is* the unit path:

```text
0::/system.slice/nginx.service  →  nginx.service
0::/user.slice/user-1000.slice/session-3.scope
```

`unit_from_cgroup_path` takes the last component when it ends in `.service`, `.scope`, `.slice`, `.socket`, `.timer`, `.mount`, `.path`, or `.target`.

# Troubleshooting

## `lynxops ps` is empty or errors on `/proc`

You are not on Linux, or `--proc-root` / `--sysroot` is wrong.

```bash
lynxops doctor
lynxops --sysroot ./tests/fixtures/sysroot ps --no-sample
```

`doctor` must report `procfs ok`.

## CPU% is always `-`

You passed `--no-sample`, the interval was too short for any tick to land, or `/proc/stat` is missing. The TUI uses 150 ms; `ps` uses 200 ms. On an idle fixture both samples are identical, so CPU stays unset — that is correct.

## `inspect` has no sockets / no io

- `io` and some `fd` links need ptrace scope (`kernel.yama.ptrace_scope`) or the same user / root.
- Sockets join through `socket:[inode]`. If `fd` is unreadable, the net table still lists the socket but `pid` stays empty.

## cgroup list is empty

cgroupfs is not mounted at `/sys/fs/cgroup`, or you pointed `--cgroup-root` at a v1 controller leaf. `lynxops doctor` prints the version. On hybrid systems we look for `cgroup.controllers` first (unified) and treat the rest as v1.

## systemd / journal commands fail

LynxOps shells out to `systemctl` and `journalctl`. If they are missing (containers, non-systemd, Windows):

```text
lynxops: command `systemctl …` failed: …
```

Process/cgroup/net still work. Snapshots then omit failed units and journal errors.

`journalctl -o json` must be available (systemd 187+). We do not link `libsystemd`.

## TUI garbled after a crash

The alternate screen may not have been restored:

```bash
reset
```

## Permission denied on another user’s process

Expected. LynxOps skips that pid in `ps` and returns `permission denied` on `inspect`. Run as root only when you need the whole box.

## eBPF says `not built`

0.1 does not load probes. See [EBPF.md](EBPF.md). `doctor` reporting `kernel BTF` is enough to know the *host* is ready for a later build.

## JSON in a pipe

`--json` is global:

```bash
lynxops --json inspect 1 | jq .summary.unit
```

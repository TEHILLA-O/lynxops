# LynxOps

**Linux observability and incident-triage CLI/TUI.**

See [FAILURES.md](./FAILURES.md) for what can go wrong, what broke, how it was fixed, and results.

LynxOps reads the host through `/proc`, cgroupfs, socket tables, systemd, and the journal, then joins those views so a process is never just a PID — it is a unit, a cgroup, a set of sockets, and a recent log trail. eBPF is the advanced layer, not the on-ramp.

```
LedgerX    →  C# / FinTech / distributed backend
Sentinel   →  C# / Kafka / real-time systems
Atlas      →  Python / LangGraph / RAG / AI agents
LynxOps    →  Rust / Linux / eBPF / systems engineering
```

## First-release order

| Layer | Status in 0.1 | Command |
| --- | --- | --- |
| Process inspection | **done** | `lynxops ps`, `lynxops inspect <pid>` |
| cgroups | **done** | `lynxops cgroup list`, `lynxops cgroup inspect` |
| Networking | **done** | `lynxops net listen\|conn\|pid\|if` |
| systemd / journal | **done** | `lynxops systemd`, `lynxops journal` |
| TUI | **done** | `lynxops tui` |
| Snapshots | **done** | `lynxops snapshot capture\|show` |
| eBPF | scaffold | `lynxops doctor` (capability probe) |

That order is intentional: you get a usable triage tool on day one, and kernel-level tracing lands later without blocking the rest.

## Build

Linux is the target. Parsers are fixture-tested and compile on other OSes; live collectors expect a real `/proc`.

```bash
cargo build --release -p lynx-cli
sudo install -m 0755 target/release/lynxops /usr/local/bin/lynxops
```

```bash
# replay a captured sysroot (CI / offline triage)
lynxops --sysroot ./tests/fixtures/sysroot ps
```

## Usage

```text
lynxops ps --sort cpu -n 20
lynxops inspect 1420
lynxops cgroup inspect /system.slice/nginx.service
lynxops net listen
lynxops systemd unit nginx.service
lynxops journal -p err -n 40
lynxops snapshot capture -o /tmp/incident.json
lynxops tui
lynxops doctor
```

`inspect` correlates cmdline, cgroup, systemd unit, namespaces, fds, and sockets. Environment variables matching `PASSWORD`, `TOKEN`, `SECRET`, and similar names are redacted unless you pass `--show-secrets`.

TUI keys: `q` quit, `tab` views, `/` filter, `r` refresh, `c/m/p/n` sort, `j/k` move.

Every command accepts `--json`.

## Workspace

```text
lynxops/
├── crates/
│   ├── lynx-cli/            # clap binary (`lynxops`)
│   ├── lynx-core/           # types, errors, formatting
│   ├── lynx-tui/            # ratatui dashboard
│   ├── lynx-proc/           # /proc collectors
│   ├── lynx-network/        # /proc/net + inode→pid join
│   ├── lynx-cgroup/         # cgroup v1/v2
│   ├── lynx-systemd/        # systemctl / journalctl
│   ├── lynx-ebpf/           # userspace probe manager (scaffold)
│   ├── lynx-ebpf-common/    # repr(C) event types
│   └── lynx-snapshot/       # incident JSON snapshots
├── docs/
├── tests/
└── packaging/
```

Collectors take [`SysPaths`](crates/lynx-core/src/paths.rs), so the same code reads a live host or a fixture tree.

## Docs

- [ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [PROCFS.md](docs/PROCFS.md)
- [CGROUPS.md](docs/CGROUPS.md)
- [NAMESPACES.md](docs/NAMESPACES.md)
- [EBPF.md](docs/EBPF.md)
- [TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md)

## License

MIT — see [LICENSE](LICENSE).

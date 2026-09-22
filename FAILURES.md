# Failure modes, fixes, and results

Honest engineering notes for this project. Nothing here is invented for polish.

## What can go wrong

- **Empty `ps` / proc errors off Linux or wrong sysroot.** Impact: no triage data. Mitigation: `lynxops doctor`, `--sysroot` fixtures for CI/offline (`docs/TROUBLESHOOTING.md`).
- **CPU% stuck as `-`.** Impact: misleading load view. Mitigation: document sampling interval; identical idle fixture samples correctly leave CPU unset.
- **Missing sockets/io under ptrace_scope or permissions.** Impact: incomplete `inspect`. Mitigation: degrade gracefully; permission denied skips pid rather than crashing.
- **Secret environ leakage in snapshots.** Impact: credential exposure. Mitigation: redact `PASSWORD`/`TOKEN`/`SECRET`-like names unless `--show-secrets`; do not commit live environ dumps (`SECURITY.md`).

## What went wrong

**No recorded production incident in this repo yet.** Troubleshooting captures real operator failure modes the tool already hits (missing systemctl/journalctl in containers, TUI alternate screen left dirty after crash, eBPF scaffold reporting `not built` in 0.1).

## How it was resolved

- Collectors take `SysPaths` so the same code reads live `/proc` or fixtures.
- `doctor` capability probe; snapshots omit failed systemd/journal sections instead of aborting the whole capture.
- eBPF deferred by design; process/cgroup/net/systemd/TUI ship first.

## Results

- First-release table in README: process, cgroup, net, systemd/journal, TUI, snapshots marked done; eBPF scaffold only.
- Successful demo: `cargo build -p lynx-cli`, `lynxops --sysroot ./tests/fixtures/sysroot ps`, `lynxops doctor`.

# Security Policy

## Authorized systems only

LynxOps inspects process, cgroup, network, systemd, and journal data on the local host (or a captured sysroot). Use it only on systems you are authorized to administer or triage.

## Supported versions

Fixes land on the default branch (`main`).

## Reporting a vulnerability

Please open a private GitHub security advisory, or contact the maintainer via the GitHub profile, with:

- Affected crate (`lynx-proc`, `lynx-network`, TUI, snapshot, eBPF scaffold)
- Whether secret redaction, sysroot handling, or privilege gaps are involved
- Reproduction steps using fixtures when possible

## Responsible use

- Prefer fixtures and `--sysroot` for sharing incident material outside the host.
- Do not commit captured environ dumps that contain live credentials.
- Elevated permissions may be required for some `/proc` fields; missing data should remain a gap, not a crash.
- eBPF probes (when enabled later) must stay capability-checked via `lynxops doctor`.

## Scope of this policy

This document covers the LynxOps codebase. It does not authorize inspection of hosts without permission.

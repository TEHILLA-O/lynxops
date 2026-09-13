# Contributing

Thanks for helping with LynxOps. Keep collectors injectable through `SysPaths` so fixtures and `--sysroot` stay first-class.

## Prerequisites

- Rust 1.80 or newer (see workspace and `rust-toolchain.toml`)
- Linux for live `/proc`, cgroup, socket, systemd, and journal collectors
- Parsers are fixture-tested and should compile on other OSes

## Setup

```bash
cargo build --release -p lynx-cli
```

Optional install:

```bash
sudo install -m 0755 target/release/lynxops /usr/local/bin/lynxops
```

## Run

```bash
cargo run -p lynx-cli -- --help
cargo run -p lynx-cli -- ps --sort cpu -n 20
cargo run -p lynx-cli -- --sysroot ./tests/fixtures/sysroot ps
cargo run -p lynx-cli -- doctor
```

## Test

```bash
cargo test --workspace
```

Format:

```bash
cargo fmt
```

## Guidelines

- Put shared types in `lynx-core`. Avoid ad-hoc maps when a struct will do.
- Redact environment values matching password/token/secret patterns unless `--show-secrets` is set.
- Treat eBPF as the advanced layer: do not block userspace collectors on probe availability.
- Keep commit messages short and human.

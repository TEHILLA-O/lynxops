use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use lynx_core::{Pid, ProcessSort, SysPaths};

#[derive(Debug, Parser)]
#[command(
    name = "lynxops",
    version,
    about = "Linux observability and incident-triage CLI/TUI",
    long_about = "LynxOps inspects processes, cgroups, sockets, systemd, and the journal,\nthen correlates them for incident triage. eBPF probes are the advanced layer."
)]
pub struct Cli {
    /// Treat DIR as the host root (`DIR/proc`, `DIR/sys/fs/cgroup`).
    #[arg(long, global = true, value_name = "DIR")]
    pub sysroot: Option<PathBuf>,

    /// Override procfs mount (default /proc).
    #[arg(long, global = true, value_name = "DIR")]
    pub proc_root: Option<PathBuf>,

    /// Override cgroupfs mount (default /sys/fs/cgroup).
    #[arg(long, global = true, value_name = "DIR")]
    pub cgroup_root: Option<PathBuf>,

    /// Machine-readable JSON instead of tables.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List processes (CPU sampled over a short interval)
    Ps {
        #[arg(long, value_enum, default_value_t = SortArg::Cpu)]
        sort: SortArg,
        /// Show parent/child tree
        #[arg(long)]
        tree: bool,
        /// Limit after sorting
        #[arg(short = 'n', long)]
        limit: Option<usize>,
        /// Substring filter on comm, user, or unit
        #[arg(long)]
        filter: Option<String>,
        /// Skip the CPU sample sleep
        #[arg(long)]
        no_sample: bool,
    },
    /// Deep inspection of one process
    Inspect {
        pid: Pid,
        /// Do not redact PASSWORD/TOKEN/SECRET-like environ keys
        #[arg(long)]
        show_secrets: bool,
    },
    /// Interactive dashboard
    Tui,
    /// Live process view (alias for tui)
    Top,
    /// cgroup v1/v2 inspection
    Cgroup {
        #[command(subcommand)]
        action: Option<CgroupCmd>,
    },
    /// Sockets and interfaces
    Net {
        #[command(subcommand)]
        action: Option<NetCmd>,
    },
    /// systemd units
    Systemd {
        #[command(subcommand)]
        action: Option<SystemdCmd>,
    },
    /// journalctl wrapper with structured output
    Journal {
        #[arg(short = 'n', long, default_value_t = 50)]
        lines: u32,
        #[arg(short = 'p', long)]
        priority: Option<String>,
        #[arg(short = 'u', long)]
        unit: Option<String>,
        /// Include previous boots
        #[arg(long)]
        all_boots: bool,
    },
    /// Capture or replay an incident snapshot
    Snapshot {
        #[command(subcommand)]
        action: SnapshotCmd,
    },
    /// Host capability report (/proc, cgroup, systemd, eBPF)
    Doctor,
}

#[derive(Debug, Subcommand)]
pub enum CgroupCmd {
    /// Walk the hierarchy
    List {
        #[arg(long, default_value_t = 5)]
        depth: usize,
    },
    /// Inspect one cgroup path (e.g. /system.slice/nginx.service)
    Inspect { path: String },
}

#[derive(Debug, Subcommand)]
pub enum NetCmd {
    /// Listening sockets (and UDP binds)
    Listen,
    /// Established TCP connections
    Conn,
    /// Sockets owned by a process
    Pid { pid: Pid },
    /// /proc/net/dev counters
    If,
}

#[derive(Debug, Subcommand)]
pub enum SystemdCmd {
    Units {
        #[arg(long)]
        failed: bool,
    },
    Unit {
        name: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum SnapshotCmd {
    /// Write a JSON snapshot of host + processes + listeners + failed units
    Capture {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Print a previously captured snapshot
    Show { path: PathBuf },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SortArg {
    Cpu,
    Mem,
    Pid,
    Name,
}

impl From<SortArg> for ProcessSort {
    fn from(value: SortArg) -> Self {
        match value {
            SortArg::Cpu => ProcessSort::Cpu,
            SortArg::Mem => ProcessSort::Memory,
            SortArg::Pid => ProcessSort::Pid,
            SortArg::Name => ProcessSort::Name,
        }
    }
}

impl Cli {
    pub fn paths(&self) -> SysPaths {
        let mut paths = if let Some(root) = &self.sysroot {
            SysPaths::from_sysroot(root)
        } else {
            SysPaths::live()
        };
        if let Some(p) = &self.proc_root {
            paths.proc = p.clone();
        }
        if let Some(c) = &self.cgroup_root {
            paths.cgroup = c.clone();
        }
        paths
    }
}

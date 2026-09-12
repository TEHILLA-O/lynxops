use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::cgroup::CgroupRef;
use crate::ids::{Inode, Pid, Uid};
use crate::net::Socket;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProcessState {
    Running,
    Sleeping,
    DiskSleep,
    Stopped,
    Tracing,
    Zombie,
    Dead,
    Idle,
    Parked,
    Unknown(char),
}

impl ProcessState {
    pub fn from_char(c: char) -> Self {
        match c {
            'R' => Self::Running,
            'S' => Self::Sleeping,
            'D' => Self::DiskSleep,
            'T' => Self::Stopped,
            't' => Self::Tracing,
            'Z' => Self::Zombie,
            'X' | 'x' => Self::Dead,
            'I' => Self::Idle,
            'P' => Self::Parked,
            other => Self::Unknown(other),
        }
    }

    pub fn as_char(self) -> char {
        match self {
            Self::Running => 'R',
            Self::Sleeping => 'S',
            Self::DiskSleep => 'D',
            Self::Stopped => 'T',
            Self::Tracing => 't',
            Self::Zombie => 'Z',
            Self::Dead => 'X',
            Self::Idle => 'I',
            Self::Parked => 'P',
            Self::Unknown(c) => c,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Sleeping => "sleeping",
            Self::DiskSleep => "disk-sleep",
            Self::Stopped => "stopped",
            Self::Tracing => "tracing",
            Self::Zombie => "zombie",
            Self::Dead => "dead",
            Self::Idle => "idle",
            Self::Parked => "parked",
            Self::Unknown(_) => "unknown",
        }
    }
}

impl std::fmt::Display for ProcessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.as_char(), self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSummary {
    pub pid: Pid,
    pub ppid: Pid,
    pub comm: String,
    pub state: ProcessState,
    pub uid: Uid,
    pub user: Option<String>,
    pub threads: u32,
    pub nice: i64,
    pub cpu_percent: Option<f32>,
    pub mem_rss_bytes: u64,
    pub mem_vsz_bytes: u64,
    pub mem_percent: Option<f32>,
    pub cgroup: Option<String>,
    pub unit: Option<String>,
    pub start_time_ticks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDetail {
    pub summary: ProcessSummary,
    pub cmdline: Vec<String>,
    pub exe: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
    pub environ: Vec<(String, String)>,
    pub namespaces: Namespaces,
    pub cgroup: Option<CgroupRef>,
    pub io: IoStats,
    pub limits: Vec<ResourceLimit>,
    pub fds: Vec<Fd>,
    pub fd_count: usize,
    pub sockets: Vec<Socket>,
    pub threads: Vec<ThreadInfo>,
    pub capabilities: Option<Capabilities>,
    pub umask: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Namespaces {
    pub pid: Option<String>,
    pub mnt: Option<String>,
    pub net: Option<String>,
    pub uts: Option<String>,
    pub ipc: Option<String>,
    pub user: Option<String>,
    pub cgroup: Option<String>,
    pub time: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IoStats {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_chars: u64,
    pub write_chars: u64,
    pub syscr: u64,
    pub syscw: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimit {
    pub name: String,
    pub soft: String,
    pub hard: String,
    pub units: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fd {
    pub fd: i32,
    pub target: String,
    pub inode: Option<Inode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadInfo {
    pub tid: Pid,
    pub comm: String,
    pub state: ProcessState,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Capabilities {
    pub cap_inh: String,
    pub cap_prm: String,
    pub cap_eff: String,
    pub cap_bnd: String,
    pub cap_amb: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessSort {
    #[default]
    Cpu,
    Memory,
    Pid,
    Name,
}

impl ProcessSort {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "cpu" => Some(Self::Cpu),
            "mem" | "memory" | "rss" => Some(Self::Memory),
            "pid" => Some(Self::Pid),
            "name" | "comm" => Some(Self::Name),
            _ => None,
        }
    }
}

pub fn sort_processes(rows: &mut [ProcessSummary], sort: ProcessSort) {
    match sort {
        ProcessSort::Cpu => rows.sort_by(|a, b| {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.mem_rss_bytes.cmp(&a.mem_rss_bytes))
        }),
        ProcessSort::Memory => rows.sort_by_key(|p| std::cmp::Reverse(p.mem_rss_bytes)),
        ProcessSort::Pid => rows.sort_by_key(|p| p.pid),
        ProcessSort::Name => rows.sort_by(|a, b| a.comm.cmp(&b.comm)),
    }
}

/// Last path component of a cgroup path when it looks like a systemd unit.
pub fn unit_from_cgroup_path(path: &str) -> Option<String> {
    let name = path.rsplit('/').next().filter(|s| !s.is_empty())?;
    const SUFFIXES: &[&str] = &[
        ".service", ".scope", ".slice", ".socket", ".timer", ".mount", ".path", ".target",
    ];
    if SUFFIXES.iter().any(|s| name.ends_with(s)) {
        Some(name.to_string())
    } else {
        None
    }
}

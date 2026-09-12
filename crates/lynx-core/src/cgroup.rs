use serde::{Deserialize, Serialize};

use crate::ids::Pid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CgroupVersion {
    V1,
    V2,
    Hybrid,
}

impl std::fmt::Display for CgroupVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1 => write!(f, "v1"),
            Self::V2 => write!(f, "v2"),
            Self::Hybrid => write!(f, "hybrid"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CgroupRef {
    pub version: CgroupVersion,
    pub path: String,
    /// Controller → path for cgroup v1; empty on v2.
    pub controllers: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CgroupSummary {
    pub path: String,
    pub nprocs: usize,
    pub memory_current: Option<u64>,
    pub memory_max: Option<u64>,
    pub memory_high: Option<u64>,
    pub cpu_usage_usec: Option<u64>,
    pub pids_current: Option<u64>,
    pub pids_max: Option<u64>,
    pub pressure: Option<Pressure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CgroupDetail {
    pub summary: CgroupSummary,
    pub version: CgroupVersion,
    pub controllers: Vec<String>,
    pub procs: Vec<Pid>,
    pub memory_stat: Vec<(String, u64)>,
    pub cpu_stat: Vec<(String, u64)>,
    pub io_stat: Vec<String>,
    pub subtree: Vec<CgroupSummary>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Pressure {
    pub some_avg10: f32,
    pub some_avg60: f32,
    pub some_avg300: f32,
    pub full_avg10: f32,
}

impl Pressure {
    pub fn is_elevated(self) -> bool {
        self.some_avg10 >= 5.0 || self.full_avg10 >= 1.0
    }
}

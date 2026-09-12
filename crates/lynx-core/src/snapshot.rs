use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::cgroup::CgroupSummary;
use crate::host::HostSnapshot;
use crate::net::Socket;
use crate::process::ProcessSummary;
use crate::systemd::{JournalEntry, SystemdUnit};

pub const SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentSnapshot {
    pub version: u32,
    pub captured_at: DateTime<Utc>,
    pub lynxops_version: String,
    pub host: HostSnapshot,
    pub processes: Vec<ProcessSummary>,
    pub listening: Vec<Socket>,
    pub hot_cgroups: Vec<CgroupSummary>,
    pub failed_units: Vec<SystemdUnit>,
    pub journal_errors: Vec<JournalEntry>,
}

impl IncidentSnapshot {
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}

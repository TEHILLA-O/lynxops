//! Point-in-time incident snapshots: host, processes, listeners, hot cgroups,
//! failed units, and recent journal errors.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use lynx_cgroup::CgroupCollector;
use lynx_core::{
    sort_processes, IncidentSnapshot, ProcessSort, Result, SysPaths, SNAPSHOT_VERSION,
};
use lynx_network::NetCollector;
use lynx_proc::{load_host, ProcCollector};
use lynx_systemd::SystemdCollector;

#[derive(Debug, Clone)]
pub struct SnapshotOptions {
    pub paths: SysPaths,
    pub process_sample: Duration,
    pub process_limit: Option<usize>,
    pub hot_cgroups: usize,
    pub journal_lines: u32,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            paths: SysPaths::live(),
            process_sample: Duration::from_millis(200),
            process_limit: None,
            hot_cgroups: 16,
            journal_lines: 80,
        }
    }
}

pub fn capture(opts: &SnapshotOptions) -> IncidentSnapshot {
    let host = load_host(&opts.paths).unwrap_or_else(|_| fallback_host());
    let proc = ProcCollector::new(opts.paths.clone());
    let mut processes = proc
        .list_with_sample(opts.process_sample)
        .unwrap_or_default();
    sort_processes(&mut processes, ProcessSort::Cpu);
    if let Some(limit) = opts.process_limit {
        processes.truncate(limit);
    }

    let net = NetCollector::new(opts.paths.clone());
    let listening = net.listening().unwrap_or_default();

    let cgroup = CgroupCollector::new(opts.paths.clone());
    let hot_cgroups = cgroup.hot(opts.hot_cgroups).unwrap_or_default();

    let systemd = SystemdCollector::new();
    let failed_units = systemd.failed_units().unwrap_or_default();
    let journal_errors = systemd
        .journal_errors(opts.journal_lines)
        .unwrap_or_default();

    IncidentSnapshot {
        version: SNAPSHOT_VERSION,
        captured_at: Utc::now(),
        lynxops_version: lynx_core::VERSION.to_string(),
        host,
        processes,
        listening,
        hot_cgroups,
        failed_units,
        journal_errors,
    }
}

pub fn save(snapshot: &IncidentSnapshot, path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref().to_path_buf();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| lynx_core::LynxError::io(e, parent.to_path_buf()))?;
        }
    }
    let json = serde_json::to_string_pretty(snapshot)
        .map_err(|e| lynx_core::LynxError::Other(e.to_string()))?;
    fs::write(&path, json).map_err(|e| lynx_core::LynxError::io(e, path.clone()))?;
    Ok(path)
}

pub fn load(path: impl AsRef<Path>) -> Result<IncidentSnapshot> {
    let path = path.as_ref();
    let text =
        fs::read_to_string(path).map_err(|e| lynx_core::LynxError::io(e, path.to_path_buf()))?;
    serde_json::from_str(&text).map_err(|e| lynx_core::LynxError::parse_at(e.to_string(), path))
}

pub fn default_path() -> PathBuf {
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    PathBuf::from(format!("lynxops-snapshot-{stamp}.json"))
}

fn fallback_host() -> lynx_core::HostSnapshot {
    lynx_core::HostSnapshot {
        hostname: None,
        kernel: None,
        os_pretty: None,
        uptime_secs: 0,
        loadavg: lynx_core::LoadAvg::default(),
        memory: lynx_core::MemoryInfo::default(),
        cpu: lynx_core::CpuInfo::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_empty() {
        let snap = IncidentSnapshot {
            version: SNAPSHOT_VERSION,
            captured_at: Utc::now(),
            lynxops_version: "0.1.0".into(),
            host: fallback_host(),
            processes: vec![],
            listening: vec![],
            hot_cgroups: vec![],
            failed_units: vec![],
            journal_errors: vec![],
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: IncidentSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.version, SNAPSHOT_VERSION);
        assert_eq!(back.process_count(), 0);
    }
}

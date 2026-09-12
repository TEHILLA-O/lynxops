//! cgroup v1 / v2 inspection from `/sys/fs/cgroup`.

use std::fs;
use std::path::{Path, PathBuf};

use lynx_core::{
    CgroupDetail, CgroupSummary, CgroupVersion, LynxError, Pid, Pressure, Result, SysPaths,
};

#[derive(Debug, Clone)]
pub struct CgroupCollector {
    pub paths: SysPaths,
}

impl Default for CgroupCollector {
    fn default() -> Self {
        Self::new(SysPaths::live())
    }
}

impl CgroupCollector {
    pub fn new(paths: SysPaths) -> Self {
        Self { paths }
    }

    pub fn version(&self) -> Result<CgroupVersion> {
        detect_version(&self.paths.cgroup)
    }

    pub fn list(&self, max_depth: usize) -> Result<Vec<CgroupSummary>> {
        let root = &self.paths.cgroup;
        if !root.exists() {
            return Err(LynxError::not_found(format!(
                "cgroup fs {} (not mounted or --cgroup-root is wrong)",
                root.display()
            )));
        }
        let version = detect_version(root)?;
        let mut out = Vec::new();
        walk_cgroups(root, root, version, 0, max_depth, &mut out)?;
        out.sort_by_key(|c| std::cmp::Reverse(c.memory_current.unwrap_or(0)));
        Ok(out)
    }

    pub fn inspect(&self, path: &str) -> Result<CgroupDetail> {
        let root = &self.paths.cgroup;
        let version = detect_version(root)?;
        let rel = path.trim_start_matches('/');
        let dir = if rel.is_empty() {
            root.clone()
        } else {
            root.join(rel)
        };
        if !dir.exists() {
            return Err(LynxError::not_found(format!("cgroup {path}")));
        }
        let summary = read_summary(root, &dir, version)?;
        let procs = read_pids(&dir.join("cgroup.procs"))
            .or_else(|_| read_pids(&dir.join("tasks")))
            .unwrap_or_default();
        let controllers = read_controllers(&dir, version);
        let memory_stat = read_stat_map(&dir.join("memory.stat"));
        let cpu_stat = read_stat_map(&dir.join("cpu.stat"));
        let io_stat = read_lines(&dir.join("io.stat"));
        let mut subtree = Vec::new();
        walk_cgroups(&dir, root, version, 0, 2, &mut subtree)?;
        subtree.retain(|s| s.path != summary.path);
        Ok(CgroupDetail {
            summary,
            version,
            controllers,
            procs,
            memory_stat,
            cpu_stat,
            io_stat,
            subtree,
        })
    }

    /// Cgroups that look "hot" during incident capture: high memory or PSI.
    pub fn hot(&self, limit: usize) -> Result<Vec<CgroupSummary>> {
        let mut all = self.list(6)?;
        all.retain(|c| {
            let mem_hot = match (c.memory_current, c.memory_max) {
                (Some(cur), Some(max)) if max > 0 => cur as f64 / max as f64 > 0.80,
                (Some(cur), _) => cur > 256 * 1024 * 1024,
                _ => false,
            };
            let psi = c.pressure.map(|p| p.is_elevated()).unwrap_or(false);
            mem_hot || psi
        });
        all.truncate(limit);
        Ok(all)
    }
}

pub fn detect_version(root: &Path) -> Result<CgroupVersion> {
    if root.join("cgroup.controllers").exists() {
        return Ok(CgroupVersion::V2);
    }
    if root.join("memory").is_dir() || root.join("cpu").is_dir() {
        if root.join("unified").join("cgroup.controllers").exists() {
            return Ok(CgroupVersion::Hybrid);
        }
        return Ok(CgroupVersion::V1);
    }
    if root.exists() {
        Ok(CgroupVersion::V2)
    } else {
        Err(LynxError::not_found(root.display().to_string()))
    }
}

fn walk_cgroups(
    dir: &Path,
    root: &Path,
    version: CgroupVersion,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<CgroupSummary>,
) -> Result<()> {
    if let Ok(summary) = read_summary(root, dir, version) {
        out.push(summary);
    }
    if depth >= max_depth {
        return Ok(());
    }
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(()),
    };
    for ent in rd.flatten() {
        let ft = match ent.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !ft.is_dir() {
            continue;
        }
        let name = ent.file_name();
        let skip = matches!(
            name.to_str(),
            Some("cgroup.controllers")
                | Some("cgroup.subtree_control")
                | Some("cpu")
                | Some("cpuacct")
                | Some("cpuset")
                | Some("devices")
                | Some("freezer")
                | Some("net_cls")
                | Some("net_prio")
                | Some("perf_event")
                | Some("pids")
                | Some("rdma")
                | Some("hugetlb")
                | Some("blkio")
        );
        // On v1 the controller mounts are siblings; still walk memory/ as the hierarchy.
        if version == CgroupVersion::V1 && dir == root {
            if name == "memory" || name == "cpu" || name == "pids" {
                walk_cgroups(&ent.path(), root, version, depth + 1, max_depth, out)?;
            }
            continue;
        }
        if skip && version == CgroupVersion::V1 {
            continue;
        }
        walk_cgroups(&ent.path(), root, version, depth + 1, max_depth, out)?;
    }
    Ok(())
}

fn read_summary(root: &Path, dir: &Path, version: CgroupVersion) -> Result<CgroupSummary> {
    let rel = dir.strip_prefix(root).unwrap_or(dir);
    let path = format!(
        "/{}",
        rel.to_string_lossy().trim_matches('/').replace('\\', "/")
    );
    let path = if path == "/" { "/".to_string() } else { path };

    let memory_dir = match version {
        CgroupVersion::V1 if dir == root => dir.join("memory"),
        CgroupVersion::V1 => {
            // If we are already under memory/, use dir; else try sibling rewrite.
            if dir.starts_with(root.join("memory")) {
                dir.to_path_buf()
            } else {
                rewrite_controller(root, dir, "memory")
            }
        }
        _ => dir.to_path_buf(),
    };

    let nprocs = read_pids(&dir.join("cgroup.procs"))
        .or_else(|_| read_pids(&dir.join("tasks")))
        .map(|p| p.len())
        .unwrap_or(0);

    let memory_current = read_u64(&memory_dir.join("memory.current"))
        .or_else(|| read_u64(&memory_dir.join("memory.usage_in_bytes")));
    let memory_max = read_max(&memory_dir.join("memory.max"))
        .or_else(|| read_u64(&memory_dir.join("memory.limit_in_bytes")));
    let memory_high = read_max(&memory_dir.join("memory.high"));
    let cpu_usage_usec = read_cpu_usage(dir);
    let pids_current = read_u64(&dir.join("pids.current"));
    let pids_max = read_max(&dir.join("pids.max"));
    let pressure = parse_pressure(&read_string(&dir.join("memory.pressure")).unwrap_or_default());

    Ok(CgroupSummary {
        path,
        nprocs,
        memory_current,
        memory_max,
        memory_high,
        cpu_usage_usec,
        pids_current,
        pids_max,
        pressure,
    })
}

fn rewrite_controller(root: &Path, dir: &Path, controller: &str) -> PathBuf {
    if let Ok(rel) = dir.strip_prefix(root) {
        root.join(controller).join(rel)
    } else {
        dir.to_path_buf()
    }
}

fn read_controllers(dir: &Path, version: CgroupVersion) -> Vec<String> {
    match version {
        CgroupVersion::V2 | CgroupVersion::Hybrid => read_string(&dir.join("cgroup.controllers"))
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_string)
            .collect(),
        CgroupVersion::V1 => vec!["memory".into(), "cpu".into(), "pids".into()],
    }
}

fn read_cpu_usage(dir: &Path) -> Option<u64> {
    if let Some(text) = read_string(&dir.join("cpu.stat")) {
        for line in text.lines() {
            let mut p = line.split_whitespace();
            if p.next() == Some("usage_usec") {
                return p.next().and_then(|s| s.parse().ok());
            }
        }
    }
    None
}

fn read_pids(path: &Path) -> Result<Vec<Pid>> {
    let text = fs::read_to_string(path).map_err(|e| LynxError::io(e, path.to_path_buf()))?;
    Ok(text
        .lines()
        .filter_map(|l| l.trim().parse::<u32>().ok().map(Pid))
        .collect())
}

fn read_stat_map(path: &Path) -> Vec<(String, u64)> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let mut p = line.split_whitespace();
            Some((p.next()?.to_string(), p.next()?.parse().ok()?))
        })
        .collect()
}

fn read_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

fn read_string(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn read_u64(path: &Path) -> Option<u64> {
    read_string(path)?.trim().parse().ok()
}

fn read_max(path: &Path) -> Option<u64> {
    let text = read_string(path)?;
    let t = text.trim();
    if t == "max" || t == "unlimited" {
        return None;
    }
    t.parse().ok()
}

pub fn parse_pressure(text: &str) -> Option<Pressure> {
    if text.is_empty() {
        return None;
    }
    let mut p = Pressure::default();
    let mut found = false;
    for line in text.lines() {
        let Some((kind, rest)) = line.split_once(' ') else {
            continue;
        };
        let mut avg10 = None;
        for field in rest.split_whitespace() {
            if let Some((k, v)) = field.split_once('=') {
                if k == "avg10" {
                    avg10 = v.parse().ok();
                }
                if k == "avg60" && kind == "some" {
                    p.some_avg60 = v.parse().unwrap_or(0.0);
                }
                if k == "avg300" && kind == "some" {
                    p.some_avg300 = v.parse().unwrap_or(0.0);
                }
            }
        }
        if let Some(v) = avg10 {
            found = true;
            if kind == "some" {
                p.some_avg10 = v;
            } else if kind == "full" {
                p.full_avg10 = v;
            }
        }
    }
    found.then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_parses() {
        let text = "\
some avg10=12.50 avg60=3.20 avg300=0.80 total=123
full avg10=1.25 avg60=0.10 avg300=0.00 total=4
";
        let p = parse_pressure(text).unwrap();
        assert!((p.some_avg10 - 12.5).abs() < 0.01);
        assert!((p.full_avg10 - 1.25).abs() < 0.01);
        assert!(p.is_elevated());
    }
}

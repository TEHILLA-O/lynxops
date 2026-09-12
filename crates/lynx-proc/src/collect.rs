use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use lynx_core::{
    unit_from_cgroup_path, CgroupRef, CgroupVersion, Fd, Inode, LynxError, Namespaces, Pid,
    ProcCpuSample, ProcessDetail, ProcessSummary, Result, SysPaths, ThreadInfo,
};

use crate::host::{self, read_cpu_sample};
use crate::stat::{parse_stat_line, PidStat};
use crate::status::{parse_io, parse_limits, parse_status};
use crate::users::{self, UserMap};
use crate::util::{
    page_size, parse_cmdline, parse_environ, read_link, read_to_string, read_to_vec, sleep_sample,
    ticks_per_second, try_read_to_string,
};

#[derive(Debug, Clone)]
pub struct ProcCollector {
    pub paths: SysPaths,
    pub page_size: u64,
    pub ticks_per_second: u64,
}

impl Default for ProcCollector {
    fn default() -> Self {
        Self::new(SysPaths::live())
    }
}

impl ProcCollector {
    pub fn new(paths: SysPaths) -> Self {
        Self {
            paths,
            page_size: page_size(),
            ticks_per_second: ticks_per_second(),
        }
    }

    pub fn list(&self) -> Result<Vec<ProcessSummary>> {
        self.list_with_sample(Duration::from_millis(200))
    }

    pub fn list_with_sample(&self, interval: Duration) -> Result<Vec<ProcessSummary>> {
        let users = users::load_users(&self.paths)?;
        let mem = host::parse_meminfo(&read_to_string(self.paths.proc.join("meminfo"))?);
        let first = self.scan_stat_samples()?;
        let sys_a = read_cpu_sample(&self.paths).ok();
        sleep_sample(interval);
        let second = self.scan_stat_samples()?;
        let sys_b = read_cpu_sample(&self.paths).ok();
        let logical = host::load_host(&self.paths)
            .map(|h| h.cpu.logical_cpus.max(1))
            .unwrap_or(1);
        let sys_delta = match (sys_a, sys_b) {
            (Some(a), Some(b)) => b.total_ticks.saturating_sub(a.total_ticks),
            _ => 0,
        };

        let mut out = Vec::with_capacity(second.len());
        for (pid, stat) in &second {
            let cpu = first.get(pid).and_then(|prev| {
                ProcCpuSample {
                    utime: prev.utime,
                    stime: prev.stime,
                }
                .usage_percent(
                    ProcCpuSample {
                        utime: stat.utime,
                        stime: stat.stime,
                    },
                    sys_delta,
                    logical,
                )
            });
            out.push(self.summary_from_stat(stat, &users, mem.total_bytes, cpu));
        }
        Ok(out)
    }

    /// Fast path used when a snapshot or TUI already sampled CPU elsewhere.
    pub fn list_once(&self) -> Result<Vec<ProcessSummary>> {
        let users = users::load_users(&self.paths)?;
        let mem = host::parse_meminfo(&read_to_string(self.paths.proc.join("meminfo"))?);
        let stats = self.scan_stat_samples()?;
        Ok(stats
            .values()
            .map(|stat| self.summary_from_stat(stat, &users, mem.total_bytes, None))
            .collect())
    }

    pub fn inspect(&self, pid: Pid, show_secrets: bool) -> Result<ProcessDetail> {
        let dir = self.paths.pid_dir(pid);
        if !dir.exists() {
            return Err(LynxError::not_found(format!("pid {pid}")));
        }
        let users = users::load_users(&self.paths)?;
        let mem = host::parse_meminfo(&read_to_string(self.paths.proc.join("meminfo"))?);
        let stat = parse_stat_line(&read_to_string(dir.join("stat"))?)?;
        let mut summary = self.summary_from_stat(&stat, &users, mem.total_bytes, None);

        let status = try_read_to_string(dir.join("status"))?.map(|t| parse_status(&t));
        if let Some(st) = &status {
            if let Some(uid) = st.uid {
                summary.uid = uid;
                summary.user = users::lookup(&users, uid);
            }
            if let Some(kb) = st.vm_rss_kb {
                summary.mem_rss_bytes = crate::util::kb_to_bytes(kb);
            }
        }

        let cmdline = try_read_to_string_bytes(&dir.join("cmdline"))
            .map(|b| parse_cmdline(&b))
            .unwrap_or_default();
        let environ_raw = try_read_to_string_bytes(&dir.join("environ")).unwrap_or_default();
        let environ = parse_environ(&environ_raw)
            .into_iter()
            .map(|(k, v)| {
                let value = lynx_core::redact::redact_value(&k, &v, show_secrets);
                (k, value)
            })
            .collect();

        let exe = read_link(dir.join("exe")).ok();
        let cwd = read_link(dir.join("cwd")).ok();
        let cgroup = parse_cgroup_file(try_read_to_string(dir.join("cgroup"))?.as_deref());
        if let Some(ref cg) = cgroup {
            summary.cgroup = Some(cg.path.clone());
            summary.unit = unit_from_cgroup_path(&cg.path);
        }

        let io = try_read_to_string(dir.join("io"))?
            .map(|t| parse_io(&t))
            .unwrap_or_default();
        let limits = try_read_to_string(dir.join("limits"))?
            .map(|t| parse_limits(&t))
            .unwrap_or_default();
        let (fds, fd_count) = self.list_fds(pid, 256);
        let threads = self.list_threads(pid);
        let namespaces = self.read_namespaces(pid);

        Ok(ProcessDetail {
            summary,
            cmdline,
            exe,
            cwd,
            environ,
            namespaces,
            cgroup,
            io,
            limits,
            fds,
            fd_count,
            sockets: Vec::new(),
            threads,
            capabilities: status.as_ref().map(|s| s.capabilities.clone()),
            umask: status.and_then(|s| s.umask),
        })
    }

    pub fn list_fds(&self, pid: Pid, cap: usize) -> (Vec<Fd>, usize) {
        let fd_dir = self.paths.pid_dir(pid).join("fd");
        let entries = match fs::read_dir(&fd_dir) {
            Ok(rd) => rd,
            Err(_) => return (Vec::new(), 0),
        };
        let mut fds = Vec::new();
        let mut total = 0usize;
        for ent in entries.flatten() {
            total += 1;
            if fds.len() >= cap {
                continue;
            }
            let name = ent.file_name();
            let Ok(fd) = name.to_string_lossy().parse::<i32>() else {
                continue;
            };
            let target = fs::read_link(ent.path())
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "?".into());
            let inode = parse_socket_inode(&target);
            fds.push(Fd { fd, target, inode });
        }
        fds.sort_by_key(|f| f.fd);
        (fds, total)
    }

    fn list_threads(&self, pid: Pid) -> Vec<ThreadInfo> {
        let task = self.paths.pid_dir(pid).join("task");
        let Ok(rd) = fs::read_dir(task) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for ent in rd.flatten() {
            let Ok(tid) = ent.file_name().to_string_lossy().parse::<u32>() else {
                continue;
            };
            let comm = read_to_string(ent.path().join("comm"))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            let state = read_to_string(ent.path().join("stat"))
                .ok()
                .and_then(|l| parse_stat_line(&l).ok())
                .map(|s| s.state)
                .unwrap_or(lynx_core::ProcessState::Unknown('?'));
            out.push(ThreadInfo {
                tid: Pid(tid),
                comm,
                state,
            });
        }
        out.sort_by_key(|t| t.tid);
        out
    }

    fn read_namespaces(&self, pid: Pid) -> Namespaces {
        let ns = self.paths.pid_dir(pid).join("ns");
        let read = |name: &str| {
            fs::read_link(ns.join(name))
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
        };
        Namespaces {
            pid: read("pid"),
            mnt: read("mnt"),
            net: read("net"),
            uts: read("uts"),
            ipc: read("ipc"),
            user: read("user"),
            cgroup: read("cgroup"),
            time: read("time"),
        }
    }

    fn scan_stat_samples(&self) -> Result<HashMap<Pid, PidStat>> {
        let rd = fs::read_dir(&self.paths.proc)
            .map_err(|e| lynx_core::LynxError::io(e, self.paths.proc.clone()))?;
        let mut map = HashMap::new();
        for ent in rd.flatten() {
            let name = ent.file_name();
            let Some(pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };
            let Ok(text) = read_to_string(ent.path().join("stat")) else {
                continue;
            };
            if let Ok(stat) = parse_stat_line(&text) {
                map.insert(Pid(pid), stat);
            }
        }
        Ok(map)
    }

    fn summary_from_stat(
        &self,
        stat: &PidStat,
        users: &UserMap,
        mem_total: u64,
        cpu_percent: Option<f32>,
    ) -> ProcessSummary {
        let rss = stat.rss_pages.saturating_mul(self.page_size);
        let mem_percent = if mem_total == 0 {
            None
        } else {
            Some((rss as f32 / mem_total as f32) * 100.0)
        };
        let (cgroup, unit) = self
            .read_cgroup_path(stat.pid)
            .map(|p| {
                let unit = unit_from_cgroup_path(&p);
                (Some(p), unit)
            })
            .unwrap_or((None, None));
        let uid = self.read_uid(stat.pid).unwrap_or(lynx_core::Uid(0));
        ProcessSummary {
            pid: stat.pid,
            ppid: stat.ppid,
            comm: stat.comm.clone(),
            state: stat.state,
            uid,
            user: users::lookup(users, uid),
            threads: stat.num_threads,
            nice: stat.nice,
            cpu_percent,
            mem_rss_bytes: rss,
            mem_vsz_bytes: stat.vsize,
            mem_percent,
            cgroup,
            unit,
            start_time_ticks: stat.starttime,
        }
    }

    fn read_cgroup_path(&self, pid: Pid) -> Option<String> {
        let text = read_to_string(self.paths.pid_dir(pid).join("cgroup")).ok()?;
        parse_cgroup_file(Some(&text)).map(|c| c.path)
    }

    fn read_uid(&self, pid: Pid) -> Option<lynx_core::Uid> {
        let text = read_to_string(self.paths.pid_dir(pid).join("status")).ok()?;
        parse_status(&text).uid
    }
}

fn try_read_to_string_bytes(path: &std::path::Path) -> Option<Vec<u8>> {
    read_to_vec(path).ok()
}

pub fn parse_cgroup_file(text: Option<&str>) -> Option<CgroupRef> {
    let text = text?;
    let mut controllers = Vec::new();
    let mut v2_path = None;
    for line in text.lines() {
        let mut parts = line.splitn(3, ':');
        let _id = parts.next()?;
        let ctrl = parts.next().unwrap_or("");
        let path = parts.next().unwrap_or("").to_string();
        if ctrl.is_empty() {
            v2_path = Some(path);
        } else {
            controllers.push((ctrl.to_string(), path));
        }
    }
    if let Some(path) = v2_path {
        Some(CgroupRef {
            version: if controllers.is_empty() {
                CgroupVersion::V2
            } else {
                CgroupVersion::Hybrid
            },
            path,
            controllers,
        })
    } else if let Some((_, path)) = controllers.iter().find(|(c, _)| c.contains("memory")) {
        Some(CgroupRef {
            version: CgroupVersion::V1,
            path: path.clone(),
            controllers,
        })
    } else {
        let path = controllers.first().map(|(_, path)| path.clone());
        path.map(|path| CgroupRef {
            version: CgroupVersion::V1,
            path,
            controllers,
        })
    }
}

pub fn parse_socket_inode(target: &str) -> Option<Inode> {
    let rest = target.strip_prefix("socket:[")?;
    let num = rest.strip_suffix(']')?;
    num.parse().ok().map(Inode)
}

pub fn start_age_secs(start_ticks: u64, uptime_secs: u64, ticks: u64) -> u64 {
    if ticks == 0 {
        return 0;
    }
    let start_secs = start_ticks / ticks;
    uptime_secs.saturating_sub(start_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cgroup_v2_and_unit() {
        let cg = parse_cgroup_file(Some("0::/system.slice/nginx.service\n")).unwrap();
        assert_eq!(cg.version, CgroupVersion::V2);
        assert_eq!(cg.path, "/system.slice/nginx.service");
        assert_eq!(
            unit_from_cgroup_path(&cg.path).as_deref(),
            Some("nginx.service")
        );
    }

    #[test]
    fn socket_inode() {
        assert_eq!(
            parse_socket_inode("socket:[33981]").map(|i| i.0),
            Some(33981)
        );
        assert_eq!(parse_socket_inode("/dev/null"), None);
    }
}

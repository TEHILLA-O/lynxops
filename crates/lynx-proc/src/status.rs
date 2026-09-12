use std::collections::HashMap;

use lynx_core::{Capabilities, IoStats, Uid};

#[derive(Debug, Clone, Default)]
pub struct PidStatus {
    pub name: Option<String>,
    pub state: Option<char>,
    pub pid: Option<u32>,
    pub ppid: Option<u32>,
    pub uid: Option<Uid>,
    pub gid: Option<u32>,
    pub threads: Option<u32>,
    pub vm_rss_kb: Option<u64>,
    pub vm_size_kb: Option<u64>,
    pub umask: Option<String>,
    pub capabilities: Capabilities,
    pub raw: HashMap<String, String>,
}

pub fn parse_status(text: &str) -> PidStatus {
    let mut st = PidStatus::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        st.raw.insert(key.to_string(), value.to_string());
        match key {
            "Name" => st.name = Some(value.to_string()),
            "State" => st.state = value.chars().next(),
            "Pid" => st.pid = value.split_whitespace().next().and_then(|s| s.parse().ok()),
            "PPid" => st.ppid = value.split_whitespace().next().and_then(|s| s.parse().ok()),
            "Uid" => {
                st.uid = value
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .map(Uid);
            }
            "Gid" => st.gid = value.split_whitespace().next().and_then(|s| s.parse().ok()),
            "Threads" => st.threads = value.parse().ok(),
            "VmRSS" => st.vm_rss_kb = first_u64(value),
            "VmSize" => st.vm_size_kb = first_u64(value),
            "Umask" => st.umask = Some(value.to_string()),
            "CapInh" => st.capabilities.cap_inh = value.to_string(),
            "CapPrm" => st.capabilities.cap_prm = value.to_string(),
            "CapEff" => st.capabilities.cap_eff = value.to_string(),
            "CapBnd" => st.capabilities.cap_bnd = value.to_string(),
            "CapAmb" => st.capabilities.cap_amb = value.to_string(),
            _ => {}
        }
    }
    st
}

pub fn parse_io(text: &str) -> IoStats {
    let mut io = IoStats::default();
    for line in text.lines() {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let Ok(n) = v.trim().parse::<u64>() else {
            continue;
        };
        match k.trim() {
            "rchar" => io.read_chars = n,
            "wchar" => io.write_chars = n,
            "syscr" => io.syscr = n,
            "syscw" => io.syscw = n,
            "read_bytes" => io.read_bytes = n,
            "write_bytes" => io.write_bytes = n,
            _ => {}
        }
    }
    io
}

pub fn parse_limits(text: &str) -> Vec<lynx_core::ResourceLimit> {
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        // Name can contain spaces; last two tokens are soft/hard, then units.
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 3 {
            continue;
        }
        let (units, hard, soft) = if looks_like_limit_value(cols[cols.len() - 1]) {
            ("", cols[cols.len() - 1], cols[cols.len() - 2])
        } else if cols.len() >= 4 {
            (
                cols[cols.len() - 1],
                cols[cols.len() - 2],
                cols[cols.len() - 3],
            )
        } else {
            continue;
        };
        let name_end = if units.is_empty() {
            cols.len() - 2
        } else {
            cols.len() - 3
        };
        if name_end == 0 {
            continue;
        }
        out.push(lynx_core::ResourceLimit {
            name: cols[..name_end].join(" "),
            soft: soft.to_string(),
            hard: hard.to_string(),
            units: units.to_string(),
        });
    }
    out
}

fn looks_like_limit_value(s: &str) -> bool {
    s == "unlimited" || s.chars().all(|c| c.is_ascii_digit())
}

fn first_u64(value: &str) -> Option<u64> {
    value.split_whitespace().next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_block() {
        let text = "\
Name:\tnginx
Umask:\t0022
State:\tS (sleeping)
Pid:\t1420
PPid:\t1
Uid:\t33\t33\t33\t33
Threads:\t1
VmRSS:\t  49324 kB
VmSize:\t  140000 kB
CapEff:\t0000000000000000
";
        let s = parse_status(text);
        assert_eq!(s.name.as_deref(), Some("nginx"));
        assert_eq!(s.uid.map(|u| u.0), Some(33));
        assert_eq!(s.vm_rss_kb, Some(49324));
        assert_eq!(s.capabilities.cap_eff, "0000000000000000");
    }

    #[test]
    fn parses_limits_table() {
        let text = "\
Limit                     Soft Limit           Hard Limit           Units
Max cpu time              unlimited            unlimited            seconds
Max open files            1024                 4096                 files
Max processes             127411               127411               processes
";
        let limits = parse_limits(text);
        assert_eq!(limits.len(), 3);
        assert_eq!(limits[1].name, "Max open files");
        assert_eq!(limits[1].soft, "1024");
        assert_eq!(limits[1].hard, "4096");
        assert_eq!(limits[1].units, "files");
    }
}

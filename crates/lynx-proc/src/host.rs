use lynx_core::{
    CpuInfo, CpuSample, HostSnapshot, LoadAvg, LynxError, MemoryInfo, Result, SysPaths,
};

use crate::util::{kb_to_bytes, read_to_string};

pub fn load_host(paths: &SysPaths) -> Result<HostSnapshot> {
    let loadavg = parse_loadavg(&read_to_string(paths.proc.join("loadavg"))?)?;
    let memory = parse_meminfo(&read_to_string(paths.proc.join("meminfo"))?);
    let uptime_secs = parse_uptime(&read_to_string(paths.proc.join("uptime"))?)?;
    let cpu = parse_cpuinfo(
        &read_optional(&paths.proc.join("stat"))?,
        &read_optional(&paths.proc.join("cpuinfo"))?,
    );
    Ok(HostSnapshot {
        hostname: read_optional(&paths.proc.join("sys/kernel/hostname"))?
            .map(|s| s.trim().to_string()),
        kernel: read_optional(&paths.proc.join("version"))?
            .map(|s| s.split_whitespace().take(3).collect::<Vec<_>>().join(" ")),
        os_pretty: parse_os_pretty(&read_optional(&paths.os_release)?.unwrap_or_default()),
        uptime_secs,
        loadavg,
        memory,
        cpu,
    })
}

fn read_optional(path: &std::path::Path) -> Result<Option<String>> {
    match read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(LynxError::NotFound(_)) | Err(LynxError::Permission { .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn parse_loadavg(text: &str) -> Result<LoadAvg> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 5 {
        return Err(LynxError::parse("loadavg: expected 5 fields"));
    }
    let (runnable, total) = parts[3]
        .split_once('/')
        .ok_or_else(|| LynxError::parse("loadavg: missing runnable/total"))?;
    Ok(LoadAvg {
        one: parts[0].parse().unwrap_or(0.0),
        five: parts[1].parse().unwrap_or(0.0),
        fifteen: parts[2].parse().unwrap_or(0.0),
        runnable: runnable.parse().unwrap_or(0),
        total_threads: total.parse().unwrap_or(0),
        last_pid: parts[4].parse().unwrap_or(0),
    })
}

pub fn parse_meminfo(text: &str) -> MemoryInfo {
    let mut map = std::collections::HashMap::new();
    for line in text.lines() {
        if let Some((k, v)) = crate::util::parse_kv_u64(line) {
            map.insert(k, kb_to_bytes(v));
        }
    }
    MemoryInfo {
        total_bytes: *map.get("MemTotal").unwrap_or(&0),
        available_bytes: *map.get("MemAvailable").unwrap_or(&0),
        free_bytes: *map.get("MemFree").unwrap_or(&0),
        buffers_bytes: *map.get("Buffers").unwrap_or(&0),
        cached_bytes: *map.get("Cached").unwrap_or(&0),
        swap_total_bytes: *map.get("SwapTotal").unwrap_or(&0),
        swap_free_bytes: *map.get("SwapFree").unwrap_or(&0),
    }
}

pub fn parse_uptime(text: &str) -> Result<u64> {
    let first = text
        .split_whitespace()
        .next()
        .ok_or_else(|| LynxError::parse("uptime: empty"))?;
    let secs: f64 = first
        .parse()
        .map_err(|_| LynxError::parse("uptime: not a float"))?;
    Ok(secs as u64)
}

pub fn parse_stat_cpu(text: &str) -> Option<CpuSample> {
    let line = text.lines().find(|l| l.starts_with("cpu "))?;
    let mut nums = line.split_whitespace().skip(1);
    let user: u64 = nums.next()?.parse().ok()?;
    let nice: u64 = nums.next()?.parse().ok()?;
    let system: u64 = nums.next()?.parse().ok()?;
    let idle: u64 = nums.next()?.parse().ok()?;
    let iowait: u64 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let irq: u64 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let softirq: u64 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let steal: u64 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let guest: u64 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let guest_nice: u64 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let total = user + nice + system + idle + iowait + irq + softirq + steal + guest + guest_nice;
    Some(CpuSample {
        total_ticks: total,
        idle_ticks: idle + iowait,
    })
}

fn parse_cpuinfo(stat_text: &Option<String>, cpuinfo: &Option<String>) -> CpuInfo {
    let sample = stat_text.as_deref().and_then(parse_stat_cpu);
    let logical = stat_text
        .as_deref()
        .map(|t| {
            t.lines()
                .filter(|l| {
                    l.starts_with("cpu")
                        && l.as_bytes()
                            .get(3)
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false)
                })
                .count() as u32
        })
        .unwrap_or(0);
    let model = cpuinfo.as_deref().and_then(|t| {
        t.lines()
            .find(|l| l.starts_with("model name"))
            .and_then(|l| l.split_once(':'))
            .map(|(_, v)| v.trim().to_string())
    });
    CpuInfo {
        logical_cpus: logical,
        total_ticks: sample.map(|s| s.total_ticks).unwrap_or(0),
        idle_ticks: sample.map(|s| s.idle_ticks).unwrap_or(0),
        model,
    }
}

fn parse_os_pretty(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("PRETTY_NAME=") {
            return Some(rest.trim_matches('"').to_string());
        }
    }
    None
}

pub fn read_cpu_sample(paths: &SysPaths) -> Result<CpuSample> {
    let text = read_to_string(paths.proc.join("stat"))?;
    parse_stat_cpu(&text).ok_or_else(|| LynxError::parse("stat: missing cpu line"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadavg_and_meminfo() {
        let la = parse_loadavg("0.52 0.58 0.59 2/882 31415\n").unwrap();
        assert!((la.one - 0.52).abs() < f32::EPSILON);
        assert_eq!(la.runnable, 2);
        assert_eq!(la.total_threads, 882);

        let mem =
            parse_meminfo("MemTotal: 16384000 kB\nMemAvailable: 8192000 kB\nMemFree: 1024 kB\n");
        assert_eq!(mem.total_bytes, 16384000 * 1024);
        assert_eq!(mem.available_bytes, 8192000 * 1024);
    }

    #[test]
    fn cpu_sample_delta() {
        let a = parse_stat_cpu("cpu  100 0 50 850 0 0 0 0 0 0\ncpu0 100 0 50 850 0 0 0 0 0 0\n")
            .unwrap();
        let b = parse_stat_cpu("cpu  150 0 70 880 0 0 0 0 0 0\n").unwrap();
        let pct = a.usage_percent(b).unwrap();
        // user+50 system+20 idle+30 → busy 70 / total 100
        assert!((pct - 70.0).abs() < 0.01);
    }
}

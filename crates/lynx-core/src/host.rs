use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostSnapshot {
    pub hostname: Option<String>,
    pub kernel: Option<String>,
    pub os_pretty: Option<String>,
    pub uptime_secs: u64,
    pub loadavg: LoadAvg,
    pub memory: MemoryInfo,
    pub cpu: CpuInfo,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct LoadAvg {
    pub one: f32,
    pub five: f32,
    pub fifteen: f32,
    pub runnable: u32,
    pub total_threads: u32,
    pub last_pid: u32,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub free_bytes: u64,
    pub buffers_bytes: u64,
    pub cached_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_free_bytes: u64,
}

impl MemoryInfo {
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }

    pub fn used_percent(&self) -> Option<f32> {
        if self.total_bytes == 0 {
            None
        } else {
            Some((self.used_bytes() as f32 / self.total_bytes as f32) * 100.0)
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuInfo {
    pub logical_cpus: u32,
    /// Cumulative ticks from `/proc/stat` `cpu` line (user+nice+system+idle+...).
    pub total_ticks: u64,
    pub idle_ticks: u64,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CpuSample {
    pub total_ticks: u64,
    pub idle_ticks: u64,
}

impl CpuSample {
    pub fn usage_percent(self, next: CpuSample) -> Option<f32> {
        let dt = next.total_ticks.saturating_sub(self.total_ticks);
        if dt == 0 {
            return None;
        }
        let di = next.idle_ticks.saturating_sub(self.idle_ticks);
        let busy = dt.saturating_sub(di) as f32;
        Some((busy / dt as f32) * 100.0)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProcCpuSample {
    pub utime: u64,
    pub stime: u64,
}

impl ProcCpuSample {
    pub fn ticks(self) -> u64 {
        self.utime.saturating_add(self.stime)
    }

    /// Process CPU% relative to wall-clock CPU ticks across all logical CPUs.
    pub fn usage_percent(
        self,
        next: Self,
        system_delta_ticks: u64,
        logical_cpus: u32,
    ) -> Option<f32> {
        if system_delta_ticks == 0 {
            return None;
        }
        let dt = next.ticks().saturating_sub(self.ticks()) as f32;
        let cpus = logical_cpus.max(1) as f32;
        Some((dt / system_delta_ticks as f32) * cpus * 100.0)
    }
}

//! Userspace side of LynxOps eBPF.
//!
//! The first release ships the event types and a capability probe. Actual
//! CO-RE programs (exec, connect) will load through Aya behind the `probes`
//! feature — see `docs/EBPF.md`.

use lynx_core::{LynxError, Result};
use lynx_ebpf_common::{ConnectEvent, EventKind, ExecEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeStatus {
    Available,
    MissingPrivilege,
    MissingBtf,
    UnsupportedKernel,
    NotLinux,
    NotBuilt,
}

impl std::fmt::Display for ProbeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Available => write!(f, "available"),
            Self::MissingPrivilege => {
                write!(f, "missing privilege (need CAP_BPF / CAP_PERFMON or root)")
            }
            Self::MissingBtf => write!(f, "kernel BTF not found at /sys/kernel/btf/vmlinux"),
            Self::UnsupportedKernel => write!(f, "kernel too old for CO-RE probes"),
            Self::NotLinux => write!(f, "not Linux — eBPF probes require a Linux kernel"),
            Self::NotBuilt => write!(f, "lynx-ebpf built without the `probes` feature"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EbpfCapability {
    pub status: ProbeStatus,
    pub btf: bool,
    pub bpf_fs: bool,
    pub kernel_release: Option<String>,
    pub notes: Vec<String>,
}

/// Host checks only — does not load a program.
pub fn probe_capability() -> EbpfCapability {
    let btf = std::path::Path::new("/sys/kernel/btf/vmlinux").exists();
    let bpf_fs = std::path::Path::new("/sys/fs/bpf").exists();
    let kernel_release = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .ok()
        .map(|s| s.trim().to_string());

    let mut notes = Vec::new();
    if !cfg!(feature = "probes") {
        notes.push("probe programs are not compiled into this build; see docs/EBPF.md".into());
    }
    if !btf {
        notes.push("install a BTF-enabled kernel (CONFIG_DEBUG_INFO_BTF=y)".into());
    }

    let status = if !cfg!(target_os = "linux") {
        ProbeStatus::NotLinux
    } else if !cfg!(feature = "probes") {
        ProbeStatus::NotBuilt
    } else if !btf {
        ProbeStatus::MissingBtf
    } else {
        ProbeStatus::Available
    };

    EbpfCapability {
        status,
        btf,
        bpf_fs,
        kernel_release,
        notes,
    }
}

/// Placeholder loader. Returns a clear error until Aya programs land.
pub fn attach_default_probes() -> Result<ProbeSession> {
    let cap = probe_capability();
    Err(LynxError::Unsupported(format!(
        "eBPF probes are the advanced layer and are not loaded in 0.1 ({})",
        cap.status
    )))
}

#[derive(Debug)]
pub struct ProbeSession {
    _private: (),
}

impl ProbeSession {
    pub fn poll(&mut self) -> Result<Vec<UserspaceEvent>> {
        Ok(Vec::new())
    }
}

#[derive(Debug, Clone)]
pub enum UserspaceEvent {
    Exec(ExecEvent),
    Connect(ConnectEvent),
}

impl UserspaceEvent {
    pub fn kind(&self) -> EventKind {
        match self {
            Self::Exec(_) => EventKind::Exec,
            Self::Connect(_) => EventKind::Connect,
        }
    }
}

//! Shared domain types for LynxOps collectors, CLI, TUI, and snapshots.
//!
//! This crate has no host I/O. Parsers live in the collector crates so they can
//! be tested against fixtures without talking to a live kernel.

pub mod cgroup;
pub mod error;
pub mod format;
pub mod host;
pub mod ids;
pub mod net;
pub mod paths;
pub mod process;
pub mod redact;
pub mod snapshot;
pub mod systemd;

pub use cgroup::{CgroupDetail, CgroupRef, CgroupSummary, CgroupVersion, Pressure};
pub use error::{LynxError, Result};
pub use host::{CpuInfo, CpuSample, HostSnapshot, LoadAvg, MemoryInfo, ProcCpuSample};
pub use ids::{Inode, Pid, Uid};
pub use net::{InterfaceCounters, Protocol, Socket, SocketState};
pub use paths::SysPaths;
pub use process::{
    sort_processes, unit_from_cgroup_path, Capabilities, Fd, IoStats, Namespaces, ProcessDetail,
    ProcessSort, ProcessState, ProcessSummary, ResourceLimit, ThreadInfo,
};
pub use snapshot::{IncidentSnapshot, SNAPSHOT_VERSION};
pub use systemd::{JournalEntry, SystemdUnit};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

//! procfs collectors. All reads go through [`lynx_core::SysPaths`] so the same
//! code path can replay a captured tree in tests.

mod collect;
mod host;
mod stat;
mod status;
mod users;
mod util;

pub use collect::{parse_cgroup_file, parse_socket_inode, start_age_secs, ProcCollector};
pub use host::{
    load_host, parse_loadavg, parse_meminfo, parse_stat_cpu, parse_uptime, read_cpu_sample,
};
pub use stat::{parse_stat_line, PidStat};
pub use status::{parse_io, parse_limits, parse_status};
pub use users::{load_users, parse_passwd};
pub use util::{page_size, ticks_per_second};

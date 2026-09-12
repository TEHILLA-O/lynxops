use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemdUnit {
    pub name: String,
    pub description: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub unit_file_state: Option<String>,
    pub fragment_path: Option<String>,
    pub main_pid: Option<u32>,
    pub cgroup: Option<String>,
    pub n_restarts: Option<u32>,
    pub result: Option<String>,
    pub invocation_id: Option<String>,
}

impl SystemdUnit {
    pub fn is_failed(&self) -> bool {
        self.active_state.eq_ignore_ascii_case("failed")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub realtime: Option<String>,
    pub priority: Option<u8>,
    pub syslog_identifier: Option<String>,
    pub unit: Option<String>,
    pub pid: Option<u32>,
    pub message: String,
    pub cursor: Option<String>,
}

impl JournalEntry {
    pub fn priority_label(&self) -> &'static str {
        match self.priority.unwrap_or(6) {
            0 => "emerg",
            1 => "alert",
            2 => "crit",
            3 => "err",
            4 => "warning",
            5 => "notice",
            6 => "info",
            _ => "debug",
        }
    }
}

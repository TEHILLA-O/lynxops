//! systemd / journal collectors.
//!
//! Live reads go through `systemctl` and `journalctl` so we do not require
//! libsystemd or D-Bus at compile time. Parsers are fixture-testable.

mod parse;

use std::process::Command;

use lynx_core::{JournalEntry, LynxError, Result, SystemdUnit};

pub use parse::{parse_journal_json_line, parse_list_units_plain, parse_systemctl_show};

#[derive(Debug, Clone, Default)]
pub struct SystemdCollector {
    pub systemctl: String,
    pub journalctl: String,
}

impl SystemdCollector {
    pub fn new() -> Self {
        Self {
            systemctl: "systemctl".into(),
            journalctl: "journalctl".into(),
        }
    }

    pub fn available(&self) -> bool {
        which(&self.systemctl)
    }

    pub fn journal_available(&self) -> bool {
        which(&self.journalctl)
    }

    pub fn list_units(&self) -> Result<Vec<SystemdUnit>> {
        let text = run(
            &self.systemctl,
            &[
                "list-units",
                "--all",
                "--plain",
                "--no-legend",
                "--no-pager",
            ],
        )?;
        Ok(parse_list_units_plain(&text))
    }

    pub fn failed_units(&self) -> Result<Vec<SystemdUnit>> {
        let text = run(
            &self.systemctl,
            &[
                "list-units",
                "--failed",
                "--plain",
                "--no-legend",
                "--no-pager",
            ],
        )?;
        Ok(parse_list_units_plain(&text))
    }

    pub fn inspect(&self, unit: &str) -> Result<SystemdUnit> {
        let text = run(
            &self.systemctl,
            &["show", "--all", "--no-pager", "--", unit],
        )?;
        let parsed = parse_systemctl_show(&text);
        if parsed.name.is_empty() && parsed.load_state.is_empty() {
            return Err(LynxError::not_found(format!("unit {unit}")));
        }
        Ok(parsed)
    }

    pub fn journal(&self, opts: JournalQuery) -> Result<Vec<JournalEntry>> {
        let mut args = vec![
            "-o".into(),
            "json".into(),
            "-n".into(),
            opts.lines.to_string(),
            "--no-pager".into(),
        ];
        if let Some(prio) = opts.priority {
            args.push("-p".into());
            args.push(prio);
        }
        if let Some(unit) = opts.unit {
            args.push("-u".into());
            args.push(unit);
        }
        if opts.boot {
            args.push("-b".into());
        }
        let text = run(
            &self.journalctl,
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
        )?;
        Ok(text.lines().filter_map(parse_journal_json_line).collect())
    }

    pub fn journal_errors(&self, lines: u32) -> Result<Vec<JournalEntry>> {
        self.journal(JournalQuery {
            lines,
            priority: Some("err".into()),
            unit: None,
            boot: true,
        })
    }
}

#[derive(Debug, Clone)]
pub struct JournalQuery {
    pub lines: u32,
    pub priority: Option<String>,
    pub unit: Option<String>,
    pub boot: bool,
}

impl Default for JournalQuery {
    fn default() -> Self {
        Self {
            lines: 50,
            priority: None,
            unit: None,
            boot: true,
        }
    }
}

fn run(cmd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| LynxError::command(cmd, e.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LynxError::command(
            format!("{cmd} {}", args.join(" ")),
            stderr.trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn which(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

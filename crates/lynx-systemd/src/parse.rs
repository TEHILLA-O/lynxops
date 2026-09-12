use lynx_core::{JournalEntry, SystemdUnit};

/// Parse `systemctl show --all` / `systemctl show UNIT` key=value output.
pub fn parse_systemctl_show(text: &str) -> SystemdUnit {
    let mut map = std::collections::HashMap::new();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.to_string(), v.to_string());
        }
    }
    let get = |k: &str| map.get(k).cloned().unwrap_or_default();
    let opt = |k: &str| {
        map.get(k)
            .cloned()
            .filter(|s| !s.is_empty() && s != "n/a" && s != "[not set]")
    };
    SystemdUnit {
        name: get("Id"),
        description: get("Description"),
        load_state: get("LoadState"),
        active_state: get("ActiveState"),
        sub_state: get("SubState"),
        unit_file_state: opt("UnitFileState"),
        fragment_path: opt("FragmentPath"),
        main_pid: opt("MainPID")
            .and_then(|s| s.parse().ok())
            .filter(|p| *p > 0),
        cgroup: opt("ControlGroup"),
        n_restarts: opt("NRestarts").and_then(|s| s.parse().ok()),
        result: opt("Result"),
        invocation_id: opt("InvocationID"),
    }
}

/// Parse `systemctl list-units --plain --no-legend --all` (classic columns).
pub fn parse_list_units_plain(text: &str) -> Vec<SystemdUnit> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("UNIT ") || line.starts_with("●") {
            continue;
        }
        let line = line.trim_start_matches(['●', '○', '×', '*', ' ']);
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 {
            continue;
        }
        // UNIT LOAD ACTIVE SUB DESCRIPTION...
        let description = if cols.len() > 4 {
            cols[4..].join(" ")
        } else {
            String::new()
        };
        out.push(SystemdUnit {
            name: cols[0].to_string(),
            description,
            load_state: cols[1].to_string(),
            active_state: cols[2].to_string(),
            sub_state: cols[3].to_string(),
            unit_file_state: None,
            fragment_path: None,
            main_pid: None,
            cgroup: None,
            n_restarts: None,
            result: None,
            invocation_id: None,
        });
    }
    out
}

/// Parse one `journalctl -o json` line.
pub fn parse_journal_json_line(line: &str) -> Option<JournalEntry> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let message = journal_string(&v, &["MESSAGE"]).unwrap_or_default();
    if message.is_empty() {
        return None;
    }
    let priority = journal_string(&v, &["PRIORITY"]).and_then(|s| s.parse().ok());
    let realtime = journal_string(&v, &["__REALTIME_TIMESTAMP", "SYSLOG_TIMESTAMP"]);
    Some(JournalEntry {
        realtime: realtime.map(format_usec_or_raw),
        priority,
        syslog_identifier: journal_string(&v, &["SYSLOG_IDENTIFIER", "_COMM"]),
        unit: journal_string(&v, &["_SYSTEMD_UNIT", "UNIT", "SYSLOG_IDENTIFIER"]),
        pid: journal_string(&v, &["_PID"]).and_then(|s| s.parse().ok()),
        message,
        cursor: journal_string(&v, &["__CURSOR"]),
    })
}

fn journal_string(v: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        match v.get(*key) {
            Some(serde_json::Value::String(s)) => return Some(s.clone()),
            Some(serde_json::Value::Number(n)) => return Some(n.to_string()),
            Some(serde_json::Value::Array(arr)) => {
                // journalctl may emit byte arrays for binary fields.
                if let Some(bytes) = arr
                    .iter()
                    .map(|x| x.as_u64().map(|n| n as u8))
                    .collect::<Option<Vec<u8>>>()
                {
                    return Some(String::from_utf8_lossy(&bytes).into_owned());
                }
            }
            _ => {}
        }
    }
    None
}

fn format_usec_or_raw(s: String) -> String {
    if let Ok(usec) = s.parse::<u128>() {
        let secs = (usec / 1_000_000) as i64;
        let nsec = ((usec % 1_000_000) * 1000) as u32;
        if let Some(dt) = chrono_from_unix(secs, nsec) {
            return dt;
        }
    }
    s
}

fn chrono_from_unix(secs: i64, nsec: u32) -> Option<String> {
    use std::time::{Duration, UNIX_EPOCH};
    let t = UNIX_EPOCH.checked_add(Duration::new(secs as u64, nsec))?;
    let dt: chrono::DateTime<chrono::Utc> = t.into();
    Some(dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_unit() {
        let text = "\
Id=nginx.service
Description=A high performance web server
LoadState=loaded
ActiveState=active
SubState=running
MainPID=1420
ControlGroup=/system.slice/nginx.service
NRestarts=0
Result=success
";
        let u = parse_systemctl_show(text);
        assert_eq!(u.name, "nginx.service");
        assert_eq!(u.main_pid, Some(1420));
        assert!(!u.is_failed());
    }

    #[test]
    fn list_units_plain() {
        let text = "nginx.service loaded active running A high performance web server\nsshd.service  loaded failed  failed  OpenSSH server daemon\n";
        let units = parse_list_units_plain(text);
        assert_eq!(units.len(), 2);
        assert!(units[1].is_failed());
    }

    #[test]
    fn journal_json() {
        let line = r#"{"MESSAGE":"nginx started","PRIORITY":"6","SYSLOG_IDENTIFIER":"nginx","_PID":"1420","_SYSTEMD_UNIT":"nginx.service"}"#;
        let e = parse_journal_json_line(line).unwrap();
        assert_eq!(e.message, "nginx started");
        assert_eq!(e.priority, Some(6));
        assert_eq!(e.unit.as_deref(), Some("nginx.service"));
    }
}

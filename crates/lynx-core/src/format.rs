/// Human-readable byte count using binary units (KiB, MiB, GiB).
pub fn bytes(n: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} {}", UNITS[unit])
    } else if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else if value >= 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

/// Format a percentage, omitting the value when sampling has not completed.
pub fn percent(value: Option<f32>) -> String {
    match value {
        Some(v) => format!("{v:5.1}%"),
        None => "    -".to_string(),
    }
}

/// Compact duration from seconds (uptime, start time).
pub fn duration_secs(secs: u64) -> String {
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let mins = (secs % 3_600) / 60;
    let s = secs % 60;
    if days > 0 {
        format!("{days}d {hours:02}h")
    } else if hours > 0 {
        format!("{hours}h {mins:02}m")
    } else if mins > 0 {
        format!("{mins}m {s:02}s")
    } else {
        format!("{s}s")
    }
}

/// Right-pad / truncate a string to `width` columns (ASCII-oriented).
pub fn cell(text: &str, width: usize) -> String {
    if text.chars().count() > width {
        let mut out: String = text.chars().take(width.saturating_sub(1)).collect();
        out.push('…');
        out
    } else {
        format!("{text:<width$}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_scales() {
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(2048), "2.00 KiB");
        assert_eq!(bytes(5 * 1024 * 1024), "5.00 MiB");
    }

    #[test]
    fn duration_compacts() {
        assert_eq!(duration_secs(9), "9s");
        assert_eq!(duration_secs(125), "2m 05s");
        assert_eq!(duration_secs(3700), "1h 01m");
    }
}

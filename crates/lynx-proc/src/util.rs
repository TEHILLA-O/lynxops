use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

use lynx_core::{LynxError, Result};

pub fn read_to_string(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(|e| map_io(e, path))
}

pub fn read_to_vec(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let path = path.as_ref();
    fs::read(path).map_err(|e| map_io(e, path))
}

pub fn read_link(path: impl AsRef<Path>) -> Result<std::path::PathBuf> {
    let path = path.as_ref();
    fs::read_link(path).map_err(|e| map_io(e, path))
}

pub fn map_io(err: io::Error, path: &Path) -> LynxError {
    match err.kind() {
        io::ErrorKind::NotFound => LynxError::not_found(path.display().to_string()),
        io::ErrorKind::PermissionDenied => {
            LynxError::permission(err.to_string(), path.to_path_buf())
        }
        _ => LynxError::io(err, path.to_path_buf()),
    }
}

/// `/proc/<pid>/environ` is NUL-separated `KEY=VALUE` pairs.
pub fn parse_environ(bytes: &[u8]) -> Vec<(String, String)> {
    bytes
        .split(|b| *b == 0)
        .filter(|chunk| !chunk.is_empty())
        .filter_map(|chunk| {
            let text = String::from_utf8_lossy(chunk);
            let (k, v) = text.split_once('=')?;
            Some((k.to_string(), v.to_string()))
        })
        .collect()
}

/// `/proc/<pid>/cmdline` is NUL-separated argv.
pub fn parse_cmdline(bytes: &[u8]) -> Vec<String> {
    let parts: Vec<String> = bytes
        .split(|b| *b == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect();
    if parts.is_empty() && !bytes.is_empty() {
        // Some kernel threads use spaces instead of NULs.
        return String::from_utf8_lossy(bytes)
            .split_whitespace()
            .map(str::to_string)
            .collect();
    }
    parts
}

pub fn parse_kv_u64(line: &str) -> Option<(String, u64)> {
    let mut parts = line.split_whitespace();
    let key = parts.next()?.trim_end_matches(':').to_string();
    let value = parts.next()?.parse().ok()?;
    Some((key, value))
}

pub fn kb_to_bytes(kb: u64) -> u64 {
    kb.saturating_mul(1024)
}

pub fn page_size() -> u64 {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: sysconf(_SC_PAGESIZE) is always safe to call.
        let n = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if n > 0 {
            n as u64
        } else {
            4096
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        4096
    }
}

pub fn ticks_per_second() -> u64 {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: sysconf(_SC_CLK_TCK) is always safe to call.
        let n = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        if n > 0 {
            n as u64
        } else {
            100
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        100
    }
}

pub fn sleep_sample(interval: Duration) {
    if !interval.is_zero() {
        std::thread::sleep(interval);
    }
}

/// Best-effort read that treats vanished processes as absence.
pub fn try_read_to_string(path: impl AsRef<Path>) -> Result<Option<String>> {
    match read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(LynxError::NotFound(_)) => Ok(None),
        Err(LynxError::Permission { .. }) => Ok(None),
        Err(other) => {
            if let LynxError::Io { source, .. } = &other {
                if source.kind() == io::ErrorKind::NotFound {
                    return Ok(None);
                }
            }
            Err(other)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environ_splits_nuls() {
        let raw = b"HOME=/root\0PATH=/bin\0";
        let env = parse_environ(raw);
        assert_eq!(
            env,
            vec![
                ("HOME".into(), "/root".into()),
                ("PATH".into(), "/bin".into())
            ]
        );
    }

    #[test]
    fn cmdline_splits_nuls() {
        assert_eq!(
            parse_cmdline(b"nginx\0-g\0daemon off;\0"),
            vec!["nginx", "-g", "daemon off;"]
        );
    }
}

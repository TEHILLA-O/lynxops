use std::io;
use std::path::PathBuf;

/// Workspace-wide error type. Collectors map I/O and parse failures here so the
/// CLI can print one consistent shape.
#[derive(Debug, thiserror::Error)]
pub enum LynxError {
    #[error("I/O error{path}: {source}", path = .path.as_ref().map(|p| format!(" ({})", p.display())).unwrap_or_default())]
    Io {
        #[source]
        source: io::Error,
        path: Option<PathBuf>,
    },

    #[error("parse error{path}: {message}", path = .path.as_ref().map(|p| format!(" ({})", p.display())).unwrap_or_default())]
    Parse {
        message: String,
        path: Option<PathBuf>,
    },

    #[error("not found: {0}")]
    NotFound(String),

    #[error("permission denied{path}: {message}", path = .path.as_ref().map(|p| format!(" ({})", p.display())).unwrap_or_default())]
    Permission {
        message: String,
        path: Option<PathBuf>,
    },

    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("command `{cmd}` failed: {message}")]
    Command { cmd: String, message: String },

    #[error("{0}")]
    Other(String),
}

impl LynxError {
    pub fn io(source: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::Io {
            source,
            path: Some(path.into()),
        }
    }

    pub fn parse(message: impl Into<String>) -> Self {
        Self::Parse {
            message: message.into(),
            path: None,
        }
    }

    pub fn parse_at(message: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::Parse {
            message: message.into(),
            path: Some(path.into()),
        }
    }

    pub fn not_found(what: impl Into<String>) -> Self {
        Self::NotFound(what.into())
    }

    pub fn permission(message: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::Permission {
            message: message.into(),
            path: Some(path.into()),
        }
    }

    pub fn command(cmd: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Command {
            cmd: cmd.into(),
            message: message.into(),
        }
    }
}

impl From<io::Error> for LynxError {
    fn from(source: io::Error) -> Self {
        Self::Io { source, path: None }
    }
}

pub type Result<T> = std::result::Result<T, LynxError>;

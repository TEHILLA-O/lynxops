use std::path::{Path, PathBuf};

/// Injectable host filesystem roots. Production uses `/proc` and
/// `/sys/fs/cgroup`; tests and offline triage pass a captured sysroot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysPaths {
    pub proc: PathBuf,
    pub cgroup: PathBuf,
    pub passwd: PathBuf,
    pub os_release: PathBuf,
}

impl Default for SysPaths {
    fn default() -> Self {
        Self::live()
    }
}

impl SysPaths {
    pub fn live() -> Self {
        Self {
            proc: PathBuf::from("/proc"),
            cgroup: PathBuf::from("/sys/fs/cgroup"),
            passwd: PathBuf::from("/etc/passwd"),
            os_release: PathBuf::from("/etc/os-release"),
        }
    }

    /// Treat `root` as the host filesystem root (`root/proc`, `root/sys/fs/cgroup`).
    pub fn from_sysroot(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            proc: root.join("proc"),
            cgroup: root.join("sys/fs/cgroup"),
            passwd: root.join("etc/passwd"),
            os_release: root.join("etc/os-release"),
        }
    }

    pub fn pid_dir(&self, pid: crate::ids::Pid) -> PathBuf {
        self.proc.join(pid.0.to_string())
    }
}

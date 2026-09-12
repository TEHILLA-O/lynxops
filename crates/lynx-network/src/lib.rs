//! Socket tables from `/proc/net/*`, joined to processes via fd inodes.

mod parse;

use std::collections::HashMap;
use std::fs;

use lynx_core::{InterfaceCounters, Pid, Protocol, Result, Socket, SocketState, SysPaths};
use lynx_proc::{parse_socket_inode, ProcCollector};

pub use parse::{parse_dev, parse_hex_addr, parse_inet_line, parse_inet_table, parse_unix_table};

#[derive(Debug, Clone)]
pub struct NetCollector {
    pub paths: SysPaths,
}

impl Default for NetCollector {
    fn default() -> Self {
        Self::new(SysPaths::live())
    }
}

impl NetCollector {
    pub fn new(paths: SysPaths) -> Self {
        Self { paths }
    }

    pub fn sockets(&self) -> Result<Vec<Socket>> {
        let mut all = Vec::new();
        all.extend(self.read_inet("tcp", Protocol::Tcp)?);
        all.extend(self.read_inet("tcp6", Protocol::Tcp6)?);
        all.extend(self.read_inet("udp", Protocol::Udp)?);
        all.extend(self.read_inet("udp6", Protocol::Udp6)?);
        if let Ok(text) = fs::read_to_string(self.paths.proc.join("net/unix")) {
            all.extend(parse_unix_table(&text)?);
        }
        self.attach_owners(&mut all);
        Ok(all)
    }

    pub fn listening(&self) -> Result<Vec<Socket>> {
        let mut socks = self.sockets()?;
        socks.retain(|s| {
            s.is_listen()
                || (s.protocol == Protocol::Udp && s.remote.map(|r| r.port() == 0).unwrap_or(false))
        });
        Ok(socks)
    }

    pub fn established(&self) -> Result<Vec<Socket>> {
        let mut socks = self.sockets()?;
        socks.retain(|s| s.state == SocketState::Established);
        Ok(socks)
    }

    pub fn for_pid(&self, pid: Pid) -> Result<Vec<Socket>> {
        let mut socks = self.sockets()?;
        socks.retain(|s| s.pid == Some(pid));
        Ok(socks)
    }

    pub fn interfaces(&self) -> Result<Vec<InterfaceCounters>> {
        let text = fs::read_to_string(self.paths.proc.join("net/dev"))
            .map_err(|e| lynx_core::LynxError::io(e, self.paths.proc.join("net/dev")))?;
        Ok(parse_dev(&text))
    }

    fn read_inet(&self, name: &str, proto: Protocol) -> Result<Vec<Socket>> {
        let path = self.paths.proc.join("net").join(name);
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(lynx_core::LynxError::io(e, path)),
        };
        parse_inet_table(&text, proto)
    }

    fn attach_owners(&self, sockets: &mut [Socket]) {
        let index = inode_to_pid(&self.paths);
        let proc = ProcCollector::new(self.paths.clone());
        let names: HashMap<Pid, String> = proc
            .list_once()
            .unwrap_or_default()
            .into_iter()
            .map(|p| (p.pid, p.comm))
            .collect();
        for sock in sockets.iter_mut() {
            if let Some(pid) = index.get(&sock.inode.0).copied() {
                sock.pid = Some(pid);
                sock.comm = names.get(&pid).cloned();
            }
        }
    }
}

/// Walk `/proc/<pid>/fd` and map `socket:[inode]` → pid (first wins).
pub fn inode_to_pid(paths: &SysPaths) -> HashMap<u64, Pid> {
    let mut map = HashMap::new();
    let Ok(rd) = fs::read_dir(&paths.proc) else {
        return map;
    };
    for ent in rd.flatten() {
        let Some(pid) = ent
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<u32>().ok())
            .map(Pid)
        else {
            continue;
        };
        let fd_dir = ent.path().join("fd");
        let Ok(fds) = fs::read_dir(fd_dir) else {
            continue;
        };
        for fd in fds.flatten() {
            let Ok(target) = fs::read_link(fd.path()) else {
                continue;
            };
            if let Some(inode) = parse_socket_inode(&target.to_string_lossy()) {
                map.entry(inode.0).or_insert(pid);
            }
        }
    }
    map
}

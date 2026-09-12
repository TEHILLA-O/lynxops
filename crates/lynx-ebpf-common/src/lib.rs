//! Shared event layout for future CO-RE probes and the userspace consumer.
//!
//! These structs are `repr(C)` so an Aya (or libbpf) program can write them
//! into a ring buffer that LynxOps reads. No probe bytecode lives here.

#![no_std]

/// Event kinds the first eBPF layer will emit.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Exec = 1,
    Exit = 2,
    Connect = 3,
    Bind = 4,
    Open = 5,
}

/// Fixed-size exec event. `comm` matches `TASK_COMM_LEN`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExecEvent {
    pub kind: u8,
    pub _pad: [u8; 3],
    pub pid: u32,
    pub tgid: u32,
    pub uid: u32,
    pub comm: [u8; 16],
    pub filename: [u8; 64],
}

/// TCP connect/bind event. Addresses are stored as raw bytes plus family.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ConnectEvent {
    pub kind: u8,
    pub family: u8,
    pub _pad: [u8; 2],
    pub pid: u32,
    pub tgid: u32,
    pub uid: u32,
    pub comm: [u8; 16],
    pub saddr: [u8; 16],
    pub daddr: [u8; 16],
    pub sport: u16,
    pub dport: u16,
}

impl ExecEvent {
    pub fn comm_str(&self) -> &str {
        cstr(&self.comm)
    }

    pub fn filename_str(&self) -> &str {
        cstr(&self.filename)
    }
}

fn cstr(bytes: &[u8]) -> &str {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    core::str::from_utf8(&bytes[..end]).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_stable() {
        assert_eq!(core::mem::size_of::<ExecEvent>(), 96);
        assert_eq!(core::mem::size_of::<ConnectEvent>(), 68);
    }
}

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use lynx_core::{Inode, InterfaceCounters, LynxError, Protocol, Result, Socket, SocketState, Uid};

/// Parse `/proc/net/{tcp,tcp6,udp,udp6}` (skip header).
pub fn parse_inet_table(text: &str, protocol: Protocol) -> Result<Vec<Socket>> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        out.push(parse_inet_line(line, protocol)?);
    }
    Ok(out)
}

pub fn parse_inet_line(line: &str, protocol: Protocol) -> Result<Socket> {
    let cols: Vec<&str> = line.split_whitespace().collect();
    // sl local rem st tx:rx tr:tm retrnsmt uid timeout inode
    if cols.len() < 10 {
        return Err(LynxError::parse(format!(
            "net table: expected ≥10 columns, got {}",
            cols.len()
        )));
    }
    let local = parse_hex_addr(cols[1], protocol)?;
    let remote = parse_hex_addr(cols[2], protocol)?;
    let state_code = u8::from_str_radix(cols[3], 16)
        .map_err(|_| LynxError::parse(format!("net table: bad state {}", cols[3])))?;
    let uid: u32 = cols[7]
        .parse()
        .map_err(|_| LynxError::parse("net table: bad uid"))?;
    let inode: u64 = cols[9]
        .parse()
        .map_err(|_| LynxError::parse("net table: bad inode"))?;
    Ok(Socket {
        protocol,
        local: Some(local),
        remote: Some(remote),
        state: SocketState::from_hex(state_code),
        inode: Inode(inode),
        uid: Uid(uid),
        pid: None,
        comm: None,
        unix_path: None,
    })
}

pub fn parse_hex_addr(field: &str, protocol: Protocol) -> Result<SocketAddr> {
    let (addr, port) = field
        .split_once(':')
        .ok_or_else(|| LynxError::parse(format!("addr: missing colon in {field}")))?;
    let port = u16::from_str_radix(port, 16)
        .map_err(|_| LynxError::parse(format!("addr: bad port {port}")))?;
    let ip = match protocol {
        Protocol::Tcp | Protocol::Udp => IpAddr::V4(parse_ipv4_le(addr)?),
        Protocol::Tcp6 | Protocol::Udp6 => IpAddr::V6(parse_ipv6_le(addr)?),
        Protocol::Unix => {
            return Err(LynxError::parse("addr: unix sockets have no inet address"));
        }
    };
    Ok(SocketAddr::new(ip, port))
}

fn parse_ipv4_le(hex: &str) -> Result<Ipv4Addr> {
    if hex.len() != 8 {
        return Err(LynxError::parse(format!(
            "ipv4: expected 8 hex chars, got {hex}"
        )));
    }
    let n = u32::from_str_radix(hex, 16)
        .map_err(|_| LynxError::parse(format!("ipv4: bad hex {hex}")))?;
    // /proc/net/tcp stores IPv4 as little-endian hex on little-endian hosts.
    Ok(Ipv4Addr::from(n.to_le_bytes()))
}

fn parse_ipv6_le(hex: &str) -> Result<Ipv6Addr> {
    if hex.len() != 32 {
        return Err(LynxError::parse(format!(
            "ipv6: expected 32 hex chars, got {}",
            hex.len()
        )));
    }
    let mut bytes = [0u8; 16];
    // Four little-endian 32-bit words.
    for (i, chunk) in hex.as_bytes().chunks(8).enumerate() {
        let word = std::str::from_utf8(chunk).unwrap_or("00000000");
        let n = u32::from_str_radix(word, 16)
            .map_err(|_| LynxError::parse(format!("ipv6: bad word {word}")))?;
        bytes[i * 4..i * 4 + 4].copy_from_slice(&n.to_le_bytes());
    }
    Ok(Ipv6Addr::from(bytes))
}

pub fn parse_unix_table(text: &str) -> Result<Vec<Socket>> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split_whitespace().collect();
        // Num RefCount Protocol Flags Type St Inode [Path]
        if cols.len() < 7 {
            continue;
        }
        let inode: u64 = match cols[6].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let path = if cols.len() > 7 {
            Some(cols[7..].join(" "))
        } else {
            None
        };
        out.push(Socket {
            protocol: Protocol::Unix,
            local: None,
            remote: None,
            state: SocketState::Unknown(0),
            inode: Inode(inode),
            uid: Uid(0),
            pid: None,
            comm: None,
            unix_path: path,
        });
    }
    Ok(out)
}

pub fn parse_dev(text: &str) -> Vec<InterfaceCounters> {
    let mut out = Vec::new();
    for line in text.lines().skip(2) {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let cols: Vec<&str> = rest.split_whitespace().collect();
        if cols.len() < 16 {
            continue;
        }
        out.push(InterfaceCounters {
            name: name.trim().to_string(),
            rx_bytes: cols[0].parse().unwrap_or(0),
            rx_packets: cols[1].parse().unwrap_or(0),
            rx_errs: cols[2].parse().unwrap_or(0),
            tx_bytes: cols[8].parse().unwrap_or(0),
            tx_packets: cols[9].parse().unwrap_or(0),
            tx_errs: cols[10].parse().unwrap_or(0),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcp_listen_localhost() {
        // 0100007F:0050 = 127.0.0.1:80, state 0A = LISTEN
        let line = "  0: 0100007F:0050 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 33981 1 00000000cafebabe 100 0 0 10 0";
        let s = parse_inet_line(line, Protocol::Tcp).unwrap();
        assert_eq!(s.state, SocketState::Listen);
        assert_eq!(s.local.unwrap().to_string(), "127.0.0.1:80");
        assert_eq!(s.inode.0, 33981);
    }

    #[test]
    fn ipv4_public_addr() {
        // 0A00020A:01BB = 10.2.0.10:443  (0A 00 02 0A little-endian)
        let addr = parse_hex_addr("0A00020A:01BB", Protocol::Tcp).unwrap();
        assert_eq!(addr.to_string(), "10.2.0.10:443");
    }
}

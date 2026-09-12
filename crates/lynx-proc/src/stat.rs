use lynx_core::{LynxError, Pid, ProcessState, Result};

/// Parsed `/proc/<pid>/stat`. Field numbers follow `proc_pid_stat(5)`.
#[derive(Debug, Clone)]
pub struct PidStat {
    pub pid: Pid,
    pub comm: String,
    pub state: ProcessState,
    pub ppid: Pid,
    pub pgrp: i64,
    pub session: i64,
    pub tty_nr: i64,
    pub flags: u64,
    pub minflt: u64,
    pub majflt: u64,
    pub utime: u64,
    pub stime: u64,
    pub cutime: i64,
    pub cstime: i64,
    pub priority: i64,
    pub nice: i64,
    pub num_threads: u32,
    pub starttime: u64,
    pub vsize: u64,
    pub rss_pages: u64,
}

pub fn parse_stat_line(line: &str) -> Result<PidStat> {
    let start = line
        .find('(')
        .ok_or_else(|| LynxError::parse("stat: missing comm start"))?;
    let end = line
        .rfind(')')
        .ok_or_else(|| LynxError::parse("stat: missing comm end"))?;
    if end <= start {
        return Err(LynxError::parse("stat: inverted comm parentheses"));
    }

    let pid: u32 = line[..start]
        .trim()
        .parse()
        .map_err(|_| LynxError::parse("stat: invalid pid"))?;
    let comm = line[start + 1..end].to_string();
    let rest: Vec<&str> = line[end + 1..].split_whitespace().collect();

    // After comm: 1=state ... 22=rss (1-based field numbers from man page minus pid+comm).
    if rest.len() < 22 {
        return Err(LynxError::parse(format!(
            "stat: expected at least 22 fields after comm, got {}",
            rest.len()
        )));
    }

    let state_ch = rest[0].chars().next().unwrap_or('?');
    let get_i = |i: usize| {
        rest[i]
            .parse::<i64>()
            .map_err(|_| LynxError::parse(format!("stat: field {i} not an integer")))
    };
    let get_u = |i: usize| {
        rest[i]
            .parse::<u64>()
            .map_err(|_| LynxError::parse(format!("stat: field {i} not an integer")))
    };

    Ok(PidStat {
        pid: Pid(pid),
        comm,
        state: ProcessState::from_char(state_ch),
        ppid: Pid(get_u(1)? as u32),
        pgrp: get_i(2)?,
        session: get_i(3)?,
        tty_nr: get_i(4)?,
        flags: get_u(6)?,
        minflt: get_u(7)?,
        majflt: get_u(9)?,
        utime: get_u(11)?,
        stime: get_u(12)?,
        cutime: get_i(13)?,
        cstime: get_i(14)?,
        priority: get_i(15)?,
        nice: get_i(16)?,
        num_threads: get_u(17)? as u32,
        starttime: get_u(19)?,
        vsize: get_u(20)?,
        rss_pages: get_u(21)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comm_with_spaces_and_parens() {
        // Realistic-ish Isolated Web Content style comm with spaces.
        let line = "26231 (Isolated Web Co) S 26154 26111 26111 0 -1 4194304 123 0 1 0 10 20 0 0 20 0 18 0 12345 1234567890 2048 18446744073709551615 0 0 0 0 0 0 0 0 0 0 0 17 3 0 0 0 0 0 0 0 0 0 0 0 0 0";
        let s = parse_stat_line(line).unwrap();
        assert_eq!(s.pid, Pid(26231));
        assert_eq!(s.comm, "Isolated Web Co");
        assert_eq!(s.state, ProcessState::Sleeping);
        assert_eq!(s.ppid, Pid(26154));
        assert_eq!(s.utime, 10);
        assert_eq!(s.stime, 20);
        assert_eq!(s.num_threads, 18);
        assert_eq!(s.vsize, 1_234_567_890);
        assert_eq!(s.rss_pages, 2048);
    }
}

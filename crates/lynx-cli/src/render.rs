use owo_colors::OwoColorize;
use std::io::{self, Write};

use lynx_core::format::{bytes, percent};
use lynx_core::{
    CgroupDetail, CgroupSummary, HostSnapshot, JournalEntry, ProcessDetail, ProcessSummary, Socket,
    SystemdUnit,
};

pub fn print_host_banner(host: &HostSnapshot) {
    let name = host.hostname.as_deref().unwrap_or("unknown");
    let os = host.os_pretty.as_deref().unwrap_or("linux");
    println!(
        "{}  {}  load {:.2} {:.2} {:.2}  mem {} / {}  up {}",
        name.bold(),
        os.dimmed(),
        host.loadavg.one,
        host.loadavg.five,
        host.loadavg.fifteen,
        bytes(host.memory.used_bytes()),
        bytes(host.memory.total_bytes),
        lynx_core::format::duration_secs(host.uptime_secs)
    );
}

pub fn print_process_table(rows: &[ProcessSummary], tree: bool) {
    println!(
        "{:>8} {:<12} {:>7} {:>10} {:>4} {:<22} {}",
        "PID".bold(),
        "USER".bold(),
        "CPU".bold(),
        "RSS".bold(),
        "ST".bold(),
        "UNIT".bold(),
        "COMM".bold()
    );
    if tree {
        print_tree(rows);
        return;
    }
    for p in rows {
        println!(
            "{:>8} {:<12} {:>7} {:>10} {:>4} {:<22} {}",
            p.pid,
            trunc(p.user.as_deref().unwrap_or(&p.uid.to_string()), 12),
            percent(p.cpu_percent).trim(),
            bytes(p.mem_rss_bytes),
            p.state.as_char(),
            trunc(p.unit.as_deref().unwrap_or(""), 22),
            p.comm
        );
    }
}

fn print_tree(rows: &[ProcessSummary]) {
    use std::collections::HashMap;
    let mut by_parent: HashMap<u32, Vec<&ProcessSummary>> = HashMap::new();
    let pids: std::collections::HashSet<u32> = rows.iter().map(|p| p.pid.0).collect();
    let mut roots = Vec::new();
    for p in rows {
        if p.ppid.0 == 0 || !pids.contains(&p.ppid.0) {
            roots.push(p);
        } else {
            by_parent.entry(p.ppid.0).or_default().push(p);
        }
    }
    roots.sort_by_key(|p| p.pid);
    for r in roots {
        walk_tree(r, &by_parent, "");
    }
}

fn walk_tree(
    p: &ProcessSummary,
    by_parent: &std::collections::HashMap<u32, Vec<&ProcessSummary>>,
    prefix: &str,
) {
    println!(
        "{:>8} {:<12} {:>7} {:>10} {:>4} {:<22} {}{}",
        p.pid,
        trunc(p.user.as_deref().unwrap_or(&p.uid.to_string()), 12),
        percent(p.cpu_percent).trim(),
        bytes(p.mem_rss_bytes),
        p.state.as_char(),
        trunc(p.unit.as_deref().unwrap_or(""), 22),
        prefix,
        p.comm
    );
    if let Some(kids) = by_parent.get(&p.pid.0) {
        let mut kids = kids.clone();
        kids.sort_by_key(|k| k.pid);
        for (i, child) in kids.iter().enumerate() {
            let last = i + 1 == kids.len();
            let next = if last {
                format!("{prefix}    ")
            } else {
                format!("{prefix}│   ")
            };
            let _ = last;
            walk_tree(child, by_parent, &next);
        }
    }
}

pub fn print_inspect(d: &ProcessDetail) {
    let s = &d.summary;
    println!("{}  {}", format!("PID {}", s.pid).bold(), s.comm.cyan());
    println!(
        "    {:<12} {}",
        "user",
        s.user.as_deref().unwrap_or(&s.uid.to_string())
    );
    println!("    {:<12} {}", "state", s.state);
    println!("    {:<12} {}", "ppid", s.ppid);
    println!("    {:<12} {}", "threads", s.threads);
    println!("    {:<12} {}", "cpu", percent(s.cpu_percent).trim());
    println!("    {:<12} {}", "rss", bytes(s.mem_rss_bytes));
    println!("    {:<12} {}", "vsz", bytes(s.mem_vsz_bytes));
    println!(
        "    {:<12} {}",
        "cmdline",
        if d.cmdline.is_empty() {
            format!("[{}]", s.comm)
        } else {
            d.cmdline.join(" ")
        }
    );
    if let Some(exe) = &d.exe {
        println!("    {:<12} {}", "exe", exe.display());
    }
    if let Some(cwd) = &d.cwd {
        println!("    {:<12} {}", "cwd", cwd.display());
    }
    println!(
        "    {:<12} {}",
        "cgroup",
        s.cgroup.as_deref().unwrap_or("-")
    );
    println!("    {:<12} {}", "unit", s.unit.as_deref().unwrap_or("-"));
    println!(
        "    {:<12} pid={} mnt={} net={} uts={}",
        "namespaces",
        d.namespaces.pid.as_deref().unwrap_or("-"),
        d.namespaces.mnt.as_deref().unwrap_or("-"),
        d.namespaces.net.as_deref().unwrap_or("-"),
        d.namespaces.uts.as_deref().unwrap_or("-")
    );
    println!(
        "    {:<12} read {}  write {}  syscalls {}/{}",
        "io",
        bytes(d.io.read_bytes),
        bytes(d.io.write_bytes),
        d.io.syscr,
        d.io.syscw
    );
    println!(
        "    {:<12} {} open (showing {})",
        "fds",
        d.fd_count,
        d.fds.len()
    );
    if !d.sockets.is_empty() {
        println!("    {}", "sockets".bold());
        for sock in &d.sockets {
            let local = sock
                .local
                .map(|a| a.to_string())
                .or_else(|| sock.unix_path.clone())
                .unwrap_or_default();
            let remote = sock.remote.map(|a| a.to_string()).unwrap_or_default();
            println!(
                "        {:<6} {:<12} {} → {}",
                sock.protocol, sock.state, local, remote
            );
        }
    }
    if !d.limits.is_empty() {
        if let Some(nofile) = d
            .limits
            .iter()
            .find(|l| l.name.to_ascii_lowercase().contains("open file"))
        {
            println!(
                "    {:<12} {} / {} {}",
                "nofile", nofile.soft, nofile.hard, nofile.units
            );
        }
    }
    if !d.environ.is_empty() {
        println!(
            "    {} ({} keys, secrets redacted)",
            "environ".bold(),
            d.environ.len()
        );
        for (k, v) in d.environ.iter().take(24) {
            println!("        {k}={v}");
        }
        if d.environ.len() > 24 {
            println!("        … {} more", d.environ.len() - 24);
        }
    }
}

pub fn print_cgroups(rows: &[CgroupSummary]) {
    println!(
        "{:>6} {:>10} {:>10} {:>7} {}",
        "NPROC".bold(),
        "MEM".bold(),
        "MAX".bold(),
        "PSI10".bold(),
        "PATH".bold()
    );
    for c in rows {
        println!(
            "{:>6} {:>10} {:>10} {:>7} {}",
            c.nprocs,
            c.memory_current.map(bytes).unwrap_or_else(|| "-".into()),
            c.memory_max.map(bytes).unwrap_or_else(|| "max".into()),
            c.pressure
                .map(|p| format!("{:.1}", p.some_avg10))
                .unwrap_or_else(|| "-".into()),
            c.path
        );
    }
}

pub fn print_cgroup_detail(d: &CgroupDetail) {
    let s = &d.summary;
    println!("{}  {}", "cgroup".bold(), s.path.cyan());
    println!("    {:<12} {}", "version", d.version);
    println!("    {:<12} {}", "procs", s.nprocs);
    println!(
        "    {:<12} {}",
        "memory",
        s.memory_current.map(bytes).unwrap_or_else(|| "-".into())
    );
    println!(
        "    {:<12} {}",
        "memory.max",
        s.memory_max.map(bytes).unwrap_or_else(|| "max".into())
    );
    if let Some(p) = s.pressure {
        println!(
            "    {:<12} some10={:.1} full10={:.1}",
            "psi", p.some_avg10, p.full_avg10
        );
    }
    if !d.controllers.is_empty() {
        println!("    {:<12} {}", "controllers", d.controllers.join(" "));
    }
    if !d.procs.is_empty() {
        let pids: Vec<String> = d.procs.iter().take(32).map(|p| p.to_string()).collect();
        println!("    {:<12} {}", "cgroup.procs", pids.join(" "));
    }
    for (k, v) in d.memory_stat.iter().take(12) {
        println!("    {:<12} {}", k, bytes(*v));
    }
}

pub fn print_sockets(rows: &[Socket]) {
    println!(
        "{:<6} {:<12} {:>8} {:<14} {:<24} {}",
        "PROTO".bold(),
        "STATE".bold(),
        "PID".bold(),
        "COMM".bold(),
        "LOCAL".bold(),
        "REMOTE".bold()
    );
    for s in rows {
        let local = s
            .local
            .map(|a| a.to_string())
            .or_else(|| s.unix_path.clone())
            .unwrap_or_default();
        let remote = s.remote.map(|a| a.to_string()).unwrap_or_default();
        println!(
            "{:<6} {:<12} {:>8} {:<14} {:<24} {}",
            s.protocol,
            s.state,
            s.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".into()),
            trunc(s.comm.as_deref().unwrap_or(""), 14),
            trunc(&local, 24),
            remote
        );
    }
}

pub fn print_units(rows: &[SystemdUnit]) {
    println!(
        "{:<10} {:<12} {:<32} {}",
        "ACTIVE".bold(),
        "SUB".bold(),
        "UNIT".bold(),
        "DESCRIPTION".bold()
    );
    for u in rows {
        let active = if u.is_failed() {
            u.active_state.red().to_string()
        } else {
            u.active_state.clone()
        };
        println!(
            "{:<10} {:<12} {:<32} {}",
            active,
            u.sub_state,
            trunc(&u.name, 32),
            u.description
        );
    }
}

pub fn print_unit(u: &SystemdUnit) {
    println!("{}  {}", "unit".bold(), u.name.cyan());
    println!("    {:<16} {}", "description", u.description);
    println!("    {:<16} {} / {}", "state", u.active_state, u.sub_state);
    println!("    {:<16} {}", "load", u.load_state);
    if let Some(pid) = u.main_pid {
        println!("    {:<16} {}", "main_pid", pid);
    }
    if let Some(cg) = &u.cgroup {
        println!("    {:<16} {cg}", "cgroup");
    }
    if let Some(p) = &u.fragment_path {
        println!("    {:<16} {p}", "fragment");
    }
    if let Some(r) = u.n_restarts {
        println!("    {:<16} {r}", "restarts");
    }
}

pub fn print_journal(rows: &[JournalEntry]) {
    for e in rows {
        let prio = e.priority_label();
        let colored = match e.priority.unwrap_or(6) {
            0..=3 => prio.red().to_string(),
            4 => prio.yellow().to_string(),
            _ => prio.to_string(),
        };
        println!(
            "{}  {:>7}  {:<20} {}",
            e.realtime.as_deref().unwrap_or("-"),
            colored,
            trunc(e.unit.as_deref().unwrap_or("-"), 20),
            e.message
        );
    }
}

pub fn dump_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    let mut out = io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, value)?;
    writeln!(out)?;
    Ok(())
}

fn trunc(s: &str, width: usize) -> String {
    lynx_core::format::cell(s, width)
}

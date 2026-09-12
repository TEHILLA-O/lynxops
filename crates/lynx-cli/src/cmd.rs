use std::time::Duration;

use anyhow::{Context, Result};
use lynx_cgroup::CgroupCollector;
use lynx_core::{sort_processes, SysPaths};
use lynx_ebpf::probe_capability;
use lynx_network::NetCollector;
use lynx_proc::{load_host, ProcCollector};
use lynx_snapshot::{self as snapshot, SnapshotOptions};
use lynx_systemd::{JournalQuery, SystemdCollector};

use crate::cli::{CgroupCmd, Cli, Command, NetCmd, SnapshotCmd, SystemdCmd};
use crate::render;

pub fn dispatch(cli: Cli) -> Result<()> {
    let paths = cli.paths();
    let json = cli.json;
    match cli.command {
        Command::Ps {
            sort,
            tree,
            limit,
            filter,
            no_sample,
        } => cmd_ps(&paths, json, sort.into(), tree, limit, filter, no_sample),
        Command::Inspect { pid, show_secrets } => cmd_inspect(&paths, json, pid, show_secrets),
        Command::Tui | Command::Top => lynx_tui::run(paths).map_err(Into::into),
        Command::Cgroup { action } => {
            cmd_cgroup(&paths, json, action.unwrap_or(CgroupCmd::List { depth: 5 }))
        }
        Command::Net { action } => cmd_net(&paths, json, action.unwrap_or(NetCmd::Listen)),
        Command::Systemd { action } => {
            cmd_systemd(json, action.unwrap_or(SystemdCmd::Units { failed: false }))
        }
        Command::Journal {
            lines,
            priority,
            unit,
            all_boots,
        } => cmd_journal(json, lines, priority, unit, !all_boots),
        Command::Snapshot { action } => cmd_snapshot(&paths, json, action),
        Command::Doctor => cmd_doctor(&paths, json),
    }
}

fn cmd_ps(
    paths: &SysPaths,
    json: bool,
    sort: lynx_core::ProcessSort,
    tree: bool,
    limit: Option<usize>,
    filter: Option<String>,
    no_sample: bool,
) -> Result<()> {
    let collector = ProcCollector::new(paths.clone());
    let mut rows = if no_sample {
        collector.list_once()?
    } else {
        collector.list_with_sample(Duration::from_millis(200))?
    };
    if let Some(q) = filter {
        let q = q.to_ascii_lowercase();
        rows.retain(|p| {
            p.comm.to_ascii_lowercase().contains(&q)
                || p.user
                    .as_deref()
                    .map(|u| u.to_ascii_lowercase().contains(&q))
                    .unwrap_or(false)
                || p.unit
                    .as_deref()
                    .map(|u| u.to_ascii_lowercase().contains(&q))
                    .unwrap_or(false)
        });
    }
    sort_processes(&mut rows, sort);
    if let Some(n) = limit {
        rows.truncate(n);
    }
    if json {
        return render::dump_json(&rows);
    }
    if let Ok(host) = load_host(paths) {
        render::print_host_banner(&host);
    }
    render::print_process_table(&rows, tree);
    Ok(())
}

fn cmd_inspect(
    paths: &SysPaths,
    json: bool,
    pid: lynx_core::Pid,
    show_secrets: bool,
) -> Result<()> {
    let mut detail = ProcCollector::new(paths.clone())
        .inspect(pid, show_secrets)
        .with_context(|| format!("inspect pid {pid}"))?;
    if let Ok(socks) = NetCollector::new(paths.clone()).for_pid(pid) {
        detail.sockets = socks;
    }
    if json {
        return render::dump_json(&detail);
    }
    render::print_inspect(&detail);
    Ok(())
}

fn cmd_cgroup(paths: &SysPaths, json: bool, action: CgroupCmd) -> Result<()> {
    let collector = CgroupCollector::new(paths.clone());
    match action {
        CgroupCmd::List { depth } => {
            let rows = collector.list(depth)?;
            if json {
                render::dump_json(&rows)
            } else {
                println!("cgroup {}", collector.version()?);
                render::print_cgroups(&rows);
                Ok(())
            }
        }
        CgroupCmd::Inspect { path } => {
            let detail = collector.inspect(&path)?;
            if json {
                render::dump_json(&detail)
            } else {
                render::print_cgroup_detail(&detail);
                Ok(())
            }
        }
    }
}

fn cmd_net(paths: &SysPaths, json: bool, action: NetCmd) -> Result<()> {
    let collector = NetCollector::new(paths.clone());
    match action {
        NetCmd::Listen => {
            let rows = collector.listening()?;
            if json {
                render::dump_json(&rows)
            } else {
                render::print_sockets(&rows);
                Ok(())
            }
        }
        NetCmd::Conn => {
            let rows = collector.established()?;
            if json {
                render::dump_json(&rows)
            } else {
                render::print_sockets(&rows);
                Ok(())
            }
        }
        NetCmd::Pid { pid } => {
            let rows = collector.for_pid(pid)?;
            if json {
                render::dump_json(&rows)
            } else {
                render::print_sockets(&rows);
                Ok(())
            }
        }
        NetCmd::If => {
            let rows = collector.interfaces()?;
            if json {
                render::dump_json(&rows)
            } else {
                for i in rows {
                    println!(
                        "{:<12} rx {} ({} pkts, {} err)  tx {} ({} pkts, {} err)",
                        i.name,
                        lynx_core::format::bytes(i.rx_bytes),
                        i.rx_packets,
                        i.rx_errs,
                        lynx_core::format::bytes(i.tx_bytes),
                        i.tx_packets,
                        i.tx_errs
                    );
                }
                Ok(())
            }
        }
    }
}

fn cmd_systemd(json: bool, action: SystemdCmd) -> Result<()> {
    let collector = SystemdCollector::new();
    match action {
        SystemdCmd::Units { failed } => {
            let rows = if failed {
                collector.failed_units()?
            } else {
                collector.list_units()?
            };
            if json {
                render::dump_json(&rows)
            } else {
                render::print_units(&rows);
                Ok(())
            }
        }
        SystemdCmd::Unit { name } => {
            let unit = collector.inspect(&name)?;
            if json {
                render::dump_json(&unit)
            } else {
                render::print_unit(&unit);
                Ok(())
            }
        }
    }
}

fn cmd_journal(
    json: bool,
    lines: u32,
    priority: Option<String>,
    unit: Option<String>,
    boot: bool,
) -> Result<()> {
    let rows = SystemdCollector::new().journal(JournalQuery {
        lines,
        priority,
        unit,
        boot,
    })?;
    if json {
        render::dump_json(&rows)
    } else {
        render::print_journal(&rows);
        Ok(())
    }
}

fn cmd_snapshot(paths: &SysPaths, json: bool, action: SnapshotCmd) -> Result<()> {
    match action {
        SnapshotCmd::Capture { output } => {
            let opts = SnapshotOptions {
                paths: paths.clone(),
                ..SnapshotOptions::default()
            };
            let snap = snapshot::capture(&opts);
            let path = output.unwrap_or_else(snapshot::default_path);
            snapshot::save(&snap, &path)?;
            if json {
                render::dump_json(&snap)
            } else {
                println!(
                    "wrote {}  ({} procs, {} listeners, {} failed units, {} journal errors)",
                    path.display(),
                    snap.processes.len(),
                    snap.listening.len(),
                    snap.failed_units.len(),
                    snap.journal_errors.len()
                );
                Ok(())
            }
        }
        SnapshotCmd::Show { path } => {
            let snap = snapshot::load(&path)?;
            if json {
                render::dump_json(&snap)
            } else {
                render::print_host_banner(&snap.host);
                println!(
                    "captured {}  lynxops {}",
                    snap.captured_at.to_rfc3339(),
                    snap.lynxops_version
                );
                println!("\ntop processes");
                render::print_process_table(
                    &snap.processes.iter().take(20).cloned().collect::<Vec<_>>(),
                    false,
                );
                if !snap.failed_units.is_empty() {
                    println!("\nfailed units");
                    render::print_units(&snap.failed_units);
                }
                if !snap.journal_errors.is_empty() {
                    println!("\njournal errors");
                    render::print_journal(&snap.journal_errors);
                }
                if !snap.listening.is_empty() {
                    println!("\nlistening");
                    render::print_sockets(&snap.listening);
                }
                Ok(())
            }
        }
    }
}

fn cmd_doctor(paths: &SysPaths, json: bool) -> Result<()> {
    let proc_ok = paths.proc.join("stat").exists();
    let cg = CgroupCollector::new(paths.clone());
    let cg_ver = cg.version().ok();
    let sd = SystemdCollector::new();
    let ebpf = probe_capability();
    #[derive(serde::Serialize)]
    struct Doctor {
        procfs: bool,
        proc_root: String,
        cgroup: Option<String>,
        systemd: bool,
        journal: bool,
        ebpf_status: String,
        ebpf_btf: bool,
        ebpf_notes: Vec<String>,
    }
    let report = Doctor {
        procfs: proc_ok,
        proc_root: paths.proc.display().to_string(),
        cgroup: cg_ver.map(|v| v.to_string()),
        systemd: sd.available(),
        journal: sd.journal_available(),
        ebpf_status: ebpf.status.to_string(),
        ebpf_btf: ebpf.btf,
        ebpf_notes: ebpf.notes.clone(),
    };
    if json {
        return render::dump_json(&report);
    }
    println!("LynxOps doctor");
    println!(
        "    {:<12} {} ({})",
        "procfs",
        yn(proc_ok),
        paths.proc.display()
    );
    println!(
        "    {:<12} {}",
        "cgroup",
        cg_ver
            .map(|v| v.to_string())
            .unwrap_or_else(|| "missing".into())
    );
    println!("    {:<12} {}", "systemd", yn(sd.available()));
    println!("    {:<12} {}", "journal", yn(sd.journal_available()));
    println!("    {:<12} {}  btf={}", "ebpf", ebpf.status, ebpf.btf);
    for n in &ebpf.notes {
        println!("                {n}");
    }
    Ok(())
}

fn yn(v: bool) -> &'static str {
    if v {
        "ok"
    } else {
        "missing"
    }
}

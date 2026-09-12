use lynx_core::{Pid, ProcessState, SysPaths};
use lynx_proc::{load_host, ProcCollector};

fn fixture_paths() -> SysPaths {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/sysroot");
    SysPaths::from_sysroot(root)
}

#[test]
fn host_from_fixture() {
    let host = load_host(&fixture_paths()).expect("host");
    assert_eq!(host.hostname.as_deref(), Some("lynxops-dev"));
    assert_eq!(host.memory.total_bytes, 16_384_000 * 1024);
    assert!((host.loadavg.one - 0.42).abs() < 0.001);
    assert_eq!(host.uptime_secs, 12345);
    assert_eq!(host.cpu.logical_cpus, 2);
}

#[test]
fn list_and_inspect_nginx() {
    let proc = ProcCollector::new(fixture_paths());
    let rows = proc.list_once().expect("list");
    assert!(rows.iter().any(|p| p.pid == Pid(1) && p.comm == "systemd"));
    let nginx = rows.iter().find(|p| p.pid == Pid(1420)).expect("nginx");
    assert_eq!(nginx.comm, "nginx");
    assert_eq!(nginx.state, ProcessState::Sleeping);
    assert_eq!(nginx.user.as_deref(), Some("www-data"));
    assert_eq!(nginx.unit.as_deref(), Some("nginx.service"));

    let detail = proc.inspect(Pid(1420), false).expect("inspect");
    assert_eq!(detail.summary.unit.as_deref(), Some("nginx.service"));
    assert!(!detail.limits.is_empty());
    assert_eq!(detail.io.read_bytes, 4096);
}

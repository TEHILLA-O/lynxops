use lynx_cgroup::CgroupCollector;
use lynx_core::{CgroupVersion, Pid, SysPaths};

fn fixture_paths() -> SysPaths {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/sysroot");
    SysPaths::from_sysroot(root)
}

#[test]
fn v2_nginx_cgroup() {
    let cg = CgroupCollector::new(fixture_paths());
    assert_eq!(cg.version().unwrap(), CgroupVersion::V2);
    let detail = cg.inspect("/system.slice/nginx.service").unwrap();
    assert_eq!(detail.summary.memory_current, Some(51_773_440));
    assert_eq!(detail.summary.memory_max, Some(134_217_728));
    assert_eq!(detail.procs, vec![Pid(1420)]);
    assert_eq!(detail.summary.cpu_usage_usec, Some(1_234_567));
}

use lynx_core::{Protocol, SocketState, SysPaths};
use lynx_network::{parse_inet_table, NetCollector};

fn fixture_paths() -> SysPaths {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/sysroot");
    SysPaths::from_sysroot(root)
}

#[test]
fn tcp_table_from_fixture() {
    let text = std::fs::read_to_string(fixture_paths().proc.join("net/tcp")).unwrap();
    let rows = parse_inet_table(&text, Protocol::Tcp).unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].state, SocketState::Listen);
    assert_eq!(rows[0].local.unwrap().port(), 80);
    assert_eq!(rows[2].state, SocketState::Established);

    let net = NetCollector::new(fixture_paths());
    let ifs = net.interfaces().unwrap();
    assert!(ifs
        .iter()
        .any(|i| i.name == "eth0" && i.rx_bytes == 2_048_000));
}

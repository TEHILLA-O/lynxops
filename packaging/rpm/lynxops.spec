Name:           lynxops
Version:        0.1.0
Release:        1%{?dist}
Summary:        Linux observability and incident-triage CLI/TUI
License:        MIT
URL:            https://github.com/lynxops/lynxops
Source0:        %{name}-%{version}.tar.gz
BuildRequires:  rust >= 1.80
BuildRequires:  cargo

%description
LynxOps inspects processes, cgroups, sockets, systemd units, and the
journal, then correlates them for incident response.

%prep
%autosetup

%build
cargo build --release --locked -p lynx-cli

%install
install -D -m 0755 target/release/lynxops %{buildroot}%{_bindir}/lynxops

%files
%license LICENSE
%doc README.md docs
%{_bindir}/lynxops

%changelog
* Sat Sep 12 2026 LynxOps Contributors <lynxops@example.com> - 0.1.0-1
- Initial package

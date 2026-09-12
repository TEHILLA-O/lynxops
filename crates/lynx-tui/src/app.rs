use std::time::{Duration, Instant};

use lynx_cgroup::CgroupCollector;
use lynx_core::{
    sort_processes, CgroupSummary, JournalEntry, ProcessSort, ProcessSummary, Socket, SysPaths,
    SystemdUnit,
};
use lynx_network::NetCollector;
use lynx_proc::{load_host, ProcCollector};
use lynx_systemd::{JournalQuery, SystemdCollector};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Processes,
    Cgroups,
    Network,
    Systemd,
    Journal,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Processes,
        Tab::Cgroups,
        Tab::Network,
        Tab::Systemd,
        Tab::Journal,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Processes => "Processes",
            Self::Cgroups => "Cgroups",
            Self::Network => "Network",
            Self::Systemd => "Systemd",
            Self::Journal => "Journal",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Processes => Self::Cgroups,
            Self::Cgroups => Self::Network,
            Self::Network => Self::Systemd,
            Self::Systemd => Self::Journal,
            Self::Journal => Self::Processes,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Processes => Self::Journal,
            Self::Cgroups => Self::Processes,
            Self::Network => Self::Cgroups,
            Self::Systemd => Self::Network,
            Self::Journal => Self::Systemd,
        }
    }
}

pub struct App {
    pub paths: SysPaths,
    pub tab: Tab,
    pub sort: ProcessSort,
    pub selected: usize,
    pub search: String,
    pub searching: bool,
    pub status: String,
    pub last_refresh: Instant,
    pub interval: Duration,
    pub should_quit: bool,
    pub host_line: String,
    pub processes: Vec<ProcessSummary>,
    pub cgroups: Vec<CgroupSummary>,
    pub sockets: Vec<Socket>,
    pub units: Vec<SystemdUnit>,
    pub journal: Vec<JournalEntry>,
}

impl App {
    pub fn new(paths: SysPaths) -> Self {
        let mut app = Self {
            paths,
            tab: Tab::Processes,
            sort: ProcessSort::Cpu,
            selected: 0,
            search: String::new(),
            searching: false,
            status: "loading…".into(),
            last_refresh: Instant::now() - Duration::from_secs(60),
            interval: Duration::from_secs(1),
            should_quit: false,
            host_line: String::new(),
            processes: Vec::new(),
            cgroups: Vec::new(),
            sockets: Vec::new(),
            units: Vec::new(),
            journal: Vec::new(),
        };
        app.refresh();
        app
    }

    pub fn filtered_len(&self) -> usize {
        self.filtered_count()
    }

    pub fn refresh(&mut self) {
        if let Ok(host) = load_host(&self.paths) {
            let mem = lynx_core::format::bytes(host.memory.used_bytes());
            let tot = lynx_core::format::bytes(host.memory.total_bytes);
            self.host_line = format!(
                "{}  load {:.2} {:.2} {:.2}  mem {mem}/{tot}  up {}",
                host.hostname.as_deref().unwrap_or("unknown"),
                host.loadavg.one,
                host.loadavg.five,
                host.loadavg.fifteen,
                lynx_core::format::duration_secs(host.uptime_secs),
            );
        }

        let proc = ProcCollector::new(self.paths.clone());
        match proc.list_with_sample(Duration::from_millis(150)) {
            Ok(mut rows) => {
                sort_processes(&mut rows, self.sort);
                self.processes = rows;
            }
            Err(e) => self.status = format!("proc: {e}"),
        }

        let cg = CgroupCollector::new(self.paths.clone());
        self.cgroups = cg.list(5).unwrap_or_default();

        let net = NetCollector::new(self.paths.clone());
        self.sockets = net.sockets().unwrap_or_default();

        let sd = SystemdCollector::new();
        if sd.available() {
            self.units = sd.list_units().unwrap_or_default();
            if sd.journal_available() {
                self.journal = sd
                    .journal(JournalQuery {
                        lines: 80,
                        priority: Some("warning".into()),
                        unit: None,
                        boot: true,
                    })
                    .unwrap_or_default();
            }
        }

        self.clamp_selection();
        self.last_refresh = Instant::now();
        self.status = format!(
            "{} procs  {} listen  {} units  sort {:?}",
            self.processes.len(),
            self.sockets.iter().filter(|s| s.is_listen()).count(),
            self.units.len(),
            self.sort
        );
    }

    pub fn maybe_refresh(&mut self) {
        if self.last_refresh.elapsed() >= self.interval && !self.searching {
            self.refresh();
        }
    }

    pub fn on_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        if self.searching {
            match key.code {
                KeyCode::Esc => {
                    self.searching = false;
                    self.search.clear();
                    self.selected = 0;
                }
                KeyCode::Enter => self.searching = false,
                KeyCode::Backspace => {
                    self.search.pop();
                    self.selected = 0;
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.search.push(c);
                    self.selected = 0;
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Tab | KeyCode::Right => {
                self.tab = self.tab.next();
                self.selected = 0;
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.tab = self.tab.prev();
                self.selected = 0;
            }
            KeyCode::Char('/') => self.searching = true,
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Char('c') => {
                self.sort = ProcessSort::Cpu;
                sort_processes(&mut self.processes, self.sort);
            }
            KeyCode::Char('m') => {
                self.sort = ProcessSort::Memory;
                sort_processes(&mut self.processes, self.sort);
            }
            KeyCode::Char('p') => {
                self.sort = ProcessSort::Pid;
                sort_processes(&mut self.processes, self.sort);
            }
            KeyCode::Char('n') => {
                self.sort = ProcessSort::Name;
                sort_processes(&mut self.processes, self.sort);
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_sel(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_sel(-1),
            KeyCode::PageDown => self.move_sel(20),
            KeyCode::PageUp => self.move_sel(-20),
            KeyCode::Home => self.selected = 0,
            KeyCode::End => {
                let n = self.filtered_count();
                self.selected = n.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn move_sel(&mut self, delta: i32) {
        let n = self.filtered_count() as i32;
        if n == 0 {
            self.selected = 0;
            return;
        }
        let next = (self.selected as i32 + delta).clamp(0, n - 1);
        self.selected = next as usize;
    }

    fn clamp_selection(&mut self) {
        let n = self.filtered_count();
        if n == 0 {
            self.selected = 0;
        } else if self.selected >= n {
            self.selected = n - 1;
        }
    }

    fn filtered_count(&self) -> usize {
        match self.tab {
            Tab::Processes => self.filtered_processes().len(),
            Tab::Cgroups => self.filtered_cgroups().len(),
            Tab::Network => self.filtered_sockets().len(),
            Tab::Systemd => self.filtered_units().len(),
            Tab::Journal => self.filtered_journal().len(),
        }
    }

    pub fn filtered_processes(&self) -> Vec<&ProcessSummary> {
        self.processes
            .iter()
            .filter(|p| {
                matches_query(
                    &self.search,
                    &[
                        &p.comm,
                        &p.pid.to_string(),
                        p.user.as_deref().unwrap_or(""),
                        p.unit.as_deref().unwrap_or(""),
                    ],
                )
            })
            .collect()
    }

    pub fn filtered_cgroups(&self) -> Vec<&CgroupSummary> {
        self.cgroups
            .iter()
            .filter(|c| matches_query(&self.search, &[&c.path]))
            .collect()
    }

    pub fn filtered_sockets(&self) -> Vec<&Socket> {
        self.sockets
            .iter()
            .filter(|s| {
                let local = s.local.map(|a| a.to_string()).unwrap_or_default();
                let comm = s.comm.clone().unwrap_or_default();
                matches_query(
                    &self.search,
                    &[&local, &comm, s.protocol.as_str(), s.state.as_str()],
                )
            })
            .collect()
    }

    pub fn filtered_units(&self) -> Vec<&SystemdUnit> {
        self.units
            .iter()
            .filter(|u| matches_query(&self.search, &[&u.name, &u.active_state, &u.description]))
            .collect()
    }

    pub fn filtered_journal(&self) -> Vec<&JournalEntry> {
        self.journal
            .iter()
            .filter(|e| matches_query(&self.search, &[&e.message, e.unit.as_deref().unwrap_or("")]))
            .collect()
    }

    pub fn selected_process(&self) -> Option<&ProcessSummary> {
        self.filtered_processes().get(self.selected).copied()
    }
}

fn matches_query(query: &str, fields: &[&str]) -> bool {
    if query.is_empty() {
        return true;
    }
    let q = query.to_ascii_lowercase();
    fields.iter().any(|f| f.to_ascii_lowercase().contains(&q))
}

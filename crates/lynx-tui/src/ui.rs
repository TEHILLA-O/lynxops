use lynx_core::format::{bytes, percent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs, Wrap};
use ratatui::Frame;

use crate::app::{App, Tab};

const ACCENT: Color = Color::Rgb(120, 200, 180);
const DIM: Color = Color::Rgb(110, 120, 130);
const WARN: Color = Color::Rgb(230, 180, 80);
const ERR: Color = Color::Rgb(220, 90, 90);

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(7),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_header(frame, chunks[0], app);
    draw_tabs(frame, chunks[1], app);
    draw_table(frame, chunks[2], app);
    draw_detail(frame, chunks[3], app);
    draw_footer(frame, chunks[4], app);
}

fn draw_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let line = Line::from(vec![
        Span::styled(
            " LynxOps ",
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(app.host_line.clone(), Style::default().fg(DIM)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_tabs(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let titles: Vec<Line> = Tab::ALL.iter().map(|t| Line::from(t.title())).collect();
    let selected = Tab::ALL.iter().position(|t| *t == app.tab).unwrap_or(0);
    let tabs = Tabs::new(titles)
        .select(selected)
        .block(Block::default().borders(Borders::ALL).title("views"))
        .highlight_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
        .divider(" │ ");
    frame.render_widget(tabs, area);
}

fn draw_table(frame: &mut Frame<'_>, area: Rect, app: &App) {
    match app.tab {
        Tab::Processes => draw_processes(frame, area, app),
        Tab::Cgroups => draw_cgroups(frame, area, app),
        Tab::Network => draw_network(frame, area, app),
        Tab::Systemd => draw_units(frame, area, app),
        Tab::Journal => draw_journal(frame, area, app),
    }
}

fn selected_style() -> Style {
    Style::default().bg(Color::Rgb(30, 50, 48)).fg(Color::White)
}

fn draw_processes(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let rows: Vec<Row> = app
        .filtered_processes()
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let row = Row::new(vec![
                Cell::from(p.pid.to_string()),
                Cell::from(p.user.clone().unwrap_or_else(|| p.uid.to_string())),
                Cell::from(percent(p.cpu_percent)),
                Cell::from(bytes(p.mem_rss_bytes)),
                Cell::from(p.state.as_char().to_string()),
                Cell::from(p.unit.clone().unwrap_or_default()),
                Cell::from(p.comm.clone()),
            ]);
            if i == app.selected {
                row.style(selected_style())
            } else if p.state == lynx_core::ProcessState::Zombie {
                row.style(Style::default().fg(WARN))
            } else {
                row
            }
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(7),
            Constraint::Length(10),
            Constraint::Length(4),
            Constraint::Length(22),
            Constraint::Min(12),
        ],
    )
    .header(header_row([
        "PID", "USER", "CPU", "RSS", "ST", "UNIT", "COMM",
    ]))
    .block(Block::default().borders(Borders::ALL).title("processes"));
    frame.render_widget(table, area);
}

fn draw_cgroups(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let rows: Vec<Row> = app
        .filtered_cgroups()
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let mem = c.memory_current.map(bytes).unwrap_or_else(|| "-".into());
            let max = c.memory_max.map(bytes).unwrap_or_else(|| "max".into());
            let psi = c
                .pressure
                .map(|p| format!("{:.1}", p.some_avg10))
                .unwrap_or_else(|| "-".into());
            let row = Row::new(vec![
                Cell::from(c.nprocs.to_string()),
                Cell::from(mem),
                Cell::from(max),
                Cell::from(psi),
                Cell::from(c.path.clone()),
            ]);
            if i == app.selected {
                row.style(selected_style())
            } else {
                row
            }
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Min(20),
        ],
    )
    .header(header_row(["NPROC", "MEM", "MAX", "PSI10", "PATH"]))
    .block(Block::default().borders(Borders::ALL).title("cgroups"));
    frame.render_widget(table, area);
}

fn draw_network(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let rows: Vec<Row> = app
        .filtered_sockets()
        .into_iter()
        .enumerate()
        .map(|(i, s)| {
            let local = s
                .local
                .map(|a| a.to_string())
                .or_else(|| s.unix_path.clone())
                .unwrap_or_default();
            let remote = s.remote.map(|a| a.to_string()).unwrap_or_default();
            let row = Row::new(vec![
                Cell::from(s.protocol.as_str()),
                Cell::from(s.state.as_str()),
                Cell::from(s.pid.map(|p| p.to_string()).unwrap_or_default()),
                Cell::from(s.comm.clone().unwrap_or_default()),
                Cell::from(local),
                Cell::from(remote),
            ]);
            if i == app.selected {
                row.style(selected_style())
            } else if s.is_listen() {
                row.style(Style::default().fg(ACCENT))
            } else {
                row
            }
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(6),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(14),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ],
    )
    .header(header_row([
        "PROTO", "STATE", "PID", "COMM", "LOCAL", "REMOTE",
    ]))
    .block(Block::default().borders(Borders::ALL).title("sockets"));
    frame.render_widget(table, area);
}

fn draw_units(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let rows: Vec<Row> = app
        .filtered_units()
        .into_iter()
        .enumerate()
        .map(|(i, u)| {
            let row = Row::new(vec![
                Cell::from(u.active_state.clone()),
                Cell::from(u.sub_state.clone()),
                Cell::from(u.name.clone()),
                Cell::from(u.description.clone()),
            ]);
            let row = if i == app.selected {
                row.style(selected_style())
            } else if u.is_failed() {
                row.style(Style::default().fg(ERR))
            } else {
                row
            };
            row
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(32),
            Constraint::Min(20),
        ],
    )
    .header(header_row(["ACTIVE", "SUB", "UNIT", "DESCRIPTION"]))
    .block(Block::default().borders(Borders::ALL).title("units"));
    frame.render_widget(table, area);
}

fn draw_journal(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let rows: Vec<Row> = app
        .filtered_journal()
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            let row = Row::new(vec![
                Cell::from(e.realtime.clone().unwrap_or_default()),
                Cell::from(e.priority_label()),
                Cell::from(e.unit.clone().unwrap_or_default()),
                Cell::from(e.message.clone()),
            ]);
            if i == app.selected {
                row.style(selected_style())
            } else if e.priority.unwrap_or(6) <= 3 {
                row.style(Style::default().fg(ERR))
            } else if e.priority.unwrap_or(6) <= 4 {
                row.style(Style::default().fg(WARN))
            } else {
                row
            }
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(8),
            Constraint::Length(24),
            Constraint::Min(20),
        ],
    )
    .header(header_row(["TIME", "PRIO", "UNIT", "MESSAGE"]))
    .block(Block::default().borders(Borders::ALL).title("journal"));
    frame.render_widget(table, area);
}

fn draw_detail(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let text = match app.tab {
        Tab::Processes => {
            if let Some(p) = app.selected_process() {
                format!(
                    "pid {}  ppid {}  {}  threads {}  rss {}  vsz {}  cgroup {}\nunit {}  user {}",
                    p.pid,
                    p.ppid,
                    p.state,
                    p.threads,
                    bytes(p.mem_rss_bytes),
                    bytes(p.mem_vsz_bytes),
                    p.cgroup.as_deref().unwrap_or("-"),
                    p.unit.as_deref().unwrap_or("-"),
                    p.user.as_deref().unwrap_or("-"),
                )
            } else {
                "no process selected".into()
            }
        }
        Tab::Cgroups => app
            .filtered_cgroups()
            .get(app.selected)
            .map(|c| {
                format!(
                    "{}  procs {}  mem {} / {}  psi10 {}",
                    c.path,
                    c.nprocs,
                    c.memory_current.map(bytes).unwrap_or_else(|| "-".into()),
                    c.memory_max.map(bytes).unwrap_or_else(|| "max".into()),
                    c.pressure
                        .map(|p| format!("{:.1}", p.some_avg10))
                        .unwrap_or_else(|| "-".into()),
                )
            })
            .unwrap_or_else(|| "no cgroup selected".into()),
        Tab::Network => app
            .filtered_sockets()
            .get(app.selected)
            .map(|s| {
                format!(
                    "{} {}  {} → {}  inode {}  pid {}",
                    s.protocol,
                    s.state,
                    s.local
                        .map(|a| a.to_string())
                        .or_else(|| s.unix_path.clone())
                        .unwrap_or_default(),
                    s.remote.map(|a| a.to_string()).unwrap_or_default(),
                    s.inode,
                    s.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".into()),
                )
            })
            .unwrap_or_else(|| "no socket selected".into()),
        Tab::Systemd => app
            .filtered_units()
            .get(app.selected)
            .map(|u| {
                format!(
                    "{}  {}/{}  pid {}  {}",
                    u.name,
                    u.active_state,
                    u.sub_state,
                    u.main_pid
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "-".into()),
                    u.description
                )
            })
            .unwrap_or_else(|| "no unit selected".into()),
        Tab::Journal => app
            .filtered_journal()
            .get(app.selected)
            .map(|e| e.message.clone())
            .unwrap_or_else(|| "no entry selected".into()),
    };
    let p = Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("detail"));
    frame.render_widget(p, area);
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let help = if app.searching {
        format!(" filter: {}_   enter confirm  esc clear", app.search)
    } else {
        format!(
            " q quit  tab views  / filter  r refresh  c/m/p/n sort  j/k move   {}",
            app.status
        )
    };
    frame.render_widget(Paragraph::new(help).style(Style::default().fg(DIM)), area);
}

fn header_row<const N: usize>(cols: [&str; N]) -> Row<'static> {
    Row::new(cols.map(|c| Cell::from(c.to_string())))
        .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
}

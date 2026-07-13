use std::{collections::HashMap, io, time::Duration};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Wrap, List, ListItem, ListState, Table, Row, Cell, TableState},
    Frame, Terminal,
    style::{Style, Color, Modifier},
    text::{Line, Span},
};
use crate::config::personality::Config as PersonalityConfig;

#[derive(Clone, Debug)]
enum SimulatedEvent {
    Cmd(&'static str, &'static str),      // (command, CWD)
    Error(&'static str, &'static str),    // (command, CWD)
    AppStart(&'static str, &'static str), // (app, CWD)
    AppStop(&'static str, &'static str),  // (app, CWD)
    Idle(u64),                            // Seconds
}

struct Scenario {
    name: &'static str,
    description: &'static str,
    events: Vec<SimulatedEvent>,
}

#[derive(Clone, Debug, Default)]
struct ScenarioMetrics {
    cmds: u64,
    trig: u64,
    idle: u64,
    duration: f64,
    longest_pause: f64,
    app_time: f64,
    trig_rate: f64,
    avg_sec: f64,
    avg_sec_excl: f64,
}

#[derive(Clone, Debug)]
struct LogEntry {
    time: f64,
    kind: LogKind,
    text: String,
    mood: String,
}

#[derive(Clone, Debug, PartialEq)]
enum LogKind {
    Cmd,
    Error,
    AppStart,
    AppStop,
    Idle,
    System,
    Speech,
}

#[derive(Clone, Debug)]
struct RunResult {
    metrics: ScenarioMetrics,
    logs: Vec<LogEntry>,
}

struct SimulatedState {
    mood: String,
    default_mood: String,
    command_counts: HashMap<String, u64>,
    last_message_time: f64,
    last_activity_time: f64,
    mood_last_set: f64,
    is_sleeping: bool,
    error_streak: u32,
    current_time: f64,
    active_app: Option<String>,
}

#[derive(PartialEq)]
enum Focus {
    ScenarioList,
    TimelineLog,
}

struct App {
    scenarios: Vec<Scenario>,
    results: Vec<RunResult>,
    selected_scenario: usize,
    selected_log: usize,
    focus: Focus,
    config: PersonalityConfig,
    list_state: ListState,
    log_list_state: TableState,
    fullscreen_matrix: bool,
}

impl App {
    fn new(config: PersonalityConfig) -> Self {
        let scenarios = get_scenarios();
        let results = vec![RunResult {
            metrics: ScenarioMetrics::default(),
            logs: Vec::new(),
        }; scenarios.len()];

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let mut log_list_state = TableState::default();
        log_list_state.select(Some(0));

        Self {
            scenarios,
            results,
            selected_scenario: 0,
            selected_log: 0,
            focus: Focus::ScenarioList,
            config,
            list_state,
            log_list_state,
            fullscreen_matrix: false,
        }
    }

    fn run_all_scenarios(&mut self) {
        for idx in 0..self.scenarios.len() {
            let (metrics, logs) = run_scenario(&self.scenarios[idx], &self.config);
            self.results[idx] = RunResult { metrics, logs };
        }
        self.update_log_index();
    }

    fn run_selected_scenario(&mut self) {
        let idx = self.selected_scenario;
        let (metrics, logs) = run_scenario(&self.scenarios[idx], &self.config);
        self.results[idx] = RunResult { metrics, logs };
        self.update_log_index();
    }

    fn update_log_index(&mut self) {
        let logs_count = self.results[self.selected_scenario].logs.len();
        if logs_count > 0 {
            if self.selected_log >= logs_count {
                self.selected_log = logs_count - 1;
            }
            self.log_list_state.select(Some(self.selected_log));
        } else {
            self.selected_log = 0;
            self.log_list_state.select(None);
        }
    }

    fn next_scenario(&mut self) {
        if self.selected_scenario < self.scenarios.len() - 1 {
            self.selected_scenario += 1;
        } else {
            self.selected_scenario = 0;
        }
        self.list_state.select(Some(self.selected_scenario));
        self.selected_log = 0;
        self.update_log_index();
    }

    fn previous_scenario(&mut self) {
        if self.selected_scenario > 0 {
            self.selected_scenario -= 1;
        } else {
            self.selected_scenario = self.scenarios.len() - 1;
        }
        self.list_state.select(Some(self.selected_scenario));
        self.selected_log = 0;
        self.update_log_index();
    }

    fn next_log(&mut self) {
        let logs_count = self.results[self.selected_scenario].logs.len();
        if logs_count > 0 {
            if self.selected_log < logs_count - 1 {
                self.selected_log += 1;
            } else {
                self.selected_log = 0;
            }
            self.log_list_state.select(Some(self.selected_log));
        }
    }

    fn previous_log(&mut self) {
        let logs_count = self.results[self.selected_scenario].logs.len();
        if logs_count > 0 {
            if self.selected_log > 0 {
                self.selected_log -= 1;
            } else {
                self.selected_log = logs_count - 1;
            }
            self.log_list_state.select(Some(self.selected_log));
        }
    }
}

pub async fn run() -> Result<()> {
    let config_dir = crate::config::get_config_dir()?;
    let personality_path = config_dir.join("personality.toml");
    let config = if personality_path.exists() {
        crate::config::personality::Config::load(personality_path)?
    } else {
        let default_toml = crate::config::personality::get_default_toml();
        toml::from_str(default_toml)?
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config);
    // Populate all results initially
    app.run_all_scenarios();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('r') | KeyCode::Enter => {
                        app.run_selected_scenario();
                    }
                    KeyCode::Char('a') => {
                        app.run_all_scenarios();
                    }
                    KeyCode::Char('m') | KeyCode::Char('f') | KeyCode::Tab => {
                        app.fullscreen_matrix = !app.fullscreen_matrix;
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        if !app.fullscreen_matrix {
                            app.focus = Focus::ScenarioList;
                        }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if !app.fullscreen_matrix {
                            app.focus = Focus::TimelineLog;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.fullscreen_matrix {
                            app.next_scenario();
                        } else {
                            match app.focus {
                                Focus::ScenarioList => app.next_scenario(),
                                Focus::TimelineLog => app.next_log(),
                            }
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.fullscreen_matrix {
                            app.previous_scenario();
                        } else {
                            match app.focus {
                                Focus::ScenarioList => app.previous_scenario(),
                                Focus::TimelineLog => app.previous_log(),
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // Main layout: Top area (Selectors & Details) & Bottom area (Scenarios Matrix)
    let (table_area, summary_area, help_area) = if app.fullscreen_matrix {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),
                Constraint::Length(3),  // Overall Summary panel
                Constraint::Length(1),  // Key help row
            ])
            .split(size);
        (chunks[0], chunks[1], chunks[2])
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),    // Top panel (Selectors & logs) gets remaining space
                Constraint::Length(14), // Matrix Table (expanded by default to show ~10 rows)
                Constraint::Length(3),  // Overall Summary panel
                Constraint::Length(1),  // Key help row
            ])
            .split(size);

        // Split top area horizontally: left 28% for scenarios list, right 72% for details
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(28),
                Constraint::Percentage(72),
            ])
            .split(chunks[0]);

        // Split right details area vertically: 62% for timeline logs list, 38% for character bubbles / stats
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(62),
                Constraint::Percentage(38),
            ])
            .split(top_chunks[1]);

        // 1. Draw Left Scenario List
        let list_border_color = if app.focus == Focus::ScenarioList { Color::Yellow } else { Color::DarkGray };
        let list_title = if app.focus == Focus::ScenarioList { "◀ SCENARIOS LIST ▶" } else { " SCENARIOS LIST " };

        let scenario_items: Vec<ListItem> = app.scenarios.iter().enumerate().map(|(idx, s)| {
            let is_run = !app.results[idx].logs.is_empty();
            let marker = if is_run { "✓ " } else { "  " };
            let style = if idx == app.selected_scenario {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Span::styled(format!("{}{:02}. {}", marker, idx + 1, s.name), style))
        }).collect();

        let scenario_list = List::new(scenario_items)
            .block(Block::default().borders(Borders::ALL).title(list_title).border_style(Style::default().fg(list_border_color)))
            .highlight_style(Style::default().bg(Color::Rgb(60, 60, 60)));

        f.render_stateful_widget(scenario_list, top_chunks[0], &mut app.list_state);

        // 2. Draw Right Top Timeline Logs List
        let log_border_color = if app.focus == Focus::TimelineLog { Color::Yellow } else { Color::DarkGray };
        let log_title = if app.focus == Focus::TimelineLog { "◀ TIMELINE EVENT LOGS ▶" } else { " TIMELINE EVENT LOGS " };

        let current_result = &app.results[app.selected_scenario];
        let log_rows: Vec<Row> = current_result.logs.iter().enumerate().map(|(log_idx, entry)| {
            let delta = if log_idx == 0 {
                0
            } else {
                (entry.time - current_result.logs[log_idx - 1].time).round() as i64
            };
            let time_str = format!("+{:>3}s", delta);
            let is_selected_log = app.focus == Focus::TimelineLog && log_idx == app.selected_log;
            let item_style = if is_selected_log {
                Style::default().bg(Color::Rgb(60, 60, 60))
            } else if entry.kind == LogKind::Speech {
                Style::default().bg(Color::Rgb(35, 35, 35))
            } else {
                Style::default()
            };

            if entry.kind == LogKind::Speech {
                let speech_text = format!("\"{}\"", entry.text);
                let mood_color = get_mood_color(&entry.mood);

                let speech_style = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
                let meta_style = Style::default().fg(mood_color).add_modifier(Modifier::BOLD);

                Row::new(vec![
                    Cell::new("🐮").style(meta_style),
                    Cell::new(time_str).style(meta_style),
                    Cell::new(speech_text).style(speech_style),
                ]).style(item_style)
            } else {
                let icon_style = Style::default().fg(Color::Rgb(200, 200, 200));
                let time_style = if entry.kind == LogKind::Cmd || entry.kind == LogKind::Error || entry.kind == LogKind::AppStart || entry.kind == LogKind::AppStop {
                    Style::default().fg(Color::Rgb(130, 200, 255))
                } else {
                    Style::default().fg(Color::Rgb(160, 160, 160))
                };

                let icon = match entry.kind {
                    LogKind::Cmd | LogKind::Error | LogKind::AppStart | LogKind::AppStop => "🧑",
                    _ => "🤖",
                };

                let cells = match entry.kind {
                    LogKind::Cmd => {
                        if let Some(cmd_part) = entry.text.strip_prefix("❯ ") {
                            vec![
                                Cell::new(icon).style(icon_style),
                                Cell::new(time_str.clone()).style(time_style),
                                Cell::new(Line::from(vec![
                                    Span::styled("❯ ", Style::default().fg(Color::Rgb(50, 255, 50)).add_modifier(Modifier::BOLD)),
                                    Span::styled(cmd_part.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                                ])),
                            ]
                        } else {
                            vec![
                                Cell::new(icon).style(icon_style),
                                Cell::new(time_str.clone()).style(time_style),
                                Cell::new(entry.text.as_str()).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                            ]
                        }
                    }
                    LogKind::Error => {
                        if let Some(cmd_part) = entry.text.strip_prefix("✗ ") {
                            vec![
                                Cell::new(icon).style(icon_style),
                                Cell::new(time_str.clone()).style(time_style),
                                Cell::new(Line::from(vec![
                                    Span::styled("✗ ", Style::default().fg(Color::Rgb(255, 80, 80)).add_modifier(Modifier::BOLD)),
                                    Span::styled(cmd_part.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                                ])),
                            ]
                        } else {
                            vec![
                                Cell::new(icon).style(icon_style),
                                Cell::new(time_str.clone()).style(time_style),
                                Cell::new(entry.text.as_str()).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                            ]
                        }
                    }
                    LogKind::AppStart | LogKind::AppStop => {
                        let text = &entry.text;
                        if text.contains("Started -> ") {
                            let parts: Vec<&str> = text.split("Started -> ").collect();
                            vec![
                                Cell::new(icon).style(icon_style),
                                Cell::new(time_str.clone()).style(time_style),
                                Cell::new(Line::from(vec![
                                    Span::styled(format!("{}Started -> ", parts[0]), Style::default().fg(Color::Rgb(200, 200, 200))),
                                    Span::styled(parts[1].to_string(), Style::default().fg(Color::Rgb(0, 210, 255)).add_modifier(Modifier::BOLD)),
                                ])),
                            ]
                        } else if text.contains("Stopped -> ") {
                            let parts: Vec<&str> = text.split("Stopped -> ").collect();
                            vec![
                                Cell::new(icon).style(icon_style),
                                Cell::new(time_str.clone()).style(time_style),
                                Cell::new(Line::from(vec![
                                    Span::styled(format!("{}Stopped -> ", parts[0]), Style::default().fg(Color::Rgb(200, 200, 200))),
                                    Span::styled(parts[1].to_string(), Style::default().fg(Color::Rgb(0, 210, 255)).add_modifier(Modifier::BOLD)),
                                ])),
                            ]
                        } else {
                            vec![
                                Cell::new(icon).style(icon_style),
                                Cell::new(time_str.clone()).style(time_style),
                                Cell::new(text.as_str()).style(Style::default().fg(Color::Rgb(220, 220, 220))),
                            ]
                        }
                    }
                    LogKind::System => {
                        let text = &entry.text;
                        if text.contains("Tick event -> ") {
                            let parts: Vec<&str> = text.split("Tick event -> ").collect();
                            vec![
                                Cell::new(icon).style(icon_style),
                                Cell::new(time_str.clone()).style(time_style),
                                Cell::new(Line::from(vec![
                                    Span::styled(format!("{}Tick event -> ", parts[0]), Style::default().fg(Color::Rgb(180, 180, 180))),
                                    Span::styled(parts[1].to_string(), Style::default().fg(Color::Rgb(255, 100, 255)).add_modifier(Modifier::BOLD)),
                                ])),
                            ]
                        } else {
                            vec![
                                Cell::new(icon).style(icon_style),
                                Cell::new(time_str.clone()).style(time_style),
                                Cell::new(text.as_str()).style(Style::default().fg(Color::Rgb(200, 200, 200))),
                            ]
                        }
                    }
                    LogKind::Idle => {
                        vec![
                            Cell::new(icon).style(icon_style),
                            Cell::new(time_str.clone()).style(time_style),
                            Cell::new(entry.text.as_str()).style(Style::default().fg(Color::Rgb(160, 160, 160)).add_modifier(Modifier::ITALIC)),
                        ]
                    }
                    _ => {
                        vec![
                            Cell::new(icon).style(icon_style),
                            Cell::new(time_str.clone()).style(time_style),
                            Cell::new(entry.text.as_str()).style(Style::default().fg(Color::Rgb(220, 220, 220))),
                        ]
                    }
                };

                Row::new(cells).style(item_style)
            }
        }).collect();

        let logs_table = Table::new(log_rows, [
            Constraint::Length(3),
            Constraint::Length(6),
            Constraint::Min(20),
        ])
        .block(Block::default().borders(Borders::ALL).title(log_title).border_style(Style::default().fg(log_border_color)))
        .column_spacing(1);

        f.render_stateful_widget(logs_table, right_chunks[0], &mut app.log_list_state);

        // 3. Draw Right Bottom Details: Description, Stats Card & Speech Bubble / Cow
        let detail_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(55), // Description & bubble info
                Constraint::Percentage(45), // Stats card
            ])
            .split(right_chunks[1]);

        // Check if the currently highlighted log item in timeline is a speech log
        let mut selected_speech = None;
        if !current_result.logs.is_empty() && app.selected_log < current_result.logs.len() {
            let entry = &current_result.logs[app.selected_log];
            if entry.kind == LogKind::Speech {
                selected_speech = Some(entry);
            }
        }

        // 3A. Draw Speech bubble & Cow OR Description
        let details_block = Block::default().borders(Borders::ALL).title(" SCENARIO DETAILS ").border_style(Style::default().fg(Color::DarkGray));

        if let Some(speech) = selected_speech {
            let bubble_color = get_mood_color(&speech.mood);
            let bubble_style = Style::default().fg(bubble_color).add_modifier(Modifier::BOLD);

            let cow_text = r#"      \   ^__^
       \  (oo)\_______
          (__)\       )\/\
              ||----w |
              ||     ||"#;

            let content = format!(
                "Speech [Mood: {}]:\n\"{}\"\n{}",
                speech.mood, speech.text, cow_text
            );

            let speech_p = Paragraph::new(content)
                .block(details_block)
                .style(bubble_style)
                .wrap(Wrap { trim: true });
            f.render_widget(speech_p, detail_chunks[0]);
        } else {
            let desc = app.scenarios[app.selected_scenario].description;
            let content = format!("Scenario Description:\n{}\n\n(Tip: Focus the timeline logs and hover a Speech event to see the cow output)", desc);
            let desc_p = Paragraph::new(content)
                .block(details_block)
                .style(Style::default().fg(Color::White))
                .wrap(Wrap { trim: true });
            f.render_widget(desc_p, detail_chunks[0]);
        }

        // 3B. Draw Stats Card for selected scenario
        let metrics = &current_result.metrics;
        let stats_text = format!(
            "Duration:    {:>6.1}s   AppTime:   {:>6.1}s\n\
             Cmds Run:    {:>6}   Ex/Sec:     {:>6.3}\n\
             Triggers:    {:>6}   Avg/Sec:    {:>6.3}\n\
             Idle Trigs:  {:>6}   MaxPause:  {:>6.1}s",
            metrics.duration, metrics.app_time,
            metrics.cmds, metrics.avg_sec_excl,
            metrics.trig, metrics.avg_sec,
            metrics.idle, metrics.longest_pause
        );

        let stats_p = Paragraph::new(stats_text)
            .block(Block::default().borders(Borders::ALL).title(" SCENARIO STATS ").border_style(Style::default().fg(Color::DarkGray)))
            .style(Style::default().fg(Color::LightGreen));
        f.render_widget(stats_p, detail_chunks[1]);

        (chunks[1], chunks[2], chunks[3])
    };

    // 4. Draw Bottom Scenarios Matrix Table
    let min_max = compute_min_max(&app.results);
    let mut rows = Vec::new();

    for (idx, r) in app.results.iter().enumerate() {
        let m = &r.metrics;
        let is_selected = idx == app.selected_scenario;

        let c_cmds = get_color_gradient(m.cmds as f64, min_max.min_cmds as f64, min_max.max_cmds as f64, true);
        let c_trig = get_color_gradient(m.trig as f64, min_max.min_trig as f64, min_max.max_trig as f64, false);
        let c_idle = get_color_gradient(m.idle as f64, min_max.min_idle as f64, min_max.max_idle as f64, false);
        let c_rate = get_color_gradient(m.trig_rate, min_max.min_rate, min_max.max_rate, false);
        let c_app = get_color_gradient(m.app_time, min_max.min_app, min_max.max_app, false);
        let c_avg = get_color_gradient(m.avg_sec, min_max.min_avg, min_max.max_avg, false);
        let c_excl = get_color_gradient(m.avg_sec_excl, min_max.min_excl, min_max.max_excl, false);
        let c_pause = get_color_gradient(m.longest_pause, min_max.min_pause, min_max.max_pause, true);

        let fg_cmds = get_contrast_fg(c_cmds);
        let fg_trig = get_contrast_fg(c_trig);
        let fg_idle = get_contrast_fg(c_idle);
        let fg_rate = get_contrast_fg(c_rate);
        let fg_app = get_contrast_fg(c_app);
        let fg_avg = get_contrast_fg(c_avg);
        let fg_excl = get_contrast_fg(c_excl);
        let fg_pause = get_contrast_fg(c_pause);

        let row_style = if is_selected {
            Style::default().add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default()
        };

        rows.push(Row::new(vec![
            Cell::new(format!("{:>2} ", idx + 1)).style(Style::default().fg(Color::Cyan)),
            Cell::new(format!(" {}", app.scenarios[idx].name)).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Cell::new(format!(" {:^3} ", m.cmds)).style(Style::default().fg(fg_cmds).bg(c_cmds).add_modifier(Modifier::BOLD)),
            Cell::new(format!(" {:^3} ", m.trig)).style(Style::default().fg(fg_trig).bg(c_trig).add_modifier(Modifier::BOLD)),
            Cell::new(format!(" {:^3} ", m.idle)).style(Style::default().fg(fg_idle).bg(c_idle).add_modifier(Modifier::BOLD)),
            Cell::new(format!(" {:^5.1}% ", m.trig_rate)).style(Style::default().fg(fg_rate).bg(c_rate).add_modifier(Modifier::BOLD)),
            Cell::new(format!(" {:^6.1}s ", m.app_time)).style(Style::default().fg(fg_app).bg(c_app).add_modifier(Modifier::BOLD)),
            Cell::new(format!(" {:^6.3} ", m.avg_sec)).style(Style::default().fg(fg_avg).bg(c_avg).add_modifier(Modifier::BOLD)),
            Cell::new(format!(" {:^6.3} ", m.avg_sec_excl)).style(Style::default().fg(fg_excl).bg(c_excl).add_modifier(Modifier::BOLD)),
            Cell::new(format!(" {:^7.1}s ", m.longest_pause)).style(Style::default().fg(fg_pause).bg(c_pause).add_modifier(Modifier::BOLD)),
        ]).style(row_style));
    }

    let headers = Row::new(vec![
        Cell::new("ID").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::new("Scenario Name").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Cell::new("Cmds").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Cell::new("Trig").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Cell::new("Idle").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Cell::new("Trig%").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Cell::new("AppTime").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Cell::new("Avg/Sec").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Cell::new("Ex/Sec").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Cell::new("MaxPause").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    ]);

    let table = Table::new(rows, [
        Constraint::Length(4),
        Constraint::Length(35),
        Constraint::Length(7),
        Constraint::Length(7),
        Constraint::Length(7),
        Constraint::Length(9),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(11),
    ])
    .header(headers)
    .block(Block::default().borders(Borders::ALL).title(" PERSONALITY ENGINE SCENARIOS MATRIX ").border_style(Style::default().fg(Color::DarkGray)))
    .column_spacing(1);

    let mut table_state = TableState::default();
    table_state.select(Some(app.selected_scenario));
    f.render_stateful_widget(table, table_area, &mut table_state);

    // 5. Draw Bottom Overall Summary Panel
    let mut total_cmds = 0;
    let mut total_trig = 0;
    let mut total_idle = 0;
    let mut total_duration = 0.0;
    let mut total_app_time = 0.0;
    let mut max_pause_overall = 0.0;

    for r in &app.results {
        let m = &r.metrics;
        total_cmds += m.cmds;
        total_trig += m.trig;
        total_idle += m.idle;
        total_duration += m.duration;
        total_app_time += m.app_time;
        if m.longest_pause > max_pause_overall {
            max_pause_overall = m.longest_pause;
        }
    }

    let total_interjects = total_trig + total_idle;
    let overall_trig_rate = if total_cmds > 0 {
        (total_trig as f64 / total_cmds as f64) * 100.0
    } else {
        0.0
    };
    let overall_avg_sec = if total_duration > 0.0 {
        total_interjects as f64 / total_duration
    } else {
        0.0
    };
    let overall_excl_dur = total_duration - total_app_time;
    let overall_avg_sec_excl = if overall_excl_dur > 0.0 {
        total_interjects as f64 / overall_excl_dur
    } else {
        0.0
    };

    let summary_text = format!(
        "Total Commands: {}  |  Triggered Interjections: {}  |  Idle Interjections: {}  |  In-App Time: {:.1}s\n\
         Combined Trig Rate: {:.1}%  |  Combined Avg/Sec: {:.4}  |  Avg/Sec (excl in-app): {:.4}  |  Max Silence Overall: {:.1}s",
        total_cmds, total_trig, total_idle, total_app_time,
        overall_trig_rate, overall_avg_sec, overall_avg_sec_excl, max_pause_overall
    );

    let summary_p = Paragraph::new(summary_text)
        .block(Block::default().borders(Borders::ALL).title(" OVERALL SUMMARY STATISTICS ").border_style(Style::default().fg(Color::DarkGray)))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(summary_p, summary_area);

    // 6. Draw Bottom Key Help Row
    let help_line = if app.fullscreen_matrix {
        Line::from(vec![
            Span::styled(" Esc/q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(" Quit ", Style::default().fg(Color::White)),
            Span::styled(" Up/Down/j/k", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" Navigate Rows ", Style::default().fg(Color::White)),
            Span::styled(" m/f/Tab", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" Exit Fullscreen", Style::default().fg(Color::White)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" Esc/q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(" Quit ", Style::default().fg(Color::White)),
            Span::styled(" Up/Down/j/k", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" Navigate ", Style::default().fg(Color::White)),
            Span::styled(" Left/Right/h/l", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" Switch Focus ", Style::default().fg(Color::White)),
            Span::styled(" r/Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" Run Selection ", Style::default().fg(Color::White)),
            Span::styled(" a", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" Run All ", Style::default().fg(Color::White)),
            Span::styled(" m/f/Tab", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" Fullscreen Matrix", Style::default().fg(Color::White)),
        ])
    };
    let help_p = Paragraph::new(help_line).style(Style::default().fg(Color::Gray));
    f.render_widget(help_p, help_area);
}

fn get_mood_color(mood: &str) -> Color {
    match mood {
        "energetic" => Color::Yellow,
        "sympathetic" => Color::Magenta,
        "sleepy" => Color::Rgb(128, 128, 128),
        "bored" => Color::Rgb(180, 180, 180),
        _ => Color::Cyan,
    }
}

fn get_color_gradient(val: f64, min: f64, max: f64, invert: bool) -> Color {
    if max == min {
        return Color::Rgb(0, 255, 0);
    }
    let mut ratio = (val - min) / (max - min);
    if invert {
        ratio = 1.0 - ratio;
    }
    let ratio = ratio.max(0.0).min(1.0);

    let (r, g, b) = if ratio <= 0.5 {
        let factor = ratio / 0.5;
        ((255.0 * factor) as u8, 255, 0)
    } else {
        let factor = (ratio - 0.5) / 0.5;
        (255, (255.0 * (1.0 - factor)) as u8, 0)
    };
    Color::Rgb(r, g, b)
}

fn get_contrast_fg(bg: Color) -> Color {
    match bg {
        Color::Rgb(r, g, b) => {
            let luminance = 0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64;
            if luminance > 128.0 {
                Color::Rgb(0, 0, 0)
            } else {
                Color::Rgb(255, 255, 255)
            }
        }
        _ => Color::Rgb(255, 255, 255),
    }
}


fn evaluate_rules(
    event_type: &str,
    event_data: Option<&str>,
    state: &SimulatedState,
    config: &PersonalityConfig,
) -> Option<(String, Option<String>)> {
    let cooldown = config.timing.message_cooldown_secs as f64;
    if event_type != "app_start" && event_type != "app_stop" && event_type != "session_start"
        && (state.current_time - state.last_message_time) < cooldown
    {
        return None;
    }

    for rule in &config.rules {
        let trigger = &rule.trigger;
        let mut trigger_matched = false;

        match event_type {
            "session_start" => {
                if trigger == "session_start" {
                    trigger_matched = true;
                }
            }
            "idle" => {
                if trigger == "idle" {
                    trigger_matched = true;
                }
            }
            "sleep" => {
                if trigger == "sleep" {
                    trigger_matched = true;
                }
            }
            "wake" => {
                if trigger == "wake" {
                    trigger_matched = true;
                }
            }
            "app_stop" => {
                if trigger == "app_stop" {
                    trigger_matched = true;
                }
            }
            "app_start" => {
                if let Some(app) = event_data {
                    if trigger == "app_start" {
                        trigger_matched = true;
                    } else if trigger.starts_with("app_start:") {
                        let pattern = trigger.split_at(10).1;
                        if app == pattern {
                            trigger_matched = true;
                        }
                    }
                }
            }
            "exec" => {
                if let Some(cmd) = event_data {
                    if trigger == "random" {
                        trigger_matched = true;
                    } else if trigger.starts_with("exec:") {
                        let pattern = trigger.split_at(5).1;
                        let cmd_word = cmd.split_whitespace().next().unwrap_or("");
                        let pat_word = pattern.split_whitespace().next().unwrap_or("");
                        if cmd_word == pat_word {
                            trigger_matched = true;
                        }
                    } else if trigger == "milestone" {
                        let cmd_word = cmd.split_whitespace().next().unwrap_or("").to_string();
                        let count = state.command_counts.get(&cmd_word).copied().unwrap_or(0);
                        if [1, 5, 10, 25, 50, 100, 123, 500, 1000].contains(&count) {
                            trigger_matched = true;
                        }
                    }
                }
            }
            "cd" => {
                if let Some(dir_path) = event_data {
                    if trigger == "random" {
                        trigger_matched = true;
                    } else if trigger.starts_with("cd:") {
                        let pattern = trigger.split_at(3).1;
                        if pattern == "~" && (dir_path == "/home/kid" || dir_path == "~" || dir_path == "/home/kid/") {
                            trigger_matched = true;
                        } else if pattern.starts_with('*') {
                            let suffix = &pattern[1..];
                            if dir_path.ends_with(suffix) {
                                trigger_matched = true;
                            }
                        } else if dir_path == pattern {
                            trigger_matched = true;
                        }
                    }
                }
            }
            "tick" => {
                if trigger == "random" {
                    trigger_matched = true;
                }
            }
            _ => {}
        }

        if !trigger_matched {
            continue;
        }

        if let Some(req_mood) = &rule.conditions.mood {
            if req_mood != &state.mood {
                continue;
            }
        }

        if let Some(min_count) = rule.conditions.min_count {
            if event_type == "exec" {
                if let Some(cmd) = event_data {
                    let cmd_word = cmd.split_whitespace().next().unwrap_or("");
                    let count = state.command_counts.get(cmd_word).copied().unwrap_or(0);
                    if count != min_count {
                        continue;
                    }
                } else {
                    continue;
                }
            } else {
                continue;
            }
        }

        if let Some(streak) = rule.conditions.streak {
            if (state.error_streak as u64) < streak as u64 {
                continue;
            }
        }

        if let Some(chance) = rule.conditions.chance {
            if rand::random::<f32>() > chance {
                continue;
            }
        }

        if rule.responses.is_empty() {
            continue;
        }

        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        if let Some(raw_response) = rule.responses.choose(&mut rng) {
            let mut response = raw_response.clone();
            if event_type == "exec" || event_type == "app_start" {
                if let Some(cmd) = event_data {
                    let cmd_word = cmd.split_whitespace().next().unwrap_or("");
                    let count = state.command_counts.get(cmd_word).copied().unwrap_or(0);
                    response = response.replace("{cmd}", cmd_word);
                    response = response.replace("{count}", &count.to_string());
                }
            } else if event_type == "cd" {
                if let Some(dir) = event_data {
                    response = response.replace("{dir}", dir);
                }
            }

            return Some((response, rule.set_mood.clone()));
        }
    }

    None
}

fn run_scenario(
    scenario: &Scenario,
    config: &PersonalityConfig,
) -> (ScenarioMetrics, Vec<LogEntry>) {
    let default_mood = config.mood.default.clone();
    let mut state = SimulatedState {
        mood: default_mood.clone(),
        default_mood,
        command_counts: HashMap::new(),
        last_message_time: -999.0,
        last_activity_time: 0.0,
        mood_last_set: 0.0,
        is_sleeping: false,
        error_streak: 0,
        current_time: 0.0,
        active_app: None,
    };

    let mut total_commands = 0;
    let mut triggered_interjections = 0;
    let mut idle_interjections = 0;
    let mut in_app_duration = 0.0;
    let mut interjection_times = Vec::new();
    let mut logs = Vec::new();

    // Trigger session start welcome message!
    logs.push(LogEntry {
        time: 0.0,
        kind: LogKind::System,
        text: "System: Session started".to_string(),
        mood: state.mood.clone(),
    });
    if let Some((msg, set_mood)) = evaluate_rules("session_start", None, &state, config) {
        triggered_interjections += 1;
        interjection_times.push(0.0);
        state.last_message_time = 0.0;
        if let Some(m) = set_mood {
            state.mood = m;
            state.mood_last_set = 0.0;
        }
        logs.push(LogEntry {
            time: 0.0,
            kind: LogKind::Speech,
            text: msg,
            mood: state.mood.clone(),
        });
    }

    for event in &scenario.events {
        match event {
            SimulatedEvent::Cmd(cmd, cwd) => {
                let mood_decay = config.timing.mood_decay_secs as f64;
                if state.mood != state.default_mood && (state.current_time - state.mood_last_set) > mood_decay {
                    state.mood = state.default_mood.clone();
                }

                logs.push(LogEntry {
                    time: state.current_time,
                    kind: LogKind::Cmd,
                    text: format!("❯ {} (cwd: {})", cmd, cwd),
                    mood: state.mood.clone(),
                });

                state.last_activity_time = state.current_time;
                if state.is_sleeping {
                    state.is_sleeping = false;
                    logs.push(LogEntry {
                        time: state.current_time,
                        kind: LogKind::System,
                        text: "System: Wake event triggered".to_string(),
                        mood: state.mood.clone(),
                    });

                    if let Some((msg, set_mood)) = evaluate_rules("wake", None, &state, config) {
                        triggered_interjections += 1;
                        interjection_times.push(state.current_time);
                        state.last_message_time = state.current_time;
                        if let Some(m) = set_mood {
                            state.mood = m;
                            state.mood_last_set = state.current_time;
                        }
                        logs.push(LogEntry {
                            time: state.current_time,
                            kind: LogKind::Speech,
                            text: msg,
                            mood: state.mood.clone(),
                        });
                    }
                }

                let cmd_word = cmd.split_whitespace().next().unwrap_or("").to_string();
                state.error_streak = 0;
                *state.command_counts.entry(cmd_word).or_insert(0) += 1;
                total_commands += 1;

                let rule_event = if cmd.starts_with("cd ") || *cmd == "cd" { "cd" } else { "exec" };
                if let Some((msg, set_mood)) = evaluate_rules(rule_event, Some(cmd), &state, config) {
                    triggered_interjections += 1;
                    interjection_times.push(state.current_time);
                    state.last_message_time = state.current_time;
                    if let Some(m) = set_mood {
                        state.mood = m;
                        state.mood_last_set = state.current_time;
                    }
                    logs.push(LogEntry {
                        time: state.current_time,
                        kind: LogKind::Speech,
                        text: msg,
                        mood: state.mood.clone(),
                    });
                }
            }
            SimulatedEvent::Error(cmd, cwd) => {
                let mood_decay = config.timing.mood_decay_secs as f64;
                if state.mood != state.default_mood && (state.current_time - state.mood_last_set) > mood_decay {
                    state.mood = state.default_mood.clone();
                }

                logs.push(LogEntry {
                    time: state.current_time,
                    kind: LogKind::Error,
                    text: format!("✗ {} (cwd: {})", cmd, cwd),
                    mood: state.mood.clone(),
                });

                state.last_activity_time = state.current_time;
                if state.is_sleeping {
                    state.is_sleeping = false;
                    logs.push(LogEntry {
                        time: state.current_time,
                        kind: LogKind::System,
                        text: "System: Wake event triggered".to_string(),
                        mood: state.mood.clone(),
                    });

                    if let Some((msg, set_mood)) = evaluate_rules("wake", None, &state, config) {
                        triggered_interjections += 1;
                        interjection_times.push(state.current_time);
                        state.last_message_time = state.current_time;
                        if let Some(m) = set_mood {
                            state.mood = m;
                            state.mood_last_set = state.current_time;
                        }
                        logs.push(LogEntry {
                            time: state.current_time,
                            kind: LogKind::Speech,
                            text: msg,
                            mood: state.mood.clone(),
                        });
                    }
                }

                state.error_streak += 1;
                total_commands += 1;

                if let Some((msg, set_mood)) = evaluate_rules("exec", Some(cmd), &state, config) {
                    triggered_interjections += 1;
                    interjection_times.push(state.current_time);
                    state.last_message_time = state.current_time;
                    if let Some(m) = set_mood {
                        state.mood = m;
                        state.mood_last_set = state.current_time;
                    }
                    logs.push(LogEntry {
                        time: state.current_time,
                        kind: LogKind::Speech,
                        text: msg,
                        mood: state.mood.clone(),
                    });
                }
            }
            SimulatedEvent::AppStart(app, cwd) => {
                state.active_app = Some(app.to_string());
                logs.push(LogEntry {
                    time: state.current_time,
                    kind: LogKind::AppStart,
                    text: format!("System: GUI App Started -> {} (cwd: {})", app, cwd),
                    mood: state.mood.clone(),
                });

                if let Some((msg, set_mood)) = evaluate_rules("app_start", Some(app), &state, config) {
                    triggered_interjections += 1;
                    interjection_times.push(state.current_time);
                    state.last_message_time = state.current_time;
                    if let Some(m) = set_mood {
                        state.mood = m;
                        state.mood_last_set = state.current_time;
                    }
                    logs.push(LogEntry {
                        time: state.current_time,
                        kind: LogKind::Speech,
                        text: msg,
                        mood: state.mood.clone(),
                    });
                }
            }
            SimulatedEvent::AppStop(app, cwd) => {
                state.active_app = None;
                logs.push(LogEntry {
                    time: state.current_time,
                    kind: LogKind::AppStop,
                    text: format!("System: GUI App Stopped -> {} (cwd: {})", app, cwd),
                    mood: state.mood.clone(),
                });

                if let Some((msg, set_mood)) = evaluate_rules("app_stop", Some(app), &state, config) {
                    triggered_interjections += 1;
                    interjection_times.push(state.current_time);
                    state.last_message_time = state.current_time;
                    if let Some(m) = set_mood {
                        state.mood = m;
                        state.mood_last_set = state.current_time;
                    }
                    logs.push(LogEntry {
                        time: state.current_time,
                        kind: LogKind::Speech,
                        text: msg,
                        mood: state.mood.clone(),
                    });
                }
            }
            SimulatedEvent::Idle(duration) => {
                let target_time = state.current_time + (*duration as f64);
                logs.push(LogEntry {
                    time: state.current_time,
                    kind: LogKind::Idle,
                    text: format!("... waiting / idle for {}s ...", duration),
                    mood: state.mood.clone(),
                });

                while state.current_time < target_time {
                    let step = (20.0f64).min(target_time - state.current_time);
                    state.current_time += step;
                    if state.active_app.is_some() {
                        in_app_duration += step;
                    }

                    let mood_decay = config.timing.mood_decay_secs as f64;
                    if state.mood != state.default_mood && (state.current_time - state.mood_last_set) > mood_decay {
                        state.mood = state.default_mood.clone();
                    }

                    if state.active_app.is_none() {
                        let idle_secs = state.current_time - state.last_activity_time;
                        let sleep_thresh = config.timing.sleep_threshold_secs as f64;
                        let idle_thresh = config.timing.idle_threshold_secs as f64;

                        let mut tick_event = None;
                        if idle_secs >= sleep_thresh && !state.is_sleeping {
                            state.is_sleeping = true;
                            tick_event = Some("sleep");
                        } else if idle_secs >= idle_thresh && !state.is_sleeping {
                            tick_event = Some("idle");
                        }

                        if let Some(te) = tick_event {
                            logs.push(LogEntry {
                                time: state.current_time,
                                kind: LogKind::System,
                                text: format!("System: Tick event -> {}", te),
                                mood: state.mood.clone(),
                            });

                            if let Some((msg, set_mood)) = evaluate_rules(te, None, &state, config) {
                                idle_interjections += 1;
                                interjection_times.push(state.current_time);
                                state.last_message_time = state.current_time;
                                if let Some(m) = set_mood {
                                    state.mood = m;
                                    state.mood_last_set = state.current_time;
                                }
                                logs.push(LogEntry {
                                    time: state.current_time,
                                    kind: LogKind::Speech,
                                    text: msg,
                                    mood: state.mood.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    let total_interjections = triggered_interjections + idle_interjections;
    let duration = state.current_time;
    let longest_pause = if !interjection_times.is_empty() {
        let mut gaps = vec![interjection_times[0]];
        for i in 1..interjection_times.len() {
            gaps.push(interjection_times[i] - interjection_times[i - 1]);
        }
        gaps.push(duration - interjection_times.last().unwrap());
        gaps.into_iter().fold(0.0, f64::max)
    } else {
        duration
    };

    let trig_rate = if total_commands > 0 {
        (triggered_interjections as f64 / total_commands as f64) * 100.0
    } else {
        0.0
    };

    let avg_sec = if duration > 0.0 {
        total_interjections as f64 / duration
    } else {
        0.0
    };

    let excl_duration = duration - in_app_duration;
    let avg_sec_excl = if excl_duration > 0.0 {
        total_interjections as f64 / excl_duration
    } else {
        0.0
    };

    (
        ScenarioMetrics {
            cmds: total_commands,
            trig: triggered_interjections,
            idle: idle_interjections,
            duration,
            longest_pause,
            app_time: in_app_duration,
            trig_rate,
            avg_sec,
            avg_sec_excl,
        },
        logs,
    )
}

fn compute_min_max(results: &[RunResult]) -> ColumnMinMax {
    let mut min_cmds = u64::MAX;
    let mut max_cmds = u64::MIN;
    let mut min_trig = u64::MAX;
    let mut max_trig = u64::MIN;
    let mut min_idle = u64::MAX;
    let mut max_idle = u64::MIN;

    let mut min_rate = f64::MAX;
    let mut max_rate = f64::MIN;
    let mut min_app = f64::MAX;
    let mut max_app = f64::MIN;
    let mut min_avg = f64::MAX;
    let mut max_avg = f64::MIN;
    let mut min_excl = f64::MAX;
    let mut max_excl = f64::MIN;
    let mut min_pause = f64::MAX;
    let mut max_pause = f64::MIN;

    for r in results {
        let m = &r.metrics;
        min_cmds = min_cmds.min(m.cmds);
        max_cmds = max_cmds.max(m.cmds);
        min_trig = min_trig.min(m.trig);
        max_trig = max_trig.max(m.trig);
        min_idle = min_idle.min(m.idle);
        max_idle = max_idle.max(m.idle);

        min_rate = min_rate.min(m.trig_rate);
        max_rate = max_rate.max(m.trig_rate);
        min_app = min_app.min(m.app_time);
        max_app = max_app.max(m.app_time);
        min_avg = min_avg.min(m.avg_sec);
        max_avg = max_avg.max(m.avg_sec);
        min_excl = min_excl.min(m.avg_sec_excl);
        max_excl = max_excl.max(m.avg_sec_excl);
        min_pause = min_pause.min(m.longest_pause);
        max_pause = max_pause.max(m.longest_pause);
    }

    ColumnMinMax {
        min_cmds, max_cmds,
        min_trig, max_trig,
        min_idle, max_idle,
        min_rate, max_rate,
        min_app, max_app,
        min_avg, max_avg,
        min_excl, max_excl,
        min_pause, max_pause,
    }
}

struct ColumnMinMax {
    min_cmds: u64,
    max_cmds: u64,
    min_trig: u64,
    max_trig: u64,
    min_idle: u64,
    max_idle: u64,
    min_rate: f64,
    max_rate: f64,
    min_app: f64,
    max_app: f64,
    min_avg: f64,
    max_avg: f64,
    min_excl: f64,
    max_excl: f64,
    min_pause: f64,
    max_pause: f64,
}

fn get_scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "First Day (New User)",
            description: "A new kid logs in for the first time, explores a bit, and launches Tux Paint.",
            events: vec![
                SimulatedEvent::Cmd("ls", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("cd apps", "/home/kid/apps"),
                SimulatedEvent::Idle(8),
                SimulatedEvent::Cmd("ls", "/home/kid/apps"),
                SimulatedEvent::Cmd("tuxpaint", "/home/kid/apps"),
                SimulatedEvent::AppStart("tuxpaint", "/home/kid/apps"),
                SimulatedEvent::Idle(120),
                SimulatedEvent::AppStop("tuxpaint", "/home/kid/apps"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("ls", "/home/kid/apps"),
            ],
        },
        Scenario {
            name: "Lost & Confused (Error Streak)",
            description: "User struggles, typing incorrect commands in a row, then finally runs help to recover.",
            events: vec![
                SimulatedEvent::Error("open paint", "/home/kid"),
                SimulatedEvent::Idle(4),
                SimulatedEvent::Error("what is this", "/home/kid"),
                SimulatedEvent::Idle(6),
                SimulatedEvent::Error("asdf", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Error("helpme", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("help", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("ls", "/home/kid"),
            ],
        },
        Scenario {
            name: "Hyperactive Kid (Spamming)",
            description: "User is super excited and rapidly types commands in quick succession.",
            events: vec![
                SimulatedEvent::Cmd("ls", "/home/kid"),
                SimulatedEvent::Idle(1),
                SimulatedEvent::Cmd("clear", "/home/kid"),
                SimulatedEvent::Idle(2),
                SimulatedEvent::Cmd("whoami", "/home/kid"),
                SimulatedEvent::Idle(1),
                SimulatedEvent::Cmd("cal", "/home/kid"),
                SimulatedEvent::Idle(2),
                SimulatedEvent::Cmd("date", "/home/kid"),
                SimulatedEvent::Idle(1),
                SimulatedEvent::Cmd("clear", "/home/kid"),
                SimulatedEvent::Idle(1),
                SimulatedEvent::Cmd("ls", "/home/kid"),
                SimulatedEvent::Idle(2),
                SimulatedEvent::Cmd("whoami", "/home/kid"),
            ],
        },
        Scenario {
            name: "Napping Companion (Idle to Sleep)",
            description: "User runs a command, then walks away. The companion goes idle, then falls asleep.",
            events: vec![
                SimulatedEvent::Cmd("ls", "/home/kid"),
                SimulatedEvent::Idle(20),
                SimulatedEvent::Idle(30),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("whoami", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("clear", "/home/kid"),
            ],
        },
        Scenario {
            name: "App Hopping",
            description: "User opens and closes educational apps one after another.",
            events: vec![
                SimulatedEvent::Cmd("tuxpaint", "/home/kid"),
                SimulatedEvent::AppStart("tuxpaint", "/home/kid"),
                SimulatedEvent::Idle(80),
                SimulatedEvent::AppStop("tuxpaint", "/home/kid"),
                SimulatedEvent::Idle(15),
                SimulatedEvent::Cmd("gcompris", "/home/kid"),
                SimulatedEvent::AppStart("gcompris", "/home/kid"),
                SimulatedEvent::Idle(200),
                SimulatedEvent::AppStop("gcompris", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("ls", "/home/kid"),
            ],
        },
        Scenario {
            name: "Deep Explorer",
            description: "User carefully navigates the directories and lists files.",
            events: vec![
                SimulatedEvent::Cmd("pwd", "/home/kid"),
                SimulatedEvent::Idle(8),
                SimulatedEvent::Cmd("cd apps", "/home/kid/apps"),
                SimulatedEvent::Idle(12),
                SimulatedEvent::Cmd("ls", "/home/kid/apps"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("cd ..", "/home/kid"),
                SimulatedEvent::Idle(6),
                SimulatedEvent::Cmd("cd creations", "/home/kid/creations"),
                SimulatedEvent::Idle(15),
                SimulatedEvent::Cmd("ls", "/home/kid/creations"),
            ],
        },
        Scenario {
            name: "The Echo Game",
            description: "User discovers the echo command and experiments with printing various words.",
            events: vec![
                SimulatedEvent::Cmd("echo hello", "/home/kid"),
                SimulatedEvent::Idle(4),
                SimulatedEvent::Cmd("echo cow", "/home/kid"),
                SimulatedEvent::Idle(3),
                SimulatedEvent::Cmd("echo coding", "/home/kid"),
                SimulatedEvent::Idle(6),
                SimulatedEvent::Cmd("echo cool", "/home/kid"),
                SimulatedEvent::Idle(4),
                SimulatedEvent::Cmd("clear", "/home/kid"),
            ],
        },
        Scenario {
            name: "Time Traveler",
            description: "User explores the date and calendar commands.",
            events: vec![
                SimulatedEvent::Cmd("date", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("cal", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("date", "/home/kid"),
                SimulatedEvent::Idle(4),
                SimulatedEvent::Cmd("cal", "/home/kid"),
                SimulatedEvent::Idle(8),
            ],
        },
        Scenario {
            name: "Creating Art (Creations)",
            description: "User creates a text file inside their creations directory.",
            events: vec![
                SimulatedEvent::Cmd("cd creations", "/home/kid/creations"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("touch picture.txt", "/home/kid/creations"),
                SimulatedEvent::Idle(8),
                SimulatedEvent::Cmd("ls", "/home/kid/creations"),
                SimulatedEvent::Idle(12),
                SimulatedEvent::Cmd("mkdir folder1", "/home/kid/creations"),
                SimulatedEvent::Idle(6),
                SimulatedEvent::Cmd("mv picture.txt folder1", "/home/kid/creations"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("cd folder1", "/home/kid/creations/folder1"),
                SimulatedEvent::Cmd("ls", "/home/kid/creations/folder1"),
            ],
        },
        Scenario {
            name: "Reading & Writing Stories",
            description: "User creates a story using nano, then reads it back.",
            events: vec![
                SimulatedEvent::Cmd("touch story.txt", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("nano story.txt", "/home/kid"),
                SimulatedEvent::AppStart("nano", "/home/kid"),
                SimulatedEvent::Idle(90),
                SimulatedEvent::AppStop("nano", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("cat story.txt", "/home/kid"),
                SimulatedEvent::Idle(15),
                SimulatedEvent::Cmd("head story.txt", "/home/kid"),
            ],
        },
        Scenario {
            name: "Frustrated Smash & Recovery",
            description: "User runs invalid keyboard smashes, gets frustrated, waits, then recovers.",
            events: vec![
                SimulatedEvent::Error("asdffas", "/home/kid"),
                SimulatedEvent::Idle(2),
                SimulatedEvent::Error("1234", "/home/kid"),
                SimulatedEvent::Idle(3),
                SimulatedEvent::Error("helpme", "/home/kid"),
                SimulatedEvent::Idle(30),
                SimulatedEvent::Cmd("help", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("clear", "/home/kid"),
            ],
        },
        Scenario {
            name: "The Matrix Addict",
            description: "User falls in love with the cool falling green code matrix effect.",
            events: vec![
                SimulatedEvent::Cmd("matrix", "/home/kid"),
                SimulatedEvent::AppStart("matrix", "/home/kid"),
                SimulatedEvent::Idle(45),
                SimulatedEvent::AppStop("matrix", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("matrix", "/home/kid"),
                SimulatedEvent::AppStart("matrix", "/home/kid"),
                SimulatedEvent::Idle(60),
                SimulatedEvent::AppStop("matrix", "/home/kid"),
            ],
        },
        Scenario {
            name: "Train Conductor",
            description: "User spams the 'sl' train command to watch the locomotive cross the terminal.",
            events: vec![
                SimulatedEvent::Cmd("sl", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("sl", "/home/kid"),
                SimulatedEvent::Idle(8),
                SimulatedEvent::Cmd("sl", "/home/kid"),
                SimulatedEvent::Idle(12),
                SimulatedEvent::Cmd("clear", "/home/kid"),
            ],
        },
        Scenario {
            name: "Quick Peek Session",
            description: "A very brief session where user checks identity, location, and logs off.",
            events: vec![
                SimulatedEvent::Cmd("whoami", "/home/kid"),
                SimulatedEvent::Idle(3),
                SimulatedEvent::Cmd("pwd", "/home/kid"),
                SimulatedEvent::Idle(4),
                SimulatedEvent::Cmd("ls", "/home/kid"),
                SimulatedEvent::Idle(10),
            ],
        },
        Scenario {
            name: "Nyan Cat Party",
            description: "User starts the nyan rainbow cat app and listens to the loop for a long time.",
            events: vec![
                SimulatedEvent::Cmd("nyan", "/home/kid"),
                SimulatedEvent::AppStart("nyan", "/home/kid"),
                SimulatedEvent::Idle(180),
                SimulatedEvent::AppStop("nyan", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("clear", "/home/kid"),
            ],
        },
        Scenario {
            name: "Spring Cleaning",
            description: "User deletes some unwanted drawing files from their home folder.",
            events: vec![
                SimulatedEvent::Cmd("ls", "/home/kid"),
                SimulatedEvent::Idle(6),
                SimulatedEvent::Cmd("rm picture.txt", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("clear", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("ls", "/home/kid"),
            ],
        },
        Scenario {
            name: "TuxMath Session",
            description: "User trains their arithmetic skills with Tux Math.",
            events: vec![
                SimulatedEvent::Cmd("tuxmath", "/home/kid"),
                SimulatedEvent::AppStart("tuxmath", "/home/kid"),
                SimulatedEvent::Idle(240),
                SimulatedEvent::AppStop("tuxmath", "/home/kid"),
                SimulatedEvent::Idle(15),
                SimulatedEvent::Cmd("ls", "/home/kid"),
            ],
        },
        Scenario {
            name: "TuxType Typing School",
            description: "User plays the typing games to practice home row keys.",
            events: vec![
                SimulatedEvent::Cmd("tuxtype", "/home/kid"),
                SimulatedEvent::AppStart("tuxtype", "/home/kid"),
                SimulatedEvent::Idle(150),
                SimulatedEvent::AppStop("tuxtype", "/home/kid"),
                SimulatedEvent::Idle(8),
            ],
        },
        Scenario {
            name: "Late Night Explorer",
            description: "User logs in, ticks show time range checks, runs basic commands.",
            events: vec![
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("whoami", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("date", "/home/kid"),
                SimulatedEvent::Idle(10),
            ],
        },
        Scenario {
            name: "Deep Concentration",
            description: "User works slowly but continuously, waiting just under the sleep threshold between inputs.",
            events: vec![
                SimulatedEvent::Cmd("ls", "/home/kid"),
                SimulatedEvent::Idle(25),
                SimulatedEvent::Cmd("cd apps", "/home/kid/apps"),
                SimulatedEvent::Idle(25),
                SimulatedEvent::Cmd("ls", "/home/kid/apps"),
                SimulatedEvent::Idle(25),
                SimulatedEvent::Cmd("clear", "/home/kid"),
            ],
        },
        Scenario {
            name: "Curious Explorer",
            description: "Navigating deep into folders, creating directories, listing files, some idle time.",
            events: vec![
                SimulatedEvent::Cmd("pwd", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("cd creations", "/home/kid/creations"),
                SimulatedEvent::Cmd("mkdir homework", "/home/kid/creations"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("cd homework", "/home/kid/creations/homework"),
                SimulatedEvent::Cmd("touch math.txt", "/home/kid/creations/homework"),
                SimulatedEvent::Idle(15),
                SimulatedEvent::Cmd("ls", "/home/kid/creations/homework"),
            ],
        },
        Scenario {
            name: "Math Champion Challenge",
            description: "Opening tuxmath, playing for a while, stopping, listing creations, repeating.",
            events: vec![
                SimulatedEvent::Cmd("tuxmath", "/home/kid"),
                SimulatedEvent::AppStart("tuxmath", "/home/kid"),
                SimulatedEvent::Idle(100),
                SimulatedEvent::AppStop("tuxmath", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("cd creations", "/home/kid/creations"),
                SimulatedEvent::Cmd("ls", "/home/kid/creations"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("tuxmath", "/home/kid/creations"),
                SimulatedEvent::AppStart("tuxmath", "/home/kid/creations"),
                SimulatedEvent::Idle(80),
                SimulatedEvent::AppStop("tuxmath", "/home/kid/creations"),
            ],
        },
        Scenario {
            name: "The Slow Typist",
            description: "Practicing tuxtype with slow commands, practicing touch/nano in between.",
            events: vec![
                SimulatedEvent::Cmd("tuxtype", "/home/kid"),
                SimulatedEvent::AppStart("tuxtype", "/home/kid"),
                SimulatedEvent::Idle(200),
                SimulatedEvent::AppStop("tuxtype", "/home/kid"),
                SimulatedEvent::Idle(30),
                SimulatedEvent::Cmd("touch practice.txt", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("nano practice.txt", "/home/kid"),
                SimulatedEvent::AppStart("nano", "/home/kid"),
                SimulatedEvent::Idle(120),
                SimulatedEvent::AppStop("nano", "/home/kid"),
            ],
        },
        Scenario {
            name: "Art & Clean Up",
            description: "Creating multiple files in creations, moving them, cleaning up with rm, checking with ls.",
            events: vec![
                SimulatedEvent::Cmd("cd creations", "/home/kid/creations"),
                SimulatedEvent::Cmd("touch art1.txt", "/home/kid/creations"),
                SimulatedEvent::Cmd("touch art2.txt", "/home/kid/creations"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("mkdir old_art", "/home/kid/creations"),
                SimulatedEvent::Cmd("mv art1.txt old_art", "/home/kid/creations"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("rm art2.txt", "/home/kid/creations"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("ls", "/home/kid/creations"),
            ],
        },
        Scenario {
            name: "Calculator Experiment",
            description: "Repeatedly calling cal and date to track time, checking directory.",
            events: vec![
                SimulatedEvent::Cmd("date", "/home/kid"),
                SimulatedEvent::Idle(4),
                SimulatedEvent::Cmd("cal", "/home/kid"),
                SimulatedEvent::Idle(4),
                SimulatedEvent::Cmd("date", "/home/kid"),
                SimulatedEvent::Idle(20),
                SimulatedEvent::Cmd("ls", "/home/kid"),
            ],
        },
        Scenario {
            name: "The Lost Programmer",
            description: "Error streak of 4, finally cd-ing home, then run whoami and help.",
            events: vec![
                SimulatedEvent::Error("cdd", "/home/kid"),
                SimulatedEvent::Idle(2),
                SimulatedEvent::Error("lss", "/home/kid"),
                SimulatedEvent::Idle(3),
                SimulatedEvent::Error("mkdirr", "/home/kid"),
                SimulatedEvent::Idle(4),
                SimulatedEvent::Error("helpp", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("cd ~", "/home/kid"),
                SimulatedEvent::Cmd("whoami", "/home/kid"),
                SimulatedEvent::Cmd("help", "/home/kid"),
            ],
        },
        Scenario {
            name: "Persistent Painter",
            description: "Tuxpaint started multiple times, with small idle pauses between sessions.",
            events: vec![
                SimulatedEvent::Cmd("tuxpaint", "/home/kid"),
                SimulatedEvent::AppStart("tuxpaint", "/home/kid"),
                SimulatedEvent::Idle(90),
                SimulatedEvent::AppStop("tuxpaint", "/home/kid"),
                SimulatedEvent::Idle(8),
                SimulatedEvent::Cmd("tuxpaint", "/home/kid"),
                SimulatedEvent::AppStart("tuxpaint", "/home/kid"),
                SimulatedEvent::Idle(120),
                SimulatedEvent::AppStop("tuxpaint", "/home/kid"),
            ],
        },
        Scenario {
            name: "Brief GCompris Session",
            description: "Fast startup and shutdown of gcompris, checking creations, logging off.",
            events: vec![
                SimulatedEvent::Cmd("gcompris", "/home/kid"),
                SimulatedEvent::AppStart("gcompris", "/home/kid"),
                SimulatedEvent::Idle(40),
                SimulatedEvent::AppStop("gcompris", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("cd creations", "/home/kid/creations"),
                SimulatedEvent::Cmd("ls", "/home/kid/creations"),
            ],
        },
        Scenario {
            name: "Word Counter",
            description: "Using touch, echo, cat, head, tail, and wc to check text files.",
            events: vec![
                SimulatedEvent::Cmd("touch notes.txt", "/home/kid"),
                SimulatedEvent::Cmd("echo one two three > notes.txt", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("cat notes.txt", "/home/kid"),
                SimulatedEvent::Cmd("head notes.txt", "/home/kid"),
                SimulatedEvent::Cmd("tail notes.txt", "/home/kid"),
                SimulatedEvent::Cmd("wc notes.txt", "/home/kid"),
            ],
        },
        Scenario {
            name: "Detective Mode",
            description: "Using grep and cat to find things, cd-ing between directories, running help.",
            events: vec![
                SimulatedEvent::Cmd("cd creations", "/home/kid/creations"),
                SimulatedEvent::Cmd("grep homework math.txt", "/home/kid/creations"),
                SimulatedEvent::Idle(8),
                SimulatedEvent::Cmd("cd ..", "/home/kid"),
                SimulatedEvent::Cmd("help", "/home/kid"),
            ],
        },
        Scenario {
            name: "Multi-App Marathon",
            description: "Starting tuxpaint, tuxmath, tuxtype, and gcompris in sequence with idle times.",
            events: vec![
                SimulatedEvent::Cmd("tuxpaint", "/home/kid"),
                SimulatedEvent::AppStart("tuxpaint", "/home/kid"),
                SimulatedEvent::Idle(60),
                SimulatedEvent::AppStop("tuxpaint", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("tuxmath", "/home/kid"),
                SimulatedEvent::AppStart("tuxmath", "/home/kid"),
                SimulatedEvent::Idle(60),
                SimulatedEvent::AppStop("tuxmath", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("tuxtype", "/home/kid"),
                SimulatedEvent::AppStart("tuxtype", "/home/kid"),
                SimulatedEvent::Idle(60),
                SimulatedEvent::AppStop("tuxtype", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("gcompris", "/home/kid"),
                SimulatedEvent::AppStart("gcompris", "/home/kid"),
                SimulatedEvent::Idle(90),
                SimulatedEvent::AppStop("gcompris", "/home/kid"),
            ],
        },
        Scenario {
            name: "Accidental Delete Recovery",
            description: "Checking files, accidentally deleting a file, practicing touch to recreate it.",
            events: vec![
                SimulatedEvent::Cmd("ls", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("rm story.txt", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("touch story.txt", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("ls", "/home/kid"),
            ],
        },
        Scenario {
            name: "Late Night Creative Session",
            description: "Idle checks, creating a folder, nanoing a text file, leaving it.",
            events: vec![
                SimulatedEvent::Idle(30),
                SimulatedEvent::Cmd("mkdir midnight", "/home/kid"),
                SimulatedEvent::Cmd("cd midnight", "/home/kid/midnight"),
                SimulatedEvent::Cmd("nano diary.txt", "/home/kid/midnight"),
                SimulatedEvent::AppStart("nano", "/home/kid/midnight"),
                SimulatedEvent::Idle(120),
                SimulatedEvent::AppStop("nano", "/home/kid/midnight"),
            ],
        },
        Scenario {
            name: "Train Driver & Matrix Fan",
            description: "Alternating sl and matrix apps and having fun.",
            events: vec![
                SimulatedEvent::Cmd("sl", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("matrix", "/home/kid"),
                SimulatedEvent::AppStart("matrix", "/home/kid"),
                SimulatedEvent::Idle(50),
                SimulatedEvent::AppStop("matrix", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("sl", "/home/kid"),
            ],
        },
        Scenario {
            name: "Echo Chamber",
            description: "Repeatedly echo-ing different sentences, checking with history/ls, clear.",
            events: vec![
                SimulatedEvent::Cmd("echo hello", "/home/kid"),
                SimulatedEvent::Cmd("echo world", "/home/kid"),
                SimulatedEvent::Cmd("echo moo", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("ls", "/home/kid"),
                SimulatedEvent::Cmd("clear", "/home/kid"),
            ],
        },
        Scenario {
            name: "Typing Speed Test",
            description: "Rapid commands, practice tuxtype, back, fast clear.",
            events: vec![
                SimulatedEvent::Cmd("pwd", "/home/kid"),
                SimulatedEvent::Cmd("whoami", "/home/kid"),
                SimulatedEvent::Cmd("tuxtype", "/home/kid"),
                SimulatedEvent::AppStart("tuxtype", "/home/kid"),
                SimulatedEvent::Idle(70),
                SimulatedEvent::AppStop("tuxtype", "/home/kid"),
                SimulatedEvent::Cmd("clear", "/home/kid"),
            ],
        },
        Scenario {
            name: "Quiet Homework Hour",
            description: "Long idle times between simple command runs, reading files.",
            events: vec![
                SimulatedEvent::Cmd("cd creations", "/home/kid/creations"),
                SimulatedEvent::Idle(35),
                SimulatedEvent::Cmd("cat math.txt", "/home/kid/creations"),
                SimulatedEvent::Idle(45),
                SimulatedEvent::Cmd("clear", "/home/kid/creations"),
            ],
        },
        Scenario {
            name: "Nyan Cat Dancer",
            description: "Starting nyan, leaving it on, stopping, starting matrix, leaving it on.",
            events: vec![
                SimulatedEvent::Cmd("nyan", "/home/kid"),
                SimulatedEvent::AppStart("nyan", "/home/kid"),
                SimulatedEvent::Idle(150),
                SimulatedEvent::AppStop("nyan", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("matrix", "/home/kid"),
                SimulatedEvent::AppStart("matrix", "/home/kid"),
                SimulatedEvent::Idle(100),
                SimulatedEvent::AppStop("matrix", "/home/kid"),
            ],
        },
        Scenario {
            name: "New Folder Structure",
            description: "Creating nested folders in creations, cd-ing, touching files, ls-ing.",
            events: vec![
                SimulatedEvent::Cmd("cd creations", "/home/kid/creations"),
                SimulatedEvent::Cmd("mkdir school", "/home/kid/creations"),
                SimulatedEvent::Cmd("cd school", "/home/kid/creations/school"),
                SimulatedEvent::Cmd("mkdir science", "/home/kid/creations/school"),
                SimulatedEvent::Cmd("cd science", "/home/kid/creations/school/science"),
                SimulatedEvent::Cmd("touch project.txt", "/home/kid/creations/school/science"),
                SimulatedEvent::Idle(15),
                SimulatedEvent::Cmd("ls", "/home/kid/creations/school/science"),
            ],
        },
        Scenario {
            name: "Frustrated Artist Recovery",
            description: "Tuxpaint error, help check, successful tuxpaint run, creations review.",
            events: vec![
                SimulatedEvent::Error("tuxpaintt", "/home/kid"),
                SimulatedEvent::Idle(5),
                SimulatedEvent::Cmd("help", "/home/kid"),
                SimulatedEvent::Idle(10),
                SimulatedEvent::Cmd("tuxpaint", "/home/kid"),
                SimulatedEvent::AppStart("tuxpaint", "/home/kid"),
                SimulatedEvent::Idle(120),
                SimulatedEvent::AppStop("tuxpaint", "/home/kid"),
                SimulatedEvent::Cmd("cd creations", "/home/kid"),
                SimulatedEvent::Cmd("ls", "/home/kid/creations"),
            ],
        },
    ]
}

#[cfg(test)]
mod scenario_tests {
    use super::*;

    #[test]
    fn dump_scenario_metrics() {
        let config = toml::from_str(crate::config::personality::get_default_toml()).unwrap();
        let scenarios = get_scenarios();
        let mut out = String::new();
        out.push_str("# Scenario Simulation Metrics Dump\n\n");
        out.push_str("| ID | Scenario Name | Cmds | Trig | Idle | Trig% | AppTime | Avg/Sec | MaxPause |\n");
        out.push_str("|---|---|---|---|---|---|---|---|---|\n");
        for (idx, sc) in scenarios.iter().enumerate() {
            let (m, _) = run_scenario(sc, &config);
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {:.1}% | {:.1}s | {:.3} | {:.1}s |\n",
                idx + 1, sc.name, m.cmds, m.trig, m.idle, m.trig_rate, m.app_time, m.avg_sec, m.longest_pause
            ));
        }
        std::fs::write("/Users/nall/pers/kid-cli/scenario_metrics.md", out).unwrap();
    }
}


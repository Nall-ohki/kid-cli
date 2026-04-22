use std::{io, time::Duration};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Alignment, Rect},
    widgets::{Block, Borders, Paragraph, Wrap, List, ListItem, ListState},
    Frame, Terminal,
    style::{Style, Color, Modifier},
    text::{Line, Span},
};
use crate::characters::{registry::Registry, render, types::CharacterKind};

#[derive(PartialEq)]
enum ViewMode {
    Detail,
    Grid,
}

struct App {
    registry: Registry,
    list_state: ListState,
    view_mode: ViewMode,
    preview_rect: Rect,
}

impl App {
    fn new() -> Self {
        let registry = Registry::from_builtins();
        let mut list_state = ListState::default();
        if registry.count() > 0 {
            list_state.select(Some(0));
        }
        Self {
            registry,
            list_state,
            view_mode: ViewMode::Detail,
            preview_rect: Rect::default(),
        }
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.registry.count() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.registry.count() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn toggle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Detail => ViewMode::Grid,
            ViewMode::Grid => ViewMode::Detail,
        };
    }
}

pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        // 4. Render Sixel (if current is Sixel and in Detail view)
        if app.view_mode == ViewMode::Detail {
            if let Some(selected_index) = app.list_state.selected() {
                if let Some(chara) = app.registry.get_by_index(selected_index) {
                    if let CharacterKind::Sixel(data) = &chara.kind {
                        use crossterm::cursor;
                        let mut stdout = io::stdout();
                        
                        // Calculate horizontal centering
                        let char_width = chara.width as u16;
                        let h_padding = app.preview_rect.width.saturating_sub(char_width) / 2;
                        
                        let _ = execute!(
                            stdout,
                            cursor::MoveTo(app.preview_rect.x + h_padding, app.preview_rect.y),
                        );
                        
                        print!("{}", data);
                        let _ = io::Write::flush(&mut stdout);
                    }
                }
            }
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('v') => { app.toggle_view(); terminal.clear()?; },
                    KeyCode::Down | KeyCode::Char('j') => { app.next(); terminal.clear()?; },
                    KeyCode::Up | KeyCode::Char('k') => { app.previous(); terminal.clear()?; },
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

    // Base block
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" KID CHARACTER VIEWER ")
        .border_style(Style::default().fg(Color::Cyan));
    f.render_widget(block, size);

    let inner_area = size.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 1 });

    match app.view_mode {
        ViewMode::Detail => draw_detail_view(f, inner_area, app),
        ViewMode::Grid => draw_grid_view(f, inner_area, app),
    }
}

fn draw_detail_view(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(25), Constraint::Min(0)])
        .split(area);

    // 1. List of characters
    let names: Vec<ListItem> = (0..app.registry.count())
        .map(|i| {
            // We need a way to get name by index if we don't want to expose too much of Registry
            // For now, I'll add a helper to Registry or just use by_name if I had the list
            // Actually, I'll just hack it for now by loading builtins again or exposing a list
            // Better: I'll update Registry to expose names or a getter.
            let chara = app.registry.get_by_index(i).unwrap();
            ListItem::new(chara.name.as_str())
        })
        .collect();

    let list = List::new(names)
        .block(Block::default().borders(Borders::RIGHT).title(" Assets "))
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::Yellow))
        .highlight_symbol(">> ");
    
    f.render_stateful_widget(list, chunks[0], &mut app.list_state);

    // 2. Preview Area
    if let Some(selected_index) = app.list_state.selected() {
        if let Some(chara) = app.registry.get_by_index(selected_index) {
            let preview_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(chunks[1]);

            // Info bar
            let info = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("Name: ", Style::default().fg(Color::Gray)),
                    Span::styled(&chara.name, Style::default().add_modifier(Modifier::BOLD).fg(Color::White)),
                    Span::raw(" ("),
                    Span::styled(&chara.id, Style::default().fg(Color::DarkGray)),
                    Span::raw(") | "),
                    Span::styled("Source: ", Style::default().fg(Color::Gray)),
                    Span::styled(format!("{:?}", chara.source), Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::styled("Type: ", Style::default().fg(Color::Gray)),
                    Span::styled(match &chara.kind {
                        CharacterKind::Grid(_) => "ASCII/Grid",
                        CharacterKind::Sixel(_) => "Sixel Bitmap",
                    }, Style::default().fg(Color::Cyan)),
                    Span::raw(" | "),
                    Span::styled("Size: ", Style::default().fg(Color::Gray)),
                    Span::styled(format!("{}x{}", chara.width, chara.height), Style::default().fg(Color::Magenta)),
                ]),
            ]).block(Block::default().borders(Borders::BOTTOM));
            f.render_widget(info, preview_chunks[0]);

            // Character render
            match &chara.kind {
                CharacterKind::Grid(grid) => {
                    let lines = render::render_grid(grid, None);
                    let para = Paragraph::new(lines)
                        .alignment(Alignment::Left);
                    f.render_widget(para, preview_chunks[1]);
                }
                CharacterKind::Sixel(_data) => {
                    // Save area for post-draw Sixel injection
                    app.preview_rect = preview_chunks[1];
                }
            }
        }
    }
}

fn draw_grid_view(f: &mut Frame, area: Rect, app: &mut App) {
    // Sprite-sheet like view
    // We'll split the area into a grid
    let cols = 3;
    let rows = (app.registry.count() as f32 / cols as f32).ceil() as usize;
    
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Ratio(1, rows as u32); rows])
        .split(area);

    for r in 0..rows {
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Ratio(1, cols as u32); cols])
            .split(vertical_chunks[r]);

        for c in 0..cols {
            let i = r * cols + c;
            if i < app.registry.count() {
                if let Some(chara) = app.registry.get_by_index(i) {
                    let block = Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" {} ", chara.name))
                        .border_style(if Some(i) == app.list_state.selected() {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        });
                    
                    let inner = block.inner(horizontal_chunks[c]);
                    f.render_widget(block, horizontal_chunks[c]);

                    match &chara.kind {
                        CharacterKind::Grid(grid) => {
                            let lines = render::render_grid(grid, None);
                            let para = Paragraph::new(lines).wrap(Wrap { trim: false });
                            f.render_widget(para, inner);
                        }
                        CharacterKind::Sixel(_) => {
                            let para = Paragraph::new("Sixel").alignment(Alignment::Center);
                            f.render_widget(para, inner);
                        }
                    }
                }
            }
        }
    }
}

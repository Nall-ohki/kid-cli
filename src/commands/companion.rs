use std::{io::{self, BufRead, BufReader}, time::Duration, fs::{self, OpenOptions}};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Alignment, Rect},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
    style::{Style, Color, Modifier},
    text::Span,
};
use tokio::sync::mpsc;
use crate::characters::{registry::Registry, render, types::CharacterKind};

const WELCOME_MSG: &str = "Hello! I am your AI coach. How can I help you today?";

pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Character Registry
    let mut registry = Registry::from_builtins();
    registry.select_by_name("cow");

    // Channel for pipe messages
    let (tx, mut rx) = mpsc::channel::<String>(10);
    
    // Background thread to listen to the pipe
    let pipe_path = "/tmp/kid_companion_pipe";
    
    // Ensure pipe exists
    let _ = fs::remove_file(pipe_path);
    nix::unistd::mkfifo(pipe_path, nix::sys::stat::Mode::S_IRWXU)?;

    tokio::spawn(async move {
        loop {
            if let Ok(file) = OpenOptions::new().read(true).open(pipe_path) {
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    if let Ok(l) = line {
                        let _ = tx.send(l).await;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let mut current_msg = WELCOME_MSG.to_string();
    let mut tick_count = 0u64;

    let mut sixel_area = Rect::default();
    
    loop {
        terminal.draw(|f| {
            let size = f.size();
            let char_height = registry.current().map(|c| c.height as u16).unwrap_or(8);
            
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(4),
                    Constraint::Length(char_height + 2),
                ])
                .split(size);

            sixel_area = chunks[1];

            // Rainbow colors logic
            let rainbow = [
                Color::Red, Color::LightRed, Color::Yellow, 
                Color::Green, Color::Cyan, Color::Blue, Color::Magenta
            ];
            let base_color = rainbow[(tick_count / 10 % rainbow.len() as u64) as usize];

            // 1. Speech Bubble
            let bubble = Paragraph::new(current_msg.as_str())
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(base_color))
                    .title(Span::styled(" COACH ", Style::default().add_modifier(Modifier::BOLD).fg(base_color))));
            
            f.render_widget(bubble, chunks[0]);

            // 2. Character (Grid path)
            if let Some(chara) = registry.current() {
                if let CharacterKind::Grid(grid) = &chara.kind {
                    let lines = render::render_grid(grid);
                    let para = Paragraph::new(lines)
                        .alignment(Alignment::Left);
                    f.render_widget(para, chunks[1]);
                }
            }
        })?;

        // If current is Sixel, output it now
        if let Some(chara) = registry.current() {
            if let CharacterKind::Sixel(data) = &chara.kind {
                use crossterm::cursor;
                let mut stdout = io::stdout();
                let _ = execute!(
                    stdout,
                    cursor::MoveTo(sixel_area.x, sixel_area.y),
                );
                // The data might have literal \x1B, we need to resolve them
                let resolved_data = data.replace("\\x1B", "\x1B");
                print!("{}", resolved_data);
                let _ = io::Write::flush(&mut stdout);
            }
        }

        // Check for messages
        while let Ok(new_msg) = rx.try_recv() {
            if new_msg == "COMMAND:RESET" {
                current_msg = WELCOME_MSG.to_string();
            } else {
                current_msg = new_msg;
            }
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('n') => { registry.next(); },
                    KeyCode::Char('p') => { registry.prev(); },
                    _ => {}
                }
            }
        }
        tick_count += 1;
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


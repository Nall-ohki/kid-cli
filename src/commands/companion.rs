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
    let mut last_msg_tick = 0u64;
    let mut sixel_area = Rect::default();
    
    loop {
        terminal.draw(|f| {
            let size = f.size();
            let char_height = registry.current().map(|c| c.height as u16).unwrap_or(8);

            // Determine if we should flash
            let is_flashing = tick_count - last_msg_tick < 40; // 40 ticks @ 50ms = 2 seconds
            
            let base_color = if is_flashing {
                let rainbow = [
                    Color::Red, Color::LightRed, Color::Yellow, 
                    Color::Green, Color::Cyan, Color::Blue, Color::Magenta
                ];
                rainbow[(tick_count / 2 % rainbow.len() as u64) as usize]
            } else {
                Color::Cyan
            };

            // Calculate bubble height (estimate)
            let wrap_width = size.width.saturating_sub(4) as usize;
            let text_lines = if wrap_width > 0 {
                (current_msg.len() / wrap_width) + 1
            } else {
                1
            };
            let bubble_height = (text_lines as u16 + 4).min(size.height / 2); // 2 padding + 2 borders
            let total_height = bubble_height + char_height;
            let v_padding = size.height.saturating_sub(total_height) / 2;

            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(v_padding),
                    Constraint::Length(bubble_height),
                    Constraint::Length(char_height),
                    Constraint::Min(0),
                ])
                .split(size);

            sixel_area = main_chunks[2];

            // 1. Speech Bubble
            let bubble_content = format!("\n{}\n", current_msg); // Padding
            let bubble = Paragraph::new(bubble_content)
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(base_color))
                    .title(Span::styled(
                        format!(" {} ", registry.current().map(|c| c.name.as_str()).unwrap_or("COACH")),
                        Style::default().add_modifier(Modifier::BOLD).fg(base_color)
                    )));
            
            f.render_widget(bubble, main_chunks[1]);

            // 2. Character (Grid path)
            if let Some(chara) = registry.current() {
                if let CharacterKind::Grid(grid) = &chara.kind {
                    let lines = render::render_grid(grid);
                    let para = Paragraph::new(lines)
                        .alignment(Alignment::Left);
                    
                    let char_width = chara.width as u16;
                    let h_padding = main_chunks[2].width.saturating_sub(char_width) / 2;
                    let char_h_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(h_padding),
                            Constraint::Length(char_width),
                            Constraint::Min(0),
                        ])
                        .split(main_chunks[2]);
                    f.render_widget(para, char_h_chunks[1]);
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
            last_msg_tick = tick_count; // Trigger flash
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


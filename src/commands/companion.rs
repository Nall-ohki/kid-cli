use std::{io, time::Duration, fs::{self, OpenOptions}};
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
        use std::io::Read;
        loop {
            if let Ok(mut file) = OpenOptions::new().read(true).open(pipe_path) {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    if !content.is_empty() {
                        let _ = tx.send(content).await;
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
    
    let timeout_ticks = 300; // 15s @ 50ms ticks
    let fade_start_ticks = 225; // 75% of 15s
    
    loop {
        // 1. Calculate Speech State
        let elapsed = tick_count.saturating_sub(last_msg_tick);
        let is_flashing = elapsed < 40; // 2 seconds
        let is_visible = elapsed < timeout_ticks;
        
        let (bubble_style, connector_style) = if !is_visible || current_msg.is_empty() {
            // Completely hidden
            (Style::default().fg(Color::Black), Some(Style::default().fg(Color::Black)))
        } else {
            let fade_color = if elapsed > fade_start_ticks {
                let progress = (elapsed - fade_start_ticks) as f32 / (timeout_ticks - fade_start_ticks) as f32;
                let grey_code = 255 - (progress * 23.0) as u8;
                Color::Indexed(grey_code)
            } else {
                Color::Cyan
            };

            let border_color = if is_flashing {
                let rainbow = [
                    Color::Red, Color::LightRed, Color::Yellow, 
                    Color::Green, Color::Cyan, Color::Blue, Color::Magenta
                ];
                rainbow[(tick_count / 2 % rainbow.len() as u64) as usize]
            } else {
                fade_color
            };

            (Style::default().fg(fade_color), Some(Style::default().fg(border_color)))
        };

        terminal.draw(|f| {
            let size = f.size();
            let char_height = registry.current().map(|c| c.height as u16).unwrap_or(8);

            // Calculate bubble height (estimate)
            let wrap_width = size.width.saturating_sub(4) as usize;
            let text_lines = if wrap_width > 0 {
                (current_msg.len() / wrap_width) + 1
            } else {
                1
            };
            let bubble_height = (text_lines as u16 + 4).min(size.height / 2);
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

            // 2. Render Speech Bubble
            if is_visible && !current_msg.is_empty() {
                let border_color = connector_style.unwrap().fg.unwrap(); // Sync border with connectors
                let bubble_content = format!("\n{}\n", current_msg);
                let bubble = Paragraph::new(bubble_content)
                    .wrap(Wrap { trim: true })
                    .alignment(Alignment::Center)
                    .style(bubble_style)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border_color))
                        .title(Span::styled(
                            format!(" {} ", registry.current().map(|c| c.name.as_str()).unwrap_or("COACH")),
                            Style::default().add_modifier(Modifier::BOLD).fg(border_color)
                        )));
                
                f.render_widget(bubble, main_chunks[1]);
            }

            // 3. Render Character (Grid path)
            if let Some(chara) = registry.current() {
                if let CharacterKind::Grid(grid) = &chara.kind {
                    let lines = render::render_grid(grid, connector_style);
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

        // 4. Render Sixel (if current is Sixel)
        if let Some(chara) = registry.current() {
            if let CharacterKind::Sixel(data) = &chara.kind {
                use crossterm::cursor;
                let mut stdout = io::stdout();
                
                // Calculate horizontal centering
                let char_width = chara.width as u16;
                let h_padding = sixel_area.width.saturating_sub(char_width) / 2;
                
                let _ = execute!(
                    stdout,
                    cursor::MoveTo(sixel_area.x + h_padding, sixel_area.y),
                );

                print!("{}", data);
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


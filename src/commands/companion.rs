use std::{io::{self, BufRead, BufReader}, time::Duration, fs::{self, OpenOptions}, os::unix::fs::OpenOptionsExt};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Alignment},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
    style::{Style, Color, Modifier},
    text::{Line, Span},
};
use tokio::sync::mpsc;

const WELCOME_MSG: &str = "Hello! I am your AI coach. How can I help you today?";

pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Channel for pipe messages
    let (tx, mut rx) = mpsc::channel::<String>(10);
    
    // Background thread to listen to the pipe
    let pipe_path = "/tmp/kid_companion_pipe";
    
    // Ensure pipe exists
    let _ = fs::remove_file(pipe_path);
    nix::unistd::mkfifo(pipe_path, nix::sys::stat::Mode::S_IRWXU)?;

    tokio::spawn(async move {
        loop {
            // Open blocks until a writer connects
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

    loop {
        terminal.draw(|f| ui(f, &current_msg, tick_count))?;

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
                if key.code == KeyCode::Char('q') {
                    break;
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

fn ui(f: &mut Frame, msg: &str, tick: u64) {
    let size = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(8)])
        .split(size);

    // Rainbow colors logic (smooth transition)
    let colors = [
        Color::Red, Color::LightRed, Color::Yellow, 
        Color::Green, Color::Cyan, Color::Blue, Color::Magenta
    ];
    let base_color = colors[(tick / 10 % colors.len() as u64) as usize];

    // 1. Speech Bubble
    let bubble = Paragraph::new(msg)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(base_color))
            .title(Span::styled(" COACH ", Style::default().add_modifier(Modifier::BOLD).fg(base_color))));
    
    f.render_widget(bubble, chunks[0]);

    // 2. Cow ASCII
    let cow = r#"
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
    "#;
    
    let cow_para = Paragraph::new(cow)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));
    
    // Put cow in the bottom chunk
    f.render_widget(cow_para, chunks[1]);
}

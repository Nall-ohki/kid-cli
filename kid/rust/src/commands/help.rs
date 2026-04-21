use std::{io, time::Duration};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, List, ListItem, ListState},
    Frame, Terminal,
};

pub fn run(initial_section: &str) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let sections = vec!["Basic", "Files", "Tools", "Fun"];
    let mut state = ListState::default();
    
    let initial_index = match initial_section.to_lowercase().as_str() {
        "basic" => 0,
        "files" => 1,
        "tools" => 2,
        "fun" => 3,
        _ => 0,
    };
    state.select(Some(initial_index));

    // Run loop
    let res = run_app(&mut terminal, &sections, &mut state);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    sections: &[&str],
    state: &mut ListState,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, sections, state))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Down => {
                        let i = match state.selected() {
                            Some(i) => {
                                if i >= sections.len() - 1 {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        state.select(Some(i));
                    }
                    KeyCode::Up => {
                        let i = match state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    sections.len() - 1
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        state.select(Some(i));
                    }
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, sections: &[&str], state: &mut ListState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(f.size());

    let items: Vec<ListItem> = sections
        .iter()
        .map(|i| ListItem::new(*i))
        .collect();

    let list = List::new(items)
        .block(Block::default().title("Sections").borders(Borders::ALL))
        .highlight_symbol(">> ");
    
    f.render_stateful_widget(list, chunks[0], state);

    let selected = sections[state.selected().unwrap_or(0)];
    let content = match selected {
        "Basic" => "🏠 home - Go home\n👀 ls - List files\n📂 cd - Change directory\n✨ clear - Clear screen\n👋 exit - Goodbye!",
        "Files" => "📄 cat - Read a file\n🔍 file - Check file type\n✨ touch - Create file\n🚫 rm - Forbidden!",
        "Tools" => "📘 man - Detailed help\n📝 nano - Text editor\n⏰ date - Show date\n🗓 cal - Show calendar",
        "Fun" => "🚂 sl - Train!\n💻 matrix - Matrix effect\n🐮 say - Cow talk\n🔡 letters - Banners\n🌈 nyan - Rainbow cat",
        _ => "Select a section",
    };

    let p = Paragraph::new(content)
        .block(Block::default().title(format!("{} Commands", selected)).borders(Borders::ALL));
    f.render_widget(p, chunks[1]);
}

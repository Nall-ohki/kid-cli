use crossterm::style::{Color, ResetColor, SetForegroundColor};
use crossterm::ExecutableCommand;
use std::io::{stderr, stdout, Write};

#[derive(Debug, Clone, Copy)]
pub enum MessageLevel {
    Error,
    Warn,
    Info,
    Ok,
}

pub fn styled_message(level: MessageLevel, text: &str) {
    match level {
        MessageLevel::Error => print_msg("✗", Color::Red, &mut stderr(), text),
        MessageLevel::Warn => print_msg("⚠", Color::Yellow, &mut stderr(), text),
        MessageLevel::Info => print_msg("ℹ", Color::Blue, &mut stdout(), text),
        MessageLevel::Ok => print_msg("✓", Color::Green, &mut stdout(), text),
    }
}

fn print_msg(sigil: &str, color: Color, stream: &mut dyn Write, text: &str) {
    let _ = stream.execute(SetForegroundColor(color));
    let _ = write!(stream, "{}  ", sigil);
    let _ = stream.execute(ResetColor);
    let _ = writeln!(stream, "{}", text);
    let _ = stream.flush();
}

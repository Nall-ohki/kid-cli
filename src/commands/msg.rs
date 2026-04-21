use crate::terminal::{styled_message, MessageLevel};
use clap::ValueEnum;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MsgLevel {
    Error,
    Warn,
    Info,
    Ok,
}

impl From<MsgLevel> for MessageLevel {
    fn from(level: MsgLevel) -> Self {
        match level {
            MsgLevel::Error => MessageLevel::Error,
            MsgLevel::Warn => MessageLevel::Warn,
            MsgLevel::Info => MessageLevel::Info,
            MsgLevel::Ok => MessageLevel::Ok,
        }
    }
}

pub fn run(level: MsgLevel, text: &str) {
    styled_message(level.into(), text);
}

use anyhow::Result;
use crate::terminal::{styled_message, MessageLevel};
use crate::config::commands::BlockConfig;

pub fn run(name: &str, config: &BlockConfig) -> Result<()> {
    let msg = config.message.replace("{cmd}", name);
    styled_message(MessageLevel::Error, &msg);
    Err(anyhow::anyhow!("BLOCKED"))
}

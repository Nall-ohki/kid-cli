pub mod commands;
pub mod daemon;
pub mod messages;
pub mod personality;

use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};

pub fn get_config_dir() -> Result<PathBuf> {
    let home = home::home_dir().context("Could not find home directory")?;
    let config_dir = home.join(".config").join("kid");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).context("Could not create config directory")?;
    }
    Ok(config_dir)
}

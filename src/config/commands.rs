use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::{fs, path::PathBuf};
use anyhow::{Context, Result};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub launchers: HashMap<String, LauncherConfig>,
    #[serde(default)]
    pub passthroughs: HashMap<String, String>,
    #[serde(default)]
    pub blocks: BlockConfig,
    #[serde(default)]
    pub systems: HashMap<String, SystemConfig>,
    #[serde(default)]
    pub games: HashMap<String, GameConfig>,
}

impl Config {
    pub fn load(path: PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path).context("Could not read commands.toml")?;
        let config: Config = toml::from_str(&content).context("Could not parse commands.toml")?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_config() {
        let toml_str = get_default_toml();
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.launchers.contains_key("matrix"));
        assert!(config.passthroughs.contains_key("ls"));
        assert!(config.blocks.commands.contains(&"sudo".to_string()));
        assert!(config.systems.contains_key("apple2gs"));
        assert!(config.systems.contains_key("snes"));
        assert!(config.systems.contains_key("scummvm"));
        assert!(config.systems.contains_key("dosbox-carmen"));
        assert!(config.systems.contains_key("dosbox-donald"));
        assert!(config.games.contains_key("oregon"));
        assert!(config.games.contains_key("murphy"));
        assert!(config.games.contains_key("mario"));
        assert!(config.games.contains_key("parade"));
        assert!(config.games.contains_key("moon"));
        assert!(config.games.contains_key("zoo"));
        assert!(config.games.contains_key("amazon"));
        assert!(config.games.contains_key("yukon"));
        assert!(config.games.contains_key("donald"));
        assert!(config.games.contains_key("carmen"));
        assert_eq!(config.launchers.get("tuxpaint").and_then(|l| l.category.as_deref()), Some("art"));
        assert_eq!(config.launchers.get("tuxmath").and_then(|l| l.category.as_deref()), Some("math"));
        assert_eq!(config.launchers.get("tuxtype").and_then(|l| l.category.as_deref()), Some("abc"));
        assert_eq!(config.launchers.get("gcompris").and_then(|l| l.category.as_deref()), Some("abc"));
        assert_eq!(config.launchers.get("scratch").and_then(|l| l.category.as_deref()), Some("code"));
        assert_eq!(config.games.get("oregon").and_then(|g| g.category.as_deref()), Some("play"));
        assert_eq!(config.games.get("nummunch").and_then(|g| g.category.as_deref()), Some("math"));
        assert_eq!(config.games.get("wordmunch").and_then(|g| g.category.as_deref()), Some("abc"));
        assert_eq!(config.games.get("donald").and_then(|g| g.category.as_deref()), Some("abc"));
        assert_eq!(config.games.get("carmen").and_then(|g| g.category.as_deref()), Some("play"));
        assert_eq!(config.games.get("mario").and_then(|g| g.category.as_deref()), Some("art"));
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemConfig {
    pub template: String,
    pub rom_dir: String,
}

pub fn default_enabled() -> bool { true }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameConfig {
    pub system: String,
    pub rom: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub category: Option<String>,
}

pub fn default_pane() -> String {
    "none".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LauncherConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub binary: Option<String>,
    #[serde(default = "default_pane")]
    pub pane: String,
    #[serde(default)]
    pub lolcat: LolcatMode,
    #[serde(default)]
    pub persist: bool,
    #[serde(default)]
    pub builtin: bool,
    #[serde(default)]
    pub gui: bool,
    #[serde(default)]
    pub category: Option<String>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            binary: None,
            pane: "none".to_string(),
            lolcat: LolcatMode::default(),
            persist: false,
            builtin: false,
            gui: false,
            category: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum LolcatMode {
    Simple(String), // "never", "always"
    Chance { chance: f32 },
}

impl Default for LolcatMode {
    fn default() -> Self {
        LolcatMode::Simple("never".to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct BlockConfig {
    pub commands: Vec<String>,
    pub message: String,
}

pub fn get_default_toml() -> &'static str {
    include_str!("commands.toml")
}

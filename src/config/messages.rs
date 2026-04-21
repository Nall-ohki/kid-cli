use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub home: SectionPool,
    pub cd: CdConfig,
    pub ls: SectionPool,
    pub discovery: DiscoveryConfig,
}

impl Config {
    pub fn load(path: std::path::PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SectionPool {
    pub pool: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CdConfig {
    pub pool: Vec<String>,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscoveryConfig {
    pub template: String,
}

pub fn get_default_toml() -> &'static str {
    r#"
[home]
pool = [
  "Welcome home! This is where you start.",
  "You can go to 'apps' from here!",
  "Type 'help' to see what you can do!",
  "お帰りなさい！",
  "Type 'ls' to see what's here.",
  "Type 'cd apps' to find programs!",
]

[cd]
pool = [
  "You moved! Type 'ls' to look around.",
  "Try 'pwd' to see where you are!",
  "Type 'cd ..' to go up.",
  "New place! What's here?",
]

[cd.context]
"*/apps"           = "Apps are here! Type 'ls' to see them all!"
"*/apps/gcompris"  = "You found GCompris! Type './gcompris' to play!"
"*/apps/tuxpaint"  = "Tux Paint! Type './tuxpaint' to draw!"

[ls]
pool = [
  "These are the files and folders!",
  "Folders are blue - you can go in them with 'cd'!",
  "Pick something to explore!",
]

[discovery]
template = "You discovered `{cmd}`! {icon}"
"#
}

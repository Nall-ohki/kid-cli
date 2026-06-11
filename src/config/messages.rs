use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub home: SectionPool,
    pub cd: CdConfig,
    pub ls: SectionPool,
    pub discovery: DiscoveryConfig,
}

impl Config {
    #[allow(dead_code)]
    pub fn load(path: std::path::PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SectionPool {
    pub pool: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CdConfig {
    pub pool: Vec<String>,
    pub context: HashMap<String, String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscoveryConfig {
    pub template: String,
}

pub fn get_default_toml() -> &'static str {
    include_str!("messages.toml")
}

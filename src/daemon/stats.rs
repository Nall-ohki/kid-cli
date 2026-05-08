use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Stats {
    pub commands: HashMap<String, CommandStats>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandStats {
    pub count: u64,
    pub last_run: DateTime<Utc>,
}

impl Stats {
    pub fn load() -> Result<Self> {
        let path = get_stats_file()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path).context("Could not read stats.toml")?;
        let stats: Stats = toml::from_str(&content).context("Could not parse stats.toml")?;
        Ok(stats)
    }

    pub fn save(&self) -> Result<()> {
        let path = get_stats_file()?;
        let content = toml::to_string_pretty(self).context("Could not serialize stats")?;
        fs::write(&path, content).context("Could not write stats.toml")?;
        Ok(())
    }
}

pub fn get_stats_file() -> Result<PathBuf> {
    if let Ok(val) = std::env::var("KID_STATS_PATH") {
        return Ok(PathBuf::from(val));
    }

    let home = home::home_dir().context("Could not find home directory")?;
    let creations_dir = home.join("creations");
    
    // Ensure the directory exists
    if !creations_dir.exists() {
        fs::create_dir_all(&creations_dir).context("Could not create creations directory")?;
    }
    Ok(creations_dir.join("stats.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_serialization() {
        let mut commands = HashMap::new();
        commands.insert("ls".to_string(), CommandStats {
            count: 42,
            last_run: Utc::now(),
        });
        let stats = Stats { commands };
        
        let toml_str = toml::to_string(&stats).unwrap();
        assert!(toml_str.contains("ls"));
        assert!(toml_str.contains("count = 42"));
        
        let decoded: Stats = toml::from_str(&toml_str).unwrap();
        assert_eq!(decoded.commands.get("ls").unwrap().count, 42);
    }

    #[test]
    fn test_stats_robustness() {
        // 1. Test path override
        std::env::set_var("KID_STATS_PATH", "/tmp/fake_stats.toml");
        let path = get_stats_file().unwrap();
        assert_eq!(path.to_str().unwrap(), "/tmp/fake_stats.toml");

        // 2. Test load no file
        std::env::set_var("KID_STATS_PATH", "/tmp/non_existent_stats.toml");
        let _ = std::fs::remove_file("/tmp/non_existent_stats.toml");
        let stats = Stats::load();
        assert!(stats.is_ok()); 
        assert!(stats.unwrap().commands.is_empty());

        // 3. Test load corrupt file
        let corrupt_path = "/tmp/corrupt_stats.toml";
        std::fs::write(corrupt_path, "this is not valid toml = [[[").unwrap();
        std::env::set_var("KID_STATS_PATH", corrupt_path);
        let stats = Stats::load();
        assert!(stats.is_err()); 

        // Cleanup
        std::env::remove_var("KID_STATS_PATH");
        let _ = std::fs::remove_file(corrupt_path);
    }
}

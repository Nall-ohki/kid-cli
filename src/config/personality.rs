use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub timing: TimingConfig,
    pub mood: MoodConfig,
    #[serde(rename = "rules")]
    pub rules: Vec<Rule>,
}

impl Config {
    pub fn load(path: std::path::PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimingConfig {
    pub idle_threshold_secs: u64,
    pub sleep_threshold_secs: u64,
    pub message_cooldown_secs: u64,
    pub mood_decay_secs: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MoodConfig {
    pub default: String,
    pub options: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rule {
    pub trigger: String,
    #[serde(default)]
    pub conditions: Conditions,
    pub responses: Vec<String>,
    #[serde(default)]
    pub set_mood: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Conditions {
    #[serde(default)]
    pub mood: Option<String>,
    #[serde(default)]
    pub min_count: Option<u64>,
    #[serde(default)]
    pub streak: Option<u32>,
    #[serde(default)]
    pub chance: Option<f32>,
    #[serde(default)]
    pub after_days: Option<i64>,
}

pub fn get_default_toml() -> &'static str {
    include_str!("personality.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_config() {
        let toml_str = get_default_toml();
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.timing.idle_threshold_secs, 15);
        assert_eq!(config.mood.default, "neutral");
        assert!(!config.rules.is_empty());
        
        // Check that we have rules for various trigger types
        let triggers: Vec<&str> = config.rules.iter().map(|r| r.trigger.as_str()).collect();
        assert!(triggers.contains(&"idle"));
        assert!(triggers.contains(&"sleep"));
        assert!(triggers.contains(&"wake"));
        assert!(triggers.contains(&"error"));
        assert!(triggers.contains(&"error_streak"));
        assert!(triggers.contains(&"milestone"));
        assert!(triggers.contains(&"absence"));
    }

    #[test]
    fn test_conditions_default() {
        let c = Conditions::default();
        assert!(c.mood.is_none());
        assert!(c.min_count.is_none());
        assert!(c.streak.is_none());
        assert!(c.chance.is_none());
        assert!(c.after_days.is_none());
    }

    #[test]
    fn test_rule_with_set_mood() {
        let toml_str = r#"
            [timing]
            idle_threshold_secs = 90
            sleep_threshold_secs = 300
            message_cooldown_secs = 30
            mood_decay_secs = 300

            [mood]
            default = "neutral"
            options = ["neutral", "sleepy"]

            [[rules]]
            trigger = "sleep"
            responses = ["zZz"]
            set_mood = "sleepy"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.rules[0].set_mood, Some("sleepy".to_string()));
    }
}

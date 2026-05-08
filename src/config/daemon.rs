use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub companion: CompanionConfig,
    pub patterns: PatternsConfig,
    pub stats: StatsConfig,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct CompanionConfig {
    pub pane_width_percent: u32,
    pub cooldown_seconds: u32,
    pub show_activity_feed: bool,
    pub max_feed_lines: u32,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct PatternsConfig {
    pub repeated_cd_threshold: u32,
    pub repeated_fail_threshold: u32,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct StatsConfig {
    pub session_milestone_minutes: Vec<u32>,
    pub command_milestones: Vec<u32>,
}

#[allow(dead_code)]
pub fn get_default_toml() -> &'static str {
    r#"
[companion]
pane_width_percent = 35
cooldown_seconds = 10
show_activity_feed = true
max_feed_lines = 20

[patterns]
repeated_cd_threshold = 5
repeated_fail_threshold = 3

[stats]
session_milestone_minutes = [10, 20, 30, 60]
command_milestones = [10, 25, 50, 100]
"#
}

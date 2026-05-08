use crate::daemon::stats::{Stats, CommandStats};
use chrono::{Utc};

pub struct Brain;

impl Brain {
    /// Analyzes the current stats and the last command to see if a contextual insight is warranted.
    pub fn get_insight(stats: &Stats, last_cmd: &str) -> Option<String> {
        let cmd_name = last_cmd.split_whitespace().next()?;
        let cmd_stats = stats.commands.get(cmd_name)?;

        // 1. Milestone checks (Priority 1)
        if let Some(msg) = Self::check_milestones(cmd_name, cmd_stats.count) {
            return Some(msg);
        }

        // 2. Absence checks (Priority 2)
        if let Some(msg) = Self::check_absence(cmd_name, cmd_stats) {
            return Some(msg);
        }
        
        None
    }

    fn check_milestones(name: &str, count: u64) -> Option<String> {
        match count {
            1 => Some(format!("First time using `{}`! How exciting! ✨", name)),
            5 => Some(format!("You've used `{}` 5 times now! Getting the hang of it?", name)),
            10 => Some(format!("Double digits! `{}` count is at 10! 🚀", name)),
            25 => Some(format!("Quarter century! You've run `{}` 25 times!", name)),
            50 => Some(format!("Half-century! `{}` is becoming a habit! 🏗️", name)),
            100 => Some(format!("💯! You've used `{}` 100 times! You're a pro!", name)),
            123 => Some(format!("1-2-3! `{}` count is exactly 123! Nice sequence! 🎵", name)),
            500 => Some(format!("500 uses of `{}`! You're an absolute master of this command!", name)),
            1000 => Some(format!("🌋 1,000! You've used `{}` a thousand times! Legend!", name)),
            _ => None,
        }
    }

    fn check_absence(name: &str, stats: &CommandStats) -> Option<String> {
        let now = Utc::now();
        let diff = now.signed_duration_since(stats.last_run);
        
        // We only trigger absence messages if it's been a significant while
        if diff.num_days() >= 10 {
            return Some(format!("It's been {} days since you used `{}`! Welcome back to it! 👋", diff.num_days(), name));
        } else if diff.num_days() >= 7 {
            return Some(format!("A whole week without `{}`! Did you miss it?", name));
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use chrono::{Duration, Utc};

    #[test]
    fn test_milestones() {
        let mut commands = HashMap::new();
        commands.insert("ls".to_string(), CommandStats {
            count: 123,
            last_run: Utc::now(),
        });
        let stats = Stats { commands };

        let insight = Brain::get_insight(&stats, "ls");
        assert!(insight.unwrap().contains("123"));
    }

    #[test]
    fn test_absence() {
        let mut commands = HashMap::new();
        // 10 days ago
        let ten_days_ago = Utc::now() - Duration::days(10);
        commands.insert("nyan".to_string(), CommandStats {
            count: 2,
            last_run: ten_days_ago,
        });
        let stats = Stats { commands };

        let insight = Brain::get_insight(&stats, "nyan");
        assert!(insight.unwrap().contains("10 days"));
    }
}

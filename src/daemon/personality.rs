use crate::config::personality::{Config as PersonalityConfig, Rule, Conditions};
use crate::daemon::state::State;
use crate::daemon::stats::Stats;
use rand::Rng;
use rand::seq::SliceRandom;
use chrono::Timelike;

/// Events that the personality system can react to.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum CompanionEvent {
    /// A command was executed (the full command string and cwd)
    Exec { cmd: String, cwd: String },
    /// The user cd'd to a directory
    Cd { dir: String },
    /// A command returned a non-zero exit code
    Error { cmd: String, exit_code: i32 },
    /// A GUI app was launched
    AppStart { app: String },
    /// A GUI app was closed
    AppStop { app: String },
    /// User has been idle past the idle threshold
    Idle,
    /// User has been idle past the sleep threshold
    Sleep,
    /// First command after idle/sleep
    Wake,
    /// Periodic tick — used for time-of-day and random rules
    Tick,
    /// Tmux session started / daemon started
    SessionStart,
}

/// Result of evaluating a rule.
#[derive(Debug, Clone)]
pub struct EvalResult {
    pub message: String,
    pub set_mood: Option<String>,
}

pub struct Personality {
    config: PersonalityConfig,
}

impl Personality {
    pub fn new(config: PersonalityConfig) -> Self {
        Self { config }
    }

    pub fn timing(&self) -> &crate::config::personality::TimingConfig {
        &self.config.timing
    }

    pub fn default_mood(&self) -> &str {
        &self.config.mood.default
    }

    /// Evaluate all rules against the current event and state.
    /// Returns the first matching rule's response (first-match-wins).
    pub fn evaluate(&self, event: &CompanionEvent, state: &State, stats: &Stats) -> Option<EvalResult> {
        let mut rng = rand::thread_rng();

        for rule in &self.config.rules {
            if !self.trigger_matches(rule, event, stats) {
                continue;
            }
            if !self.conditions_match(&rule.conditions, state, stats, event) {
                continue;
            }

            // Chance gate — checked last so trigger+conditions are evaluated deterministically
            if let Some(chance) = rule.conditions.chance {
                if rng.gen::<f32>() > chance {
                    continue;
                }
            }

            // Pick a random response
            if let Some(response) = rule.responses.choose(&mut rng) {
                let message = self.expand_placeholders(response, event, stats);
                return Some(EvalResult {
                    message,
                    set_mood: rule.set_mood.clone(),
                });
            }
        }

        None
    }

    /// Check if a rule's trigger matches the current event.
    fn trigger_matches(&self, rule: &Rule, event: &CompanionEvent, stats: &Stats) -> bool {
        let trigger = rule.trigger.as_str();

        match event {
            CompanionEvent::SessionStart => trigger == "session_start",
            CompanionEvent::Idle => trigger == "idle",
            CompanionEvent::Sleep => trigger == "sleep",
            CompanionEvent::Wake => trigger == "wake",
            CompanionEvent::AppStop { .. } => trigger == "app_stop",

            CompanionEvent::Exec { cmd, .. } => {
                if trigger == "random" {
                    return true;
                }
                if let Some(pattern) = trigger.strip_prefix("exec:") {
                    return command_matches(cmd, pattern);
                }
                // Milestone check: trigger = "milestone" if count is a milestone number
                if trigger == "milestone" {
                    let cmd_name = cmd.split_whitespace().next().unwrap_or(cmd);
                    if let Some(cmd_stats) = stats.commands.get(cmd_name) {
                        return is_milestone(cmd_stats.count);
                    }
                }
                // Absence check
                if trigger == "absence" {
                    let cmd_name = cmd.split_whitespace().next().unwrap_or(cmd);
                    if let Some(_cmd_stats) = stats.commands.get(cmd_name) {
                        return true; // Actual day check happens in conditions_match
                    }
                }
                false
            }

            CompanionEvent::Cd { dir } => {
                if trigger == "random" {
                    return true;
                }
                if let Some(pattern) = trigger.strip_prefix("cd:") {
                    return path_matches(dir, pattern);
                }
                false
            }

            CompanionEvent::Error { .. } => {
                trigger == "error" || trigger == "error_streak"
            }

            CompanionEvent::AppStart { app } => {
                if let Some(pattern) = trigger.strip_prefix("app_start:") {
                    return app == pattern;
                }
                trigger == "app_start"
            }

            CompanionEvent::Tick => {
                if trigger == "random" {
                    return true;
                }
                if let Some(range) = trigger.strip_prefix("time:") {
                    return time_in_range(range);
                }
                false
            }
        }
    }

    /// Check if all conditions on a rule are satisfied.
    fn conditions_match(&self, conditions: &Conditions, state: &State, stats: &Stats, event: &CompanionEvent) -> bool {
        // Mood condition
        if let Some(ref required_mood) = conditions.mood {
            if state.mood != *required_mood {
                return false;
            }
        }

        // Streak condition (for error_streak trigger)
        if let Some(required_streak) = conditions.streak {
            if state.error_streak < required_streak {
                return false;
            }
        }

        // Min count condition (for milestones)
        if let Some(min_count) = conditions.min_count {
            if let CompanionEvent::Exec { cmd, .. } = event {
                let cmd_name = cmd.split_whitespace().next().unwrap_or(cmd);
                if let Some(cmd_stats) = stats.commands.get(cmd_name) {
                    // For milestones: count must exactly equal min_count
                    if cmd_stats.count != min_count {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }

        // Absence day threshold
        if let Some(after_days) = conditions.after_days {
            if let CompanionEvent::Exec { cmd, .. } = event {
                let cmd_name = cmd.split_whitespace().next().unwrap_or(cmd);
                if let Some(cmd_stats) = stats.commands.get(cmd_name) {
                    let now = chrono::Utc::now();
                    let diff = now.signed_duration_since(cmd_stats.last_run);
                    if diff.num_days() < after_days {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }

        true
    }

    /// Expand {cmd}, {count}, {days}, {dir}, {time} placeholders.
    fn expand_placeholders(&self, template: &str, event: &CompanionEvent, stats: &Stats) -> String {
        let mut result = template.to_string();

        match event {
            CompanionEvent::Exec { cmd, cwd } => {
                let cmd_name = cmd.split_whitespace().next().unwrap_or(cmd);
                result = result.replace("{cmd}", cmd_name);
                result = result.replace("{dir}", cwd);

                if let Some(cmd_stats) = stats.commands.get(cmd_name) {
                    result = result.replace("{count}", &cmd_stats.count.to_string());
                    let now = chrono::Utc::now();
                    let days = now.signed_duration_since(cmd_stats.last_run).num_days();
                    result = result.replace("{days}", &days.to_string());
                }
            }
            CompanionEvent::Cd { dir } => {
                result = result.replace("{dir}", dir);
            }
            CompanionEvent::Error { cmd, exit_code } => {
                let cmd_name = cmd.split_whitespace().next().unwrap_or(cmd);
                result = result.replace("{cmd}", cmd_name);
                result = result.replace("{exit_code}", &exit_code.to_string());
            }
            CompanionEvent::AppStart { app } => {
                result = result.replace("{cmd}", app);
            }
            _ => {}
        }

        let now = chrono::Local::now();
        result = result.replace("{time}", &now.format("%H:%M").to_string());

        result
    }
}

/// Check if a command string matches a trigger pattern.
/// Supports exact match on the first word and prefix matching.
/// e.g. "ls" matches "ls -la", "rm" matches "rm foo.txt"
fn command_matches(cmd: &str, pattern: &str) -> bool {
    let cmd_name = cmd.split_whitespace().next().unwrap_or(cmd);
    let pattern_name = pattern.split_whitespace().next().unwrap_or(pattern);
    cmd_name == pattern_name
}

/// Check if a path matches a trigger pattern.
/// Supports:
///   "~" → matches home directory (/home/kid or $HOME)
///   "*/suffix" → matches any path ending with suffix
///   exact match
fn path_matches(path: &str, pattern: &str) -> bool {
    let path = path.trim_end_matches('/');
    let pattern = pattern.trim_end_matches('/');

    if pattern == "~" {
        // Match home directory
        return path == "/home/kid" || path == home::home_dir().map(|h| h.to_string_lossy().to_string()).unwrap_or_default();
    }

    if let Some(suffix) = pattern.strip_prefix("*") {
        return path.ends_with(suffix);
    }

    path == pattern
}

/// Check if a count is a milestone number.
fn is_milestone(count: u64) -> bool {
    matches!(count, 1 | 5 | 10 | 25 | 50 | 100 | 123 | 500 | 1000)
}

/// Parse a time range "HH:MM-HH:MM" and check if the current local time falls within it.
pub fn time_in_range(range: &str) -> bool {
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        return false;
    }

    let now = chrono::Local::now();
    let current_minutes = now.hour() * 60 + now.minute();

    if let (Some(start), Some(end)) = (parse_time_minutes(parts[0]), parse_time_minutes(parts[1])) {
        if start <= end {
            // Normal range (e.g. 06:00-09:00)
            current_minutes >= start && current_minutes <= end
        } else {
            // Wrapping range (e.g. 22:00-06:00)
            current_minutes >= start || current_minutes <= end
        }
    } else {
        false
    }
}

fn parse_time_minutes(time_str: &str) -> Option<u32> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let hours: u32 = parts[0].parse().ok()?;
    let minutes: u32 = parts[1].parse().ok()?;
    Some(hours * 60 + minutes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::stats::CommandStats;
    use std::collections::HashMap;

    fn make_state() -> State {
        State::new()
    }

    fn make_stats() -> Stats {
        Stats { commands: HashMap::new() }
    }

    fn make_personality() -> Personality {
        let config: PersonalityConfig = toml::from_str(crate::config::personality::get_default_toml()).unwrap();
        Personality::new(config)
    }

    #[test]
    fn test_command_matches() {
        assert!(command_matches("ls", "ls"));
        assert!(command_matches("ls -la", "ls"));
        assert!(command_matches("rm foo.txt", "rm"));
        assert!(!command_matches("ls", "cat"));
        assert!(!command_matches("lsof", "ls"));
    }

    #[test]
    fn test_path_matches() {
        assert!(path_matches("/home/kid", "~"));
        assert!(path_matches("/home/kid/apps", "*/apps"));
        assert!(path_matches("/home/kid/apps/gcompris", "*/apps/gcompris"));
        assert!(!path_matches("/home/kid/apps", "*/games"));
    }

    #[test]
    fn test_is_milestone() {
        assert!(is_milestone(1));
        assert!(is_milestone(10));
        assert!(is_milestone(100));
        assert!(is_milestone(1000));
        assert!(!is_milestone(2));
        assert!(!is_milestone(99));
    }

    #[test]
    fn test_time_range_parsing() {
        // We can't control the system clock, but we can test the parsing
        assert!(parse_time_minutes("06:00").is_some());
        assert!(parse_time_minutes("23:59").is_some());
        assert!(parse_time_minutes("invalid").is_none());
        assert_eq!(parse_time_minutes("06:30"), Some(390));
    }

    #[test]
    fn test_idle_trigger_fires() {
        let p = make_personality();
        let state = make_state();
        let stats = make_stats();

        let result = p.evaluate(&CompanionEvent::Idle, &state, &stats);
        assert!(result.is_some(), "Idle trigger should fire for neutral mood");
    }

    #[test]
    fn test_sleep_trigger_sets_mood() {
        let p = make_personality();
        let state = make_state();
        let stats = make_stats();

        let result = p.evaluate(&CompanionEvent::Sleep, &state, &stats);
        assert!(result.is_some());
        assert_eq!(result.unwrap().set_mood, Some("sleepy".to_string()));
    }

    #[test]
    fn test_wake_from_sleepy() {
        let p = make_personality();
        let mut state = make_state();
        state.set_mood("sleepy");
        let stats = make_stats();

        let result = p.evaluate(&CompanionEvent::Wake, &state, &stats);
        assert!(result.is_some());
        assert_eq!(result.unwrap().set_mood, Some("neutral".to_string()));
    }

    #[test]
    fn test_milestone_first_use() {
        let p = make_personality();
        let state = make_state();
        let mut stats = make_stats();
        stats.commands.insert("ls".to_string(), CommandStats {
            count: 1,
            last_run: chrono::Utc::now(),
        });

        let result = p.evaluate(
            &CompanionEvent::Exec { cmd: "ls".to_string(), cwd: "/home/kid".to_string() },
            &state,
            &stats,
        );
        assert!(result.is_some());
        assert!(result.unwrap().message.contains("First time"));
    }

    #[test]
    fn test_milestone_100() {
        let p = make_personality();
        let state = make_state();
        let mut stats = make_stats();
        stats.commands.insert("cat".to_string(), CommandStats {
            count: 100,
            last_run: chrono::Utc::now(),
        });

        let result = p.evaluate(
            &CompanionEvent::Exec { cmd: "cat".to_string(), cwd: "/home/kid".to_string() },
            &state,
            &stats,
        );
        assert!(result.is_some());
        let msg = result.unwrap().message;
        assert!(msg.contains("100") || msg.contains("💯"), "Expected milestone message, got: {}", msg);
    }

    #[test]
    fn test_no_milestone_at_arbitrary_count() {
        let p = make_personality();
        let state = make_state();
        let mut stats = make_stats();
        stats.commands.insert("pwd".to_string(), CommandStats {
            count: 7,
            last_run: chrono::Utc::now(),
        });

        let result = p.evaluate(
            &CompanionEvent::Exec { cmd: "pwd".to_string(), cwd: "/home/kid".to_string() },
            &state,
            &stats,
        );
        // At count 7, there's no milestone rule, and exec:pwd has chance 0.4,
        // plus "random" rules exist. But milestone should NOT fire.
        // We can't deterministically test random rules, so just check that
        // if a result appears, it's not a milestone message.
        if let Some(result) = result {
            assert!(!result.message.contains("First time"));
            assert!(!result.message.contains("Double digits"));
        }
    }

    #[test]
    fn test_error_streak_trigger() {
        let p = make_personality();
        let mut state = make_state();
        state.error_streak = 3;
        let stats = make_stats();

        let result = p.evaluate(
            &CompanionEvent::Error { cmd: "bad".to_string(), exit_code: 1 },
            &state,
            &stats,
        );
        assert!(result.is_some());
    }

    #[test]
    fn test_cd_home_trigger() {
        let p = make_personality();
        let state = make_state();
        let stats = make_stats();

        let result = p.evaluate(
            &CompanionEvent::Cd { dir: "/home/kid".to_string() },
            &state,
            &stats,
        );
        assert!(result.is_some());
        let msg = result.unwrap().message;
        assert!(msg.contains("Home") || msg.contains("home") || msg.contains("base"), "Got: {}", msg);
    }

    #[test]
    fn test_app_stop_trigger() {
        let p = make_personality();
        let state = make_state();
        let stats = make_stats();

        let result = p.evaluate(
            &CompanionEvent::AppStop { app: "gcompris".to_string() },
            &state,
            &stats,
        );
        assert!(result.is_some());
    }

    #[test]
    fn test_placeholder_expansion() {
        let p = make_personality();
        let mut stats = make_stats();
        stats.commands.insert("ls".to_string(), CommandStats {
            count: 42,
            last_run: chrono::Utc::now(),
        });

        let event = CompanionEvent::Exec { cmd: "ls".to_string(), cwd: "/home/kid".to_string() };
        let expanded = p.expand_placeholders("Used {cmd} {count} times at {time}", &event, &stats);
        assert!(expanded.contains("ls"));
        assert!(expanded.contains("42"));
        assert!(!expanded.contains("{time}")); // Should be replaced with actual time
    }

    #[test]
    fn test_absence_trigger() {
        let p = make_personality();
        let state = make_state();
        let mut stats = make_stats();
        stats.commands.insert("nyan".to_string(), CommandStats {
            count: 2,
            last_run: chrono::Utc::now() - chrono::Duration::days(10),
        });

        let result = p.evaluate(
            &CompanionEvent::Exec { cmd: "nyan".to_string(), cwd: "/home/kid".to_string() },
            &state,
            &stats,
        );
        assert!(result.is_some());
        let msg = result.unwrap().message;
        assert!(msg.contains("nyan") || msg.contains("days"), "Got: {}", msg);
    }

    #[test]
    fn test_session_start_trigger() {
        let p = make_personality();
        let state = make_state();
        let stats = make_stats();

        let result = p.evaluate(
            &CompanionEvent::SessionStart,
            &state,
            &stats,
        );
        assert!(result.is_some());
        let msg = result.unwrap().message;
        assert!(
            msg.contains("Welcome") || msg.contains("Hello") || msg.contains("Moo") || msg.contains("Hi"),
            "Got: {}",
            msg
        );
    }
}

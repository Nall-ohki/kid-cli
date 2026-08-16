use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use crate::daemon::state::State;
use crate::daemon::effects;
use crate::daemon::personality::{Personality, CompanionEvent};
use crate::config;

#[derive(Clone)]
pub struct Engine {
    state: Arc<Mutex<State>>,
    messages_config: Arc<config::messages::Config>,
    commands_config: Arc<config::commands::Config>,
    personality: Arc<Personality>,
    primary_pane_id: String,
}

impl Engine {
    pub fn new(primary_pane_id: String) -> Self {
        let config_dir = config::get_config_dir().unwrap_or_else(|_| std::path::PathBuf::from("/home/kid/.config/kid"));
        let msg_config = config::messages::Config::load(config_dir.join("messages.toml"))
            .unwrap_or_else(|_| {
                toml::from_str(config::messages::get_default_toml()).unwrap()
            });
        
        let cmd_config = config::commands::Config::load(config_dir.join("commands.toml"))
            .unwrap_or_else(|_| {
                config::commands::Config::default()
            });

        let personality_config = config::personality::Config::load(config_dir.join("personality.toml"))
            .unwrap_or_else(|_| {
                toml::from_str(config::personality::get_default_toml()).unwrap()
            });

        // LOAD PERSISTENT STATS
        let persistent_stats = crate::daemon::stats::Stats::load().unwrap_or_default();

        let mut state = State::new();
        state.stats = persistent_stats;
        state.mood = personality_config.mood.default.clone();

        let personality = Personality::new(personality_config);

        Self {
            state: Arc::new(Mutex::new(state)),
            messages_config: Arc::new(msg_config),
            commands_config: Arc::new(cmd_config),
            personality: Arc::new(personality),
            primary_pane_id,
        }
    }

    /// Spawn the background idle/tick loop that drives spontaneous companion behavior.
    pub fn spawn_idle_loop(&self) {
        let engine = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(20));
            loop {
                interval.tick().await;
                if let Err(e) = engine.tick().await {
                    eprintln!("Personality tick error: {}", e);
                }
            }
        });
    }

    /// Called every ~20 seconds by the background loop.
    async fn tick(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        
        // Skip if a GUI app is active
        if state.active_app.is_some() {
            return Ok(());
        }

        // Mood decay: if mood was set a while ago, drift back to default
        let mood_decay = Duration::from_secs(self.personality.timing().mood_decay_secs);
        if state.mood != self.personality.default_mood() && state.mood_last_set.elapsed() > mood_decay {
            state.set_mood(self.personality.default_mood());
        }

        let idle_secs = state.idle_duration().as_secs();
        let idle_threshold = self.personality.timing().idle_threshold_secs;
        let sleep_threshold = self.personality.timing().sleep_threshold_secs;
        let cooldown = Duration::from_secs(self.personality.timing().message_cooldown_secs);

        // Check cooldown
        if let Some(last_msg) = state.last_message_time {
            if last_msg.elapsed() < cooldown {
                return Ok(());
            }
        }

        // Determine which event to emit
        let event = if idle_secs >= sleep_threshold && !state.is_sleeping {
            state.is_sleeping = true;
            Some(CompanionEvent::Sleep)
        } else if idle_secs >= idle_threshold && !state.is_sleeping {
            Some(CompanionEvent::Idle)
        } else {
            // Time-of-day / random tick — only when not sleeping
            if !state.is_sleeping {
                Some(CompanionEvent::Tick)
            } else {
                None
            }
        };

        if let Some(event) = event {
            let stats_snapshot = state.stats.clone();
            if let Some(result) = self.personality.evaluate(&event, &state, &stats_snapshot) {
                // Apply mood change
                if let Some(ref mood) = result.set_mood {
                    state.set_mood(mood);
                }
                state.update_last_message_time();
                let current_mood = state.mood.clone();
                
                // Drop lock before IO
                drop(state);
                let _ = crate::daemon::pane::ensure_companion_pane();
                effects::show_companion_message(&result.message, &current_mood).await?;
            }
        }

        Ok(())
    }

    pub async fn trigger_session_start(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        let stats_snapshot = state.stats.clone();
        if let Some(result) = self.personality.evaluate(&CompanionEvent::SessionStart, &state, &stats_snapshot) {
            if let Some(ref mood) = result.set_mood {
                state.set_mood(mood);
            }
            state.update_last_message_time();
            let current_mood = state.mood.clone();
            drop(state);
            let _ = crate::daemon::pane::ensure_companion_pane();
            effects::show_companion_message(&result.message, &current_mood).await?;
        }
        Ok(())
    }

    pub async fn process(&self, event_type: &str, data: &str, cwd: &str, pane_id: &str) -> Result<()> {
        println!("Engine processing event: {} | data: {} | cwd: {} | pane: {}", event_type, data, cwd, pane_id);
        
        // GLOBAL FILTER: If the command didn't come from the primary pane, 
        // we record it for state tracking (counts etc) but skip ALL UI effects.
        let is_primary = pane_id == self.primary_pane_id || pane_id == "unknown";
        
        let cooldown = Duration::from_secs(self.personality.timing().message_cooldown_secs);
        
        {
            let mut state = self.state.lock().await;
            match event_type {
                "panic" | "abort" => {
                    drop(state);
                    crate::daemon::input::execute_kiosk_exit();
                    return Ok(());
                }
                "pre" => {
                    if state.active_app.is_some() {
                        return Ok(());
                    }
                    state.last_command = Some(data.to_string());
                    state.record_activity();

                    let cmd_name = data.split_whitespace().next().unwrap_or(data);
                    let is_launcher = self.commands_config.launchers.contains_key(cmd_name);

                    // Discovery event? Unconditionally ensure pane is open (ONLY IF PRIMARY)
                    if is_primary && !is_launcher && data.starts_with("ls") {
                        let _ = crate::daemon::pane::ensure_companion_pane();
                    }

                    // Wake from sleep/idle on first command
                    if is_primary && (state.is_sleeping || state.mood == "bored") {
                        let was_sleeping = state.is_sleeping;
                        state.is_sleeping = false;
                        
                        // Only send wake message if we were actually sleeping/bored
                        if was_sleeping || state.mood == "bored" {
                            let event = CompanionEvent::Wake;
                            let stats_snapshot = state.stats.clone();
                            if let Some(result) = self.personality.evaluate(&event, &state, &stats_snapshot) {
                                if let Some(ref mood) = result.set_mood {
                                    state.set_mood(mood);
                                }
                                state.update_last_message_time();
                                let current_mood = state.mood.clone();
                                drop(state);
                                let _ = crate::daemon::pane::ensure_companion_pane();
                                effects::show_companion_message(&result.message, &current_mood).await?;
                                return Ok(());
                            }
                        }
                    }
                }
                "exec" => {
                    if state.active_app.is_some() {
                        return Ok(());
                    }
                    state.record_activity();

                    // 1. UPDATE STATS
                    let cmd_name = data.split_whitespace().next().unwrap_or(data).to_string();
                    
                    if let Some(s) = state.stats.commands.get_mut(&cmd_name) {
                        s.count += 1;
                    } else {
                        state.stats.commands.insert(cmd_name.clone(), crate::daemon::stats::CommandStats {
                            count: 1,
                            last_run: chrono::Utc::now(),
                        });
                    }

                    // 2. PERSONALITY EVALUATION
                    let companion_event = if data.starts_with("cd") {
                        CompanionEvent::Cd { dir: cwd.to_string() }
                    } else {
                        CompanionEvent::Exec { cmd: data.to_string(), cwd: cwd.to_string() }
                    };

                    let stats_snapshot = state.stats.clone();
                    let personality_result = self.personality.evaluate(&companion_event, &state, &stats_snapshot);

                    // 3. Update timestamp and PERSIST
                    if let Some(s) = state.stats.commands.get_mut(&cmd_name) {
                        s.last_run = chrono::Utc::now();
                    }
                    let _ = state.stats.save();

                    // 4. ENERGETIC MOOD: detect rapid command usage
                    // If 5+ commands in the last 60 seconds, set mood to energetic
                    // (Simple heuristic: track commands_since_last_idle is effectively
                    //  idle_duration < 60s means rapid usage)
                    if state.idle_duration() < Duration::from_secs(5) && state.mood == "neutral" {
                        // Quick succession — could track a counter, but for now,
                        // let the "random" rules with mood=energetic handle it once set
                        // We'll use error_streak == 0 and recent activity as a proxy
                    }

                    // 5. DELIVER MESSAGE
                    if is_primary {
                        if let Some(result) = personality_result {
                            // Check cooldown
                            let can_send = match state.last_message_time {
                                Some(time) => time.elapsed() > cooldown,
                                None => true,
                            };
                            
                            if can_send {
                                if let Some(ref mood) = result.set_mood {
                                    state.set_mood(mood);
                                }
                                state.update_last_message_time();
                                state.last_message_is_discovery = self.commands_config.launchers.contains_key(&cmd_name);
                                let current_mood = state.mood.clone();

                                drop(state);
                                let _ = crate::daemon::pane::ensure_companion_pane();
                                effects::show_companion_message(&result.message, &current_mood).await?;
                                return Ok(());
                            }
                        }

                        // Fallback: if personality didn't fire, use legacy greeting/discovery for cd and launchers
                        let is_launcher = self.commands_config.launchers.contains_key(&cmd_name);
                        
                        if data.starts_with("cd") {
                            let _ = crate::daemon::pane::ensure_companion_pane();
                            let can_greet = match state.last_message_time {
                                Some(time) => !state.last_message_is_discovery || time.elapsed() > Duration::from_secs(30),
                                None => true,
                            };
                            if can_greet {
                                effects::trigger_greeting(&self.messages_config, cwd, None).await?;
                                state.update_last_message_time();
                                state.last_message_is_discovery = false;
                            }
                        } else if is_launcher {
                            let _ = crate::daemon::pane::ensure_companion_pane();
                            effects::trigger_discovery(&self.messages_config, data, None).await?;
                            state.update_last_message_time();
                            state.last_message_is_discovery = true;
                        }
                    }
                }
                "post" => {
                    if state.active_app.is_some() {
                        return Ok(());
                    }
                    let exit_code = data.parse().unwrap_or(0);
                    state.last_exit_code = Some(exit_code);

                    if exit_code != 0 {
                        state.error_streak += 1;

                        if is_primary {
                            let event = CompanionEvent::Error {
                                cmd: state.last_command.clone().unwrap_or_default(),
                                exit_code,
                            };
                            let stats_snapshot = state.stats.clone();
                            
                            let can_send = match state.last_message_time {
                                Some(time) => time.elapsed() > cooldown,
                                None => true,
                            };

                            if can_send {
                                if let Some(result) = self.personality.evaluate(&event, &state, &stats_snapshot) {
                                    if let Some(ref mood) = result.set_mood {
                                        state.set_mood(mood);
                                    }
                                    state.update_last_message_time();
                                    let current_mood = state.mood.clone();
                                    drop(state);
                                    let _ = crate::daemon::pane::ensure_companion_pane();
                                    effects::show_companion_message(&result.message, &current_mood).await?;
                                    return Ok(());
                                }
                            }
                        }
                    } else {
                        state.error_streak = 0;
                    }
                }
                "app_start" => {
                    state.active_app = Some(data.to_string());
                    state.record_activity();
                    let cmd_name = data.to_string();
                    if let Some(s) = state.stats.commands.get_mut(&cmd_name) {
                        s.count += 1;
                        s.last_run = chrono::Utc::now();
                    } else {
                        state.stats.commands.insert(cmd_name.clone(), crate::daemon::stats::CommandStats {
                            count: 1,
                            last_run: chrono::Utc::now(),
                        });
                    }
                    let _ = state.stats.save();
                    
                    if is_primary {
                        let event = CompanionEvent::AppStart { app: data.to_string() };
                        let stats_snapshot = state.stats.clone();
                        
                        if let Some(result) = self.personality.evaluate(&event, &state, &stats_snapshot) {
                            if let Some(ref mood) = result.set_mood {
                                state.set_mood(mood);
                            }
                            state.update_last_message_time();
                            let current_mood = state.mood.clone();
                            drop(state);
                            let _ = crate::daemon::pane::ensure_companion_pane();
                            effects::show_companion_message(&result.message, &current_mood).await?;
                            return Ok(());
                        }
                        
                        // Fallback to legacy discovery
                        let _ = crate::daemon::pane::ensure_companion_pane();
                        effects::trigger_discovery(&self.messages_config, &format!("launch {}", data), None).await?;
                        state.update_last_message_time();
                        state.last_message_is_discovery = true;
                    }
                }
                "app_stop" => {
                    state.active_app = None;
                    state.record_activity();
                    
                    if is_primary {
                        let event = CompanionEvent::AppStop { app: data.to_string() };
                        let stats_snapshot = state.stats.clone();
                        
                        if let Some(result) = self.personality.evaluate(&event, &state, &stats_snapshot) {
                            if let Some(ref mood) = result.set_mood {
                                state.set_mood(mood);
                            }
                            state.update_last_message_time();
                            let current_mood = state.mood.clone();
                            drop(state);
                            let _ = crate::daemon::pane::ensure_companion_pane();
                            effects::show_companion_message(&result.message, &current_mood).await?;
                            return Ok(());
                        }

                        // Fallback
                        effects::trigger_greeting(&self.messages_config, cwd, None).await?;
                        state.update_last_message_time();
                        state.last_message_is_discovery = false;
                    }
                }
                _ => {}
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_counting() {
        // Use a temporary file for stats in test
        let temp_dir = std::env::temp_dir();
        let stats_path = temp_dir.join("kid_test_counting_stats.toml");
        let _ = std::fs::remove_file(&stats_path);
        std::env::set_var("KID_STATS_PATH", stats_path.to_str().unwrap());

        // Use "primary" as the ID, but send events from "other" to skip UI effects in test
        let engine = Engine::new("primary".to_string());
        
        // Prevent companion message from triggering during test (safety fallback)
        // and clear any stats loaded from previous test runs
        {
            let mut state = engine.state.lock().await;
            state.update_last_message_time();
            state.stats = crate::daemon::stats::Stats::default();
        }

        engine.process("exec", "ls", "/home/kid", "other").await.unwrap();
        engine.process("exec", "cd", "/home/kid", "other").await.unwrap();
        
        let state = engine.state.lock().await;
        assert_eq!(state.stats.commands.get("ls").unwrap().count, 1);
        assert_eq!(state.stats.commands.get("cd").unwrap().count, 1);
        
        // Cleanup after test
        std::env::remove_var("KID_STATS_PATH");
        let _ = std::fs::remove_file(&stats_path);
    }

    #[tokio::test]
    async fn test_engine_error_streak() {
        let temp_dir = std::env::temp_dir();
        let stats_path = temp_dir.join("kid_test_error_streak.toml");
        let _ = std::fs::remove_file(&stats_path);
        std::env::set_var("KID_STATS_PATH", stats_path.to_str().unwrap());

        let engine = Engine::new("primary".to_string());
        {
            let mut state = engine.state.lock().await;
            state.update_last_message_time();
        }

        // Non-zero exit codes should increment error_streak
        engine.process("pre", "badcmd", "/home/kid", "other").await.unwrap();
        engine.process("post", "1", "/home/kid", "other").await.unwrap();
        engine.process("post", "1", "/home/kid", "other").await.unwrap();
        engine.process("post", "1", "/home/kid", "other").await.unwrap();
        
        {
            let state = engine.state.lock().await;
            assert_eq!(state.error_streak, 3);
        }

        // Zero exit code resets streak
        engine.process("post", "0", "/home/kid", "other").await.unwrap();
        {
            let state = engine.state.lock().await;
            assert_eq!(state.error_streak, 0);
        }

        std::env::remove_var("KID_STATS_PATH");
        let _ = std::fs::remove_file(&stats_path);
    }
}

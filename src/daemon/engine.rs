use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use crate::daemon::state::State;
use crate::daemon::effects;
use crate::config;

#[derive(Clone)]
pub struct Engine {
    state: Arc<Mutex<State>>,
    messages_config: Arc<config::messages::Config>,
    commands_config: Arc<config::commands::Config>,
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

        // LOAD PERSISTENT STATS
        let persistent_stats = crate::daemon::stats::Stats::load().unwrap_or_default();

        let mut state = State::new();
        state.stats = persistent_stats;

        Self {
            state: Arc::new(Mutex::new(state)),
            messages_config: Arc::new(msg_config),
            commands_config: Arc::new(cmd_config),
            primary_pane_id,
        }
    }

    pub async fn process(&self, event_type: &str, data: &str, cwd: &str, pane_id: &str) -> Result<()> {
        println!("Engine processing event: {} | data: {} | cwd: {} | pane: {}", event_type, data, cwd, pane_id);
        
        // GLOBAL FILTER: If the command didn't come from the primary pane, 
        // we record it for state tracking (counts etc) but skip ALL UI effects.
        let is_primary = pane_id == self.primary_pane_id || pane_id == "unknown";
        
        let mut trigger = false;
        let mut brain_insight = None;
        
        {
            let mut state = self.state.lock().await;
            match event_type {
                "pre" => {
                    state.last_command = Some(data.to_string());
                    
                    let cmd_name = data.split_whitespace().next().unwrap_or(data);
                    let is_launcher = self.commands_config.launchers.contains_key(cmd_name);

                    // Discovery event? Unconditionally ensure pane is open (ONLY IF PRIMARY)
                    if is_primary && !is_launcher && data.starts_with("ls") {
                        let _ = crate::daemon::pane::ensure_companion_pane();
                    }

                    if is_primary && !is_launcher && (state.should_show_companion() || data.starts_with("cd")) {
                        trigger = true;
                    }
                }
                "exec" => {
                    // 1. UPDATE STATS
                    let cmd_name = data.split_whitespace().next().unwrap_or(data).to_string();
                    
                    // We split the updates to avoid borrow checker issues with the Brain call
                    if let Some(s) = state.stats.commands.get_mut(&cmd_name) {
                        s.count += 1;
                    } else {
                        state.stats.commands.insert(cmd_name.clone(), crate::daemon::stats::CommandStats {
                            count: 1,
                            last_run: chrono::Utc::now(),
                        });
                    }
                    
                    // 2. BRAIN CHECK (uses NEW count but OLD timestamp for absence check)
                    brain_insight = crate::daemon::brain::Brain::get_insight(&state.stats, data);
                    
                    // 3. Update timestamp and PERSIST
                    if let Some(s) = state.stats.commands.get_mut(&cmd_name) {
                        s.last_run = chrono::Utc::now();
                    }
                    let _ = state.stats.save();

                    // Discovery trigger: ONLY if it's a registered launcher or starts with cd
                    if data.starts_with("cd") {
                         // Unconditionally ensure pane (ONLY IF PRIMARY)
                         if is_primary {
                            let _ = crate::daemon::pane::ensure_companion_pane();
                         }

                         // Pin discovery messages for 30 seconds
                         let can_greet = match state.last_message_time {
                             Some(time) => !state.last_message_is_discovery || time.elapsed() > Duration::from_secs(30),
                             None => true,
                         };
                         if is_primary && can_greet {
                             effects::trigger_greeting(&self.messages_config, cwd, brain_insight.clone()).await?;
                             state.update_last_message_time();
                             state.last_message_is_discovery = false;
                         }
                    } else if self.commands_config.launchers.contains_key(&cmd_name) {
                         // Unconditionally ensure pane (ONLY IF PRIMARY)
                         if is_primary {
                            let _ = crate::daemon::pane::ensure_companion_pane();
                         }

                         // Only trigger discovery for launchers, skip generic greeting
                         if is_primary {
                             effects::trigger_discovery(&self.messages_config, data, brain_insight.clone()).await?;
                             state.update_last_message_time();
                             state.last_message_is_discovery = true;
                         }
                    }
                }
                "post" => {
                    state.last_exit_code = Some(data.parse().unwrap_or(0));
                }
                _ => {}
            }
        }

        if is_primary && trigger {
            effects::trigger_greeting(&self.messages_config, cwd, brain_insight).await?;
            
            let mut state = self.state.lock().await;
            state.update_last_message_time();
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
        let stats_path = temp_dir.join("kid_test_stats.toml");
        let _ = std::fs::remove_file(&stats_path);
        std::env::set_var("KID_STATS_PATH", stats_path.to_str().unwrap());

        // Use "primary" as the ID, but send events from "other" to skip UI effects in test
        let engine = Engine::new("primary".to_string());
        
        // Prevent companion message from triggering during test (safety fallback)
        {
            let mut state = engine.state.lock().await;
            state.update_last_message_time();
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
}

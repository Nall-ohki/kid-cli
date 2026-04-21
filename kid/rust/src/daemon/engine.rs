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

        Self {
            state: Arc::new(Mutex::new(State::new())),
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
        
        {
            let mut state = self.state.lock().await;
            match event_type {
                "pre" => {
                    state.last_command = Some(data.to_string());
                    state.command_count += 1;
                    
                    if data.starts_with("ls") {
                        state.ls_count += 1;
                    }
                    
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
                             effects::trigger_greeting(&self.messages_config, cwd).await?;
                             state.update_last_message_time();
                             state.last_message_is_discovery = false;
                         }
                    } else {
                         // Check if the command (first word of data) is in our launchers
                         let cmd_name = data.split_whitespace().next().unwrap_or(data);
                         if self.commands_config.launchers.contains_key(cmd_name) {
                             // Unconditionally ensure pane (ONLY IF PRIMARY)
                             if is_primary {
                                let _ = crate::daemon::pane::ensure_companion_pane();
                             }

                             // Only trigger discovery for launchers, skip generic greeting
                             if is_primary {
                                 effects::trigger_discovery(&self.messages_config, data).await?;
                                 state.update_last_message_time();
                                 state.last_message_is_discovery = true;
                             }
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
            effects::trigger_greeting(&self.messages_config, cwd).await?;
            
            let mut state = self.state.lock().await;
            state.update_last_message_time();
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_engine_counting() {
        let engine = Engine::new();
        
        // Prevent companion message from triggering during test
        // by setting a recent message time
        {
            let mut state = engine.state.lock().await;
            state.update_last_message_time();
        }

        engine.process("pre", "ls").await.unwrap();
        engine.process("pre", "cd").await.unwrap();
        
        let state = engine.state.lock().await;
        assert_eq!(state.command_count, 2);
        assert_eq!(state.ls_count, 1);
    }
}

use anyhow::Result;
use crate::config;
use crate::commands;

pub async fn handle_busybox(full_name: &str, args: &[String]) -> Result<()> {
    // 1. Get basename for matching
    let name = std::path::Path::new(full_name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(full_name);

    // 2. Get config directory
    let config_dir = config::get_config_dir()?;
    let commands_config = config::commands::Config::load(config_dir.join("commands.toml"))?;

    // 3. Dispatch based on taxonomy

    // A. Validators (Hardcoded since they use functions)
    match name {
        "cd" => return commands::validate::cd(args),
        "home" => return commands::validate::home(),
        "exit" => return commands::validate::exit(),
        "clear" => return commands::validate::clear(),
        "msg-pipe" | "kid-msg-pipe" => {
            use std::io::{self, Read};
            let mut buffer = String::new();
            if io::stdin().read_to_string(&mut buffer).is_ok() {
                let pipe_path = "/tmp/kid_companion_pipe";
                if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(pipe_path) {
                    use std::io::Write;
                    let _ = write!(file, "{}", buffer);
                }
            }
            return Ok(());
        }
        _ => {}
    }

    // B. Launchers (TOML)
    if let Some(launcher) = commands_config.launchers.get(name) {
        if !launcher.enabled {
            return Err(anyhow::anyhow!("Command '{}' is currently disabled.", name));
        }
        return commands::launch::run(name, launcher, args).await;
    }

    // B2. Games (TOML)
    if let Some(game) = commands_config.games.get(name) {
        if !game.enabled {
            return Err(anyhow::anyhow!("Game '{}' is currently disabled.", name));
        }
        if let Some(system) = commands_config.systems.get(&game.system) {
            let rom_path = format!("{}/{}", system.rom_dir, game.rom);
            let binary = system.template
                .replace("{rom_dir}", &system.rom_dir)
                .replace("{rom_path}", &rom_path);

            let mut launcher = config::commands::LauncherConfig::default();
            launcher.binary = Some(binary);
            launcher.gui = true;
            launcher.pane = "none".to_string();
            
            return commands::launch::run(name, &launcher, args).await;
        } else {
            return Err(anyhow::anyhow!("System '{}' not found for game '{}'", game.system, name));
        }
    }

    // C. Passthroughs (TOML)
    if let Some(real_binary) = commands_config.passthroughs.get(name) {
        return commands::passthrough::run(name, real_binary, args).await;
    }

    // D. Blocks (TOML) - Check BEFORE fallbacks for security
    if commands_config.blocks.commands.contains(&name.to_string()) {
        return commands::block::run(name, &commands_config.blocks);
    }

    // E. Hardcoded Fallback for Emergency Passthroughs (if not in TOML)
    let emergency_fallbacks = [
        ("ls", "/bin/ls"), ("cat", "/usr/bin/cat"), ("grep", "/bin/grep"),
        ("mkdir", "/bin/mkdir"), ("rmdir", "/bin/rmdir"), ("rm", "/bin/rm"),
        ("cp", "/bin/cp"), ("mv", "/bin/mv"), ("pwd", "/bin/pwd"),
        ("wc", "/usr/bin/wc"), ("head", "/usr/bin/head"), ("tail", "/usr/bin/tail"),
        ("echo", "/usr/bin/echo"), ("whoami", "/usr/bin/whoami"), ("date", "/usr/bin/date"),
        ("clear", "/usr/bin/clear"), ("reset", "/usr/bin/reset"), ("groups", "/usr/bin/groups"),
        ("id", "/usr/bin/id"), ("cal", "/usr/bin/cal"), ("man", "/usr/bin/man"),
        ("touch", "/usr/bin/touch"), ("file", "/usr/bin/file"), ("less", "/usr/bin/less"),
        ("uv", "/usr/local/bin/uv")
    ];

    for (cmd_name, real_path) in emergency_fallbacks {
        if name == cmd_name {
            return commands::passthrough::run(name, real_path, args).await;
        }
    }

    // E. Default behavior: block unregistered commands
    Err(anyhow::anyhow!("Command '{}' not found in kid environment registry.", name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_busybox_routing_basic() {
        // This is hard to test without a real config file on disk
        // in the current implementation. 
        // For now, we verify that hardcoded validators work.
        // We'll need a way to mock Config::load for deeper tests.
        assert!(handle_busybox("cd", &["/non_existent_directory_for_test".to_string()]).await.is_err());
    }
}

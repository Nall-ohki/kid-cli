use anyhow::Result;
use crate::config::commands::{LauncherConfig, LolcatMode};

pub async fn run(name: &str, config: &LauncherConfig, args: &[String]) -> Result<()> {
    let mut binary = config.binary.as_deref().unwrap_or(name).to_string();
    
    // Resolve {KID} placeholder if present
    if binary.contains("{KID}") {
        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(path_str) = current_exe.to_str() {
                binary = binary.replace("{KID}", path_str);
            }
        }
    }

    let mut full_cmd = binary.clone();
    if !args.is_empty() {
        full_cmd.push(' ');
        full_cmd.push_str(&args.join(" "));
    }

    // Command is now ready for execution

    // Handle lolcat formatting
    if let LolcatMode::Simple(ref mode) = config.lolcat {
        if mode == "always" {
            full_cmd = format!("{} | /usr/games/lolcat", full_cmd);
        }
    }

    // Handle persist
    if config.persist && config.pane != "companion" {
        full_cmd = format!("{}; echo \"\"; echo \"Press Enter to close...\"; read dummy", full_cmd);
    }


    // 1. Notify the daemon of application start
    let _ = crate::commands::event::run("app_start", name, None).await;

    // 2. Open tmux pane/popup or run directly/GUI-mode
    // CRITICAL: If stdin is NOT a TTY (e.g. echo test | say), we MUST run directly
    // because tmux split-window cannot inherit stdin easily.
    use std::os::unix::io::AsRawFd;
    let is_pipe = !nix::unistd::isatty(std::io::stdin().as_raw_fd()).unwrap_or(false);
    let has_tmux = std::env::var("TMUX").is_ok() && !is_pipe && config.pane != "none";
    let tmux_bin = "/usr/bin/tmux";

    let infra_path = std::env::var("_INFRA_PATH").unwrap_or_else(|_| "/usr/bin:/bin:/usr/local/bin:/usr/games".to_string());

    let execution_result = if config.gui {
        let has_display = std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("DISPLAY").is_ok();
        
        let gui_cmd = if has_display {
            // We are already inside a graphical session (e.g. kid's native cage, or a desktop environment)
            // Just run the app directly, it will attach to the existing display server automatically.
            full_cmd.clone()
        } else {
            // We are in a raw TTY or SSH session. Start a Wayland compositor (cage) to host the graphical app.
            format!("/usr/bin/cage -s -d -- {}", full_cmd)
        };

        std::process::Command::new("/bin/sh")
            .env("PATH", &infra_path)
            .arg("-c")
            .arg(gui_cmd)
            .status()
    } else if has_tmux {
        let full_cmd_escaped = full_cmd.replace("\"", "\\\"");
        let target = std::env::var("TMUX_PANE").unwrap_or_else(|_| "kid".to_string());
        
        let tmux_cmd = if config.pane == "popup" {
            format!("{} display-popup -t {} -E -w 80% -h 80% \"PATH=\\\"{}\\\" {}\"", tmux_bin, target, infra_path, full_cmd_escaped)
        } else if config.pane == "companion" {
             if config.persist {
                 format!("( PATH=\"{}\" {} ; echo \"\" ; echo \"Press Enter to close...\" ) | /kid/bin/kid-msg-pipe ; read dummy", infra_path, full_cmd)
             } else {
                 format!("PATH=\"{}\" {} | /kid/bin/kid-msg-pipe", infra_path, full_cmd)
             }
        } else if config.pane == "bottom" {
            format!("{} split-window -d -t {} -v -f -p 35 \"PATH=\\\"{}\\\" {}\"", tmux_bin, target, infra_path, full_cmd_escaped)
        } else {
            format!("{} split-window -t {} -v -p 35 \"PATH=\\\"{}\\\" {}\"", tmux_bin, target, infra_path, full_cmd_escaped)
        };

        std::process::Command::new("/bin/sh")
            .env("PATH", &infra_path)
            .arg("-c")
            .arg(tmux_cmd)
            .status()
    } else {
        std::process::Command::new("/bin/sh")
            .env("PATH", &infra_path)
            .arg("-c")
            .arg(full_cmd)
            .status()
    };

    // 3. Notify the daemon of application stop
    let _ = crate::commands::event::run("app_stop", name, None).await;

    let status = execution_result?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Command exited with status: {}", status))
    }
}

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
    if config.persist {
        full_cmd = format!("{}; echo \"\"; echo \"Press Enter to close...\"; read dummy", full_cmd);
    }


    // 2. Open tmux pane/popup and run command
    // CRITICAL: If stdin is NOT a TTY (e.g. echo test | say), we MUST run directly
    // because tmux split-window cannot inherit stdin easily.
    use std::os::unix::io::AsRawFd;
    let is_pipe = !nix::unistd::isatty(std::io::stdin().as_raw_fd()).unwrap_or(false);
    let has_tmux = std::env::var("TMUX").is_ok() && !is_pipe && config.pane != "none";
    let tmux_bin = "/usr/bin/tmux";

    let status = if has_tmux {
        let full_cmd_escaped = full_cmd.replace("\"", "\\\"");
        let target = std::env::var("TMUX_PANE").unwrap_or_else(|_| "kid".to_string());
        
        let tmux_cmd = if config.pane == "popup" {
            format!("{} display-popup -t {} -E -w 80% -h 80% \"{}\"", tmux_bin, target, full_cmd_escaped)
        } else if config.pane == "companion" {
             // Redirection mode: Run command and pipe output to kid companion
             // Instead of a shell, we just send text to the coach.
             format!("{} | /kid/bin/kid-msg-pipe", full_cmd)
        } else if config.pane == "bottom" {
            // Explicitly split at the full width of the bottom and select it
            // -d flag ensures focus stays on the user pane
            format!("{} split-window -d -t {} -v -f -p 35 \"{}\"", tmux_bin, target, full_cmd_escaped)
        } else {
            // Standard vertical split
            format!("{} split-window -t {} -v -p 35 \"{}\"", tmux_bin, target, full_cmd_escaped)
        };

        std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(tmux_cmd)
            .status()?
    } else {
        // Fallback to direct execution if not in tmux, explicitly 'none', or piped
        // Use /bin/sh -c to safely handle commands with arguments (like /bin/ls -la)
        std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(full_cmd)
            .status()?
    };

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Command exited with status: {}", status))
    }
}

use anyhow::{Result, anyhow};
use std::path::Path;
use crate::terminal::{styled_message, MessageLevel};

pub fn cd(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Ok(());
    }

    let target = &args[0];
    let path = Path::new(target);

    if !path.exists() {
        styled_message(MessageLevel::Error, &format!("Directory does not exist: {}", target));
        return Err(anyhow!("Directory not found"));
    }

    if !path.is_dir() {
        styled_message(MessageLevel::Error, &format!("Not a directory: {}", target));
        return Err(anyhow!("Not a directory"));
    }

    Ok(())
}

pub fn exit() -> Result<()> {
    // Logic to kill tmux session cleanly
    // For now, just a placeholder
    styled_message(MessageLevel::Info, "Cleaning up and exiting session...");
    std::process::Command::new("/usr/bin/tmux")
        .arg("kill-session")
        .status()?;
    Ok(())
}

pub fn home() -> Result<()> {
    styled_message(MessageLevel::Ok, "Heading home! 🏠");
    Ok(())
}

pub fn clear() -> Result<()> {
    let tmux = "/usr/bin/tmux";
    
    // 1. Clear the primary terminal screen
    let _ = std::process::Command::new("/usr/bin/clear").status();

    // 2. Signal the Companion TUI to reset its state
    // We don't want to block the user if the pipe is busy
    let pipe_path = "/tmp/kid_companion_pipe";
    if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(pipe_path) {
        use std::io::Write;
        let _ = writeln!(file, "COMMAND:RESET");
    }

    // 3. Kill all other panes (except User and Companion)
    // First, find all panes and their titles
    let output = std::process::Command::new(tmux)
        .arg("list-panes")
        .arg("-F")
        .arg("#{pane_id} #{pane_title}")
        .output()?;
        
    if output.status.success() {
        let panes_str = String::from_utf8(output.stdout)?;
        for line in panes_str.lines() {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() == 2 {
                let id = parts[0];
                let title = parts[1].to_lowercase();
                
                // Keep 'User' and 'Companion' (and any pane labeled kid-companion)
                if !title.contains("user") && !title.contains("companion") && !title.contains("kid-companion") {
                    let _ = std::process::Command::new(tmux)
                        .arg("kill-pane")
                        .arg("-t")
                        .arg(id)
                        .status();
                }
            }
        }
    }

    Ok(())
}

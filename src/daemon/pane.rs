use std::process::Command;
use anyhow::{Result, anyhow};

const PANE_TITLE: &str = "kid-companion";

pub async fn show_message(text: &str) -> Result<()> {
    // 1. Ensure companion pane exists
    let _ = ensure_companion_pane()?;
    
    // 2. Write to the pipe
    let pipe_path = "/tmp/kid_companion_pipe";
    
    // We use a separate thread/task for writing to avoid blocking the daemon
    // if the reader is busy or closed.
    let text = text.to_string();
    tokio::task::spawn_blocking(move || {
        use std::io::Write;
        // Optimization: if we can't open for writing, it might mean the reader isn't there yet.
        // We'll try with a short timeout or just fail silently to avoid hanging.
        if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(pipe_path) {
            let _ = writeln!(file, "{}", text);
        }
    });

    Ok(())
}

fn get_active_pane(tmux: &str) -> String {
    // Try environment first
    if let Ok(pane) = std::env::var("TMUX_PANE") {
        return pane;
    }
    
    // Fallback: ask tmux for the active pane in the current session
    let output = Command::new(tmux)
        .arg("display-message")
        .arg("-p")
        .arg("#{pane_id}")
        .output();
        
    if let Ok(out) = output {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return s;
        }
    }
    
    "kid".to_string() // Absolute fallback
}

pub fn ensure_companion_pane() -> Result<String> {
    let tmux = "/usr/bin/tmux";
    let target = get_active_pane(tmux);
    
    let mut debug_log = format!("TARGET: {}\n", target);

    // 1. Try to find pane with title "Companion" across all panes globally
    let output = Command::new(tmux)
        .arg("list-panes")
        .arg("-a")
        .arg("-F")
        .arg("#{pane_id} #{pane_title}")
        .output()?;
        
    if output.status.success() {
        let panes = String::from_utf8(output.stdout)?;
        for line in panes.lines() {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() == 2 && parts[1].to_lowercase().contains("companion") {
                return Ok(parts[0].to_string());
            }
        }
    }
    
    // 2. If not found or list-panes failed because of no session yet, we might need to wait or just use last
    let mut cmd = Command::new(tmux);
    cmd.arg("list-panes");
    if !target.is_empty() { cmd.arg("-t").arg(&target); }
    let output = cmd.arg("-F").arg("#{pane_id}").output()?;
        
    if !output.status.success() {
        return Err(anyhow!("Tmux list-panes failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let panes_str = String::from_utf8(output.stdout)?;
    let pane_ids: Vec<&str> = panes_str.lines().collect();
    let initial_count = pane_ids.len();
    let mut debug_log = format!("TARGET: {}\nPANE_COUNT: {}\n", target, initial_count);

    if initial_count == 1 {
        let split_out = Command::new(tmux)
            .arg("split-window")
            .arg("-t")
            .arg(&target)
            .arg("-h")
            .arg("-p")
            .arg("35")
            .arg("-d")
            .arg("/kid/bin/kid companion")
            .output()?;
            
        debug_log.push_str(&format!("SPLIT_STATUS: {}\nSPLIT_ERR: {}\n", split_out.status, String::from_utf8_lossy(&split_out.stderr)));
            
        // Re-list to get the new pane
        let mut cmd = Command::new(tmux);
        cmd.arg("list-panes");
        if !target.is_empty() { cmd.arg("-t").arg(&target); }
        let output = cmd.arg("-F").arg("#{pane_id}").output()?;
        if !output.status.success() {
             return Err(anyhow!("Second tmux list-panes failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
        let updated_str = String::from_utf8(output.stdout)?;
        let updated_ids: Vec<&str> = updated_str.lines().collect();
        debug_log.push_str(&format!("RE-LIST: {}\n", updated_str));
        
        let _ = std::fs::write("/tmp/pane_debug.txt", &debug_log);
        
        if updated_ids.len() > initial_count {
            if let Some(last_id) = updated_ids.last() {
                let last_id = last_id.trim();
                // DOUBLE CHECK: Never rename the source pane
                if last_id != target {
                    // NEW: Brand the source pane as "User" so clear() doesn't kill it
                    let _ = Command::new(tmux)
                        .arg("select-pane")
                        .arg("-t")
                        .arg(&target)
                        .arg("-T")
                        .arg("User")
                        .status();

                    Command::new(tmux)
                        .arg("select-pane")
                        .arg("-t")
                        .arg(last_id)
                        .arg("-T")
                        .arg("Companion")
                        .status()?;
                    return Ok(last_id.to_string());
                }
            }
        }
        return Err(anyhow!("Tmux split-window reported success but no new pane was found. Check if '/kid/bin/kid companion' is failing."));
    }
    
    // NEW: Also brand on fallback path
    let last_id = pane_ids.last().unwrap().trim();
    if target != last_id {
        let _ = Command::new(tmux)
            .arg("select-pane")
            .arg("-t")
            .arg(&target)
            .arg("-T")
            .arg("User")
            .status();
    }
    
    Command::new(tmux)
        .arg("select-pane")
        .arg("-t")
        .arg(last_id)
        .arg("-T")
        .arg("Companion")
        .status()?;

    Ok(last_id.to_string())
}

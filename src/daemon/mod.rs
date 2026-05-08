pub mod socket;
pub mod pane;
pub mod engine;
pub mod state;
pub mod effects;
pub mod stats;
pub mod brain;

use std::fs;
use std::path::PathBuf;
use anyhow::Result;
use std::process::{Command, Stdio};

pub fn start() -> Result<()> {
    let pid_file = get_pid_file()?;
    
    if pid_file.exists() {
        if let Ok(content) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = content.trim().parse::<i32>() {
                // Preemptive Take-over: Try to kill existing process
                println!("Found existing daemon (PID {}). Terminating for take-over...", pid);
                let _ = std::process::Command::new("/bin/kill").arg("-9").arg(pid.to_string()).status();
                // Brief pause to allow OS to reclaim resources/sockets
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        let _ = fs::remove_file(&pid_file);
    }

    println!("Starting kid watch daemon...");
    
    // For simplicity in this environment, we'll use a basic spawn for now
    // A full daemonize would involve double-fork, but let's see if this suffices
    use std::os::unix::process::CommandExt;
    
    let mut cmd = Command::new(std::env::current_exe()?);
    
    unsafe {
        cmd.pre_exec(|| {
            let _ = nix::unistd::setsid();
            Ok(())
        });
    }

    let home = home::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    
    let child = cmd
        .arg("watch")
        .stdin(Stdio::null())
        .stdout(fs::File::create(home.join(".kid_watch.log"))?)
        .stderr(fs::File::create(home.join(".kid_watch.err"))?)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("HOME", std::env::var("HOME").unwrap_or_default())
        .env("TMUX", std::env::var("TMUX").unwrap_or_default())
        .env("TMUX_PANE", std::env::var("TMUX_PANE").unwrap_or_default())
        .spawn()?;

    fs::write(&pid_file, child.id().to_string())?;
    println!("Daemon started with PID {}", child.id());
    Ok(())
}

pub fn get_pid_file() -> Result<PathBuf> {
    let home = home::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home.join(".kid_watch.pid"))
}

pub async fn run_server(primary_pane_id: String) -> Result<()> {
    // Brand the primary pane
    if primary_pane_id != "unknown" {
        let _ = std::process::Command::new("/usr/bin/tmux")
            .arg("select-pane")
            .arg("-t")
            .arg(&primary_pane_id)
            .arg("-T")
            .arg("User")
            .status();
    }

    // This will be called by 'kid watch' (without --daemon)
    // or by the background process
    socket::listen(primary_pane_id).await
}

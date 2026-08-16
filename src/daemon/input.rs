use evdev::{Device, KeyCode, EventSummary};
use std::time::Duration;
use std::path::Path;
use tokio::time::sleep;

pub async fn monitor_inputs() {
    println!("Starting evdev input monitor for Kiosk Exit (F10 / F12)");
    
    // In a real system, we might want to use udev or inotify to watch for new devices,
    // but for kiosk mode, periodically scanning available devices is robust enough.
    loop {
        // Find all input devices
        let mut devices = vec![];
        for i in 0..32 {
            let path_str = format!("/dev/input/event{}", i);
            let path = Path::new(&path_str);
            if path.exists() {
                match Device::open(path) {
                    Ok(device) => {
                        if device.supported_keys().map_or(false, |keys| {
                            keys.contains(KeyCode::KEY_F10) || keys.contains(KeyCode::KEY_F12)
                        }) {
                            println!("Found suitable device: {} at {}", device.name().unwrap_or("Unknown"), path_str);
                            devices.push(device);
                        }
                    }
                    Err(_) => {
                        // Could be permission denied
                        // println!("Failed to open {}: {}", path_str, e);
                    }
                }
            }
        }

        if devices.is_empty() {
            // println!("No F12 devices found, rescanning in 5s...");
            sleep(Duration::from_secs(5)).await;
            continue;
        }
        
        println!("Started listening on {} devices", devices.len());

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);

        // Spawn a blocking thread for each device to read events
        for mut device in devices {
            let tx = tx.clone();
            tokio::task::spawn_blocking(move || {
                loop {
                    match device.fetch_events() {
                        Ok(events) => {
                            for event in events {
                                match event.destructure() {
                                    EventSummary::Key(_, KeyCode::KEY_F10 | KeyCode::KEY_F12, 1) => {
                                        let _ = tx.blocking_send(true); // Key Down
                                    }
                                    EventSummary::Key(_, KeyCode::KEY_F10 | KeyCode::KEY_F12, 0) => {
                                        let _ = tx.blocking_send(false); // Key Up
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            });
        }

        // Process events asynchronously with a 2-second hold timeout
        let mut is_pressed = false;
        loop {
            if is_pressed {
                match tokio::time::timeout(Duration::from_millis(2000), rx.recv()).await {
                    Ok(Some(false)) => {
                        // Key released before 2 seconds
                        is_pressed = false;
                    }
                    Ok(Some(true)) => {
                        // Another key down (autorepeat or multiple keyboards), ignore
                    }
                    Ok(None) => {
                        // Channels closed
                        break;
                    }
                    Err(_) => {
                        // Timeout reached! F10/F12 has been held for 2 seconds
                        println!("Panic Hotkey (F10/F12) detected! Executing Kiosk Exit.");
                        execute_kiosk_exit();
                        is_pressed = false; // Reset state
                    }
                }
            } else {
                match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
                    Ok(Some(true)) => {
                        is_pressed = true;
                    }
                    Ok(Some(false)) => {}
                    Ok(None) => {
                        break;
                    }
                    Err(_) => {
                        // Timeout reached while idle, break to rescan devices (hotplug support)
                    }
                }
            }
        }
        
        // If we reach here, we need to rescan devices (maybe one was unplugged and threw an error)
        sleep(Duration::from_secs(2)).await;
    }
}

pub fn execute_kiosk_exit() {
    println!("Panic / Kiosk Exit triggered! Terminating active GUI games and applications...");
    let targets = [
        "mame", "retroarch", "gcompris-qt", "gcompris", "scratch", 
        "tuxpaint", "tuxmath", "tuxtype", "klettres", "cmatrix", "nyancat"
    ];
    
    let mut app_killed = false;
    
    // 1. Native /proc scanning and direct SIGKILL (does not rely on $PATH or external binaries)
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if let Ok(pid) = file_name.parse::<i32>() {
                        // Don't kill ourselves or init
                        if pid <= 1 || pid == std::process::id() as i32 {
                            continue;
                        }

                        let cmdline_path = entry.path().join("cmdline");
                        if let Ok(cmdline) = std::fs::read_to_string(&cmdline_path) {
                            for target in &targets {
                                if cmdline.contains(target) {
                                    println!("Killing matched process '{}' (PID {})", target, pid);
                                    let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
                                    app_killed = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Fallback using system binaries with explicit full paths
    for target in &targets {
        for pkill_bin in &["/usr/bin/pkill", "/bin/pkill"] {
            if std::path::Path::new(pkill_bin).exists() {
                if let Ok(status) = std::process::Command::new(pkill_bin)
                    .arg("-9")
                    .arg("-f")
                    .arg(target)
                    .status() 
                {
                    if status.success() {
                        app_killed = true;
                    }
                }
            }
        }
        for killall_bin in &["/usr/bin/killall", "/bin/killall"] {
            if std::path::Path::new(killall_bin).exists() {
                if let Ok(status) = std::process::Command::new(killall_bin)
                    .arg("-9")
                    .arg(target)
                    .status() 
                {
                    if status.success() {
                        app_killed = true;
                    }
                }
            }
        }
    }

    // 3. Clear terminal and close popup panes
    println!("Kiosk exit complete (app_killed: {}). Clearing screen.", app_killed);
    let _ = crate::commands::validate::clear();
}

use evdev::{Device, KeyCode, EventSummary};
use std::time::Duration;
use std::path::Path;
use tokio::time::sleep;

pub async fn monitor_inputs() {
    println!("Starting evdev input monitor for Kiosk Exit (F12)");
    
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
                        if device.supported_keys().map_or(false, |keys| keys.contains(KeyCode::KEY_F12)) {
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
                                    EventSummary::Key(_, KeyCode::KEY_F12, 1) => {
                                        let _ = tx.blocking_send(true); // Key Down
                                    }
                                    EventSummary::Key(_, KeyCode::KEY_F12, 0) => {
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

        // Process events asynchronously with a timeout
        let mut is_pressed = false;
        loop {
            if is_pressed {
                match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
                    Ok(Some(false)) => {
                        // Key released before 5 seconds
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
                        // Timeout reached! F12 has been held for 5 seconds
                        println!("Panic Hotkey (F12) detected! Executing Kiosk Exit.");
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
                        break;
                    }
                }
            }
        }
        
        // If we reach here, we need to rescan devices (maybe one was unplugged and threw an error)
        sleep(Duration::from_secs(2)).await;
    }
}

pub fn execute_kiosk_exit() {
    println!("Panic Hotkey (F12) detected! Executing Kiosk Exit.");
    let targets = ["retroarch", "mame", "gcompris-qt", "scratch", "tuxpaint", "tuxmath", "tuxtype", "klettres"];
    
    let mut app_killed = false;
    for target in targets.iter() {
        if let Ok(status) = std::process::Command::new("pkill")
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

    if !app_killed {
        println!("No apps running, clearing screen.");
        let _ = crate::commands::validate::clear();
    }
}

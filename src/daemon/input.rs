use evdev::{Device, KeyCode, EventSummary};
use std::time::{Duration, Instant};
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
                if let Ok(device) = Device::open(path) {
                    // Only care about devices that have keys (like F12)
                    if device.supported_keys().map_or(false, |keys| keys.contains(KeyCode::KEY_F12)) {
                        devices.push(device);
                    }
                }
            }
        }

        if devices.is_empty() {
            // No suitable devices found yet (maybe permissions or none connected), sleep and retry
            sleep(Duration::from_secs(5)).await;
            continue;
        }

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
                match rx.recv().await {
                    Some(true) => {
                        is_pressed = true;
                    }
                    Some(false) => {}
                    None => {
                        break;
                    }
                }
            }
        }
        
        // If we reach here, we need to rescan devices (maybe one was unplugged and threw an error)
        sleep(Duration::from_secs(2)).await;
    }
}

fn execute_kiosk_exit() {
    let targets = ["retroarch", "mame", "gcompris-qt", "scratch", "tuxpaint", "tuxmath", "tuxtype", "klettres"];
    for target in targets.iter() {
        let _ = std::process::Command::new("killall")
            .arg("-9")
            .arg(target)
            .status();
    }
}

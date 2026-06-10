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
                let mut f12_pressed_time: Option<Instant> = None;
                
                loop {
                    // fetch_events blocks until events are available
                    match device.fetch_events() {
                        Ok(events) => {
                            for event in events {
                                match event.destructure() {
                                    EventSummary::Key(_, KeyCode::KEY_F12, 1) => {
                                        // Key down
                                        f12_pressed_time = Some(Instant::now());
                                    }
                                    EventSummary::Key(_, KeyCode::KEY_F12, 0) => {
                                        // Key up
                                        f12_pressed_time = None;
                                    }
                                    _ => {}
                                }
                            }
                            
                            // Check if F12 has been held for 5 seconds
                            if let Some(pressed_time) = f12_pressed_time {
                                if pressed_time.elapsed() >= Duration::from_secs(5) {
                                    let _ = tx.blocking_send(());
                                    // Reset to avoid spamming
                                    f12_pressed_time = None;
                                }
                            }
                        }
                        Err(_) => {
                            // Device disconnected or error
                            break;
                        }
                    }
                }
            });
        }

        // Wait for a trigger
        if rx.recv().await.is_some() {
            println!("Panic Hotkey (F12) detected! Executing Kiosk Exit.");
            execute_kiosk_exit();
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

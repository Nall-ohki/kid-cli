use tokio::net::{UnixListener, UnixStream};
use tokio::io::AsyncBufReadExt;
use anyhow::{Result, Context};
use std::fs;
use crate::daemon::engine;

pub fn get_socket_path() -> Result<std::path::PathBuf> {
    let home = home::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home.join(".kid_watch.sock"))
}

pub async fn listen(primary_pane_id: String) -> Result<()> {
    let socket_path = get_socket_path()?;
    if socket_path.exists() {
        fs::remove_file(&socket_path).context("Could not remove old socket file")?;
    }

    let listener = UnixListener::bind(&socket_path).context("Could not bind to Unix socket")?;
    println!("Listening on Unix socket: {:?}", socket_path);

    let engine = engine::Engine::new(primary_pane_id);
    engine.spawn_idle_loop();

    // Trigger session start welcome message!
    let engine_clone = engine.clone();
    tokio::spawn(async move {
        // Give a tiny moment for everything to settle
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        if let Err(e) = engine_clone.trigger_session_start().await {
            eprintln!("Failed to trigger session start: {}", e);
        }
    });

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let engine = engine.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, engine).await {
                        let err_str = e.to_string();
                        if !err_str.contains("no server running") && !err_str.contains("no current target") {
                            eprintln!("Error handling connection: {}", e);
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("Socket accept error: {}", e);
            }
        }
    }
}

async fn handle_connection(stream: UnixStream, engine: engine::Engine) -> Result<()> {
    // Need to read first, then write ACK
    // To do this properly with the same stream, we use into_split.
    let (reader, mut writer) = stream.into_split();
    let reader = tokio::io::BufReader::new(reader);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        match parse_event(&line) {
            Ok((event_type, data, cwd, pane_id)) => {
                engine.process(event_type, data, cwd, pane_id).await?;
            }
            Err(e) => {
                eprintln!("Failed to parse event: {}. Line: {}", e, line);
            }
        }
        use tokio::io::AsyncWriteExt;
        let _ = writer.write_all(b"OK\n").await;
    }

    Ok(())
}

fn parse_event(line: &str) -> Result<(&str, &str, &str, &str)> {
    let parts: Vec<&str> = line.splitn(4, ':').collect();
    if parts.len() < 2 {
        return Err(anyhow::anyhow!("Invalid event format"));
    }
    let event_type = parts[0];
    let data = parts[1];
    let cwd = if parts.len() > 2 { parts[2] } else { "/home/kid" };
    let pane_id = if parts.len() > 3 { parts[3] } else { "unknown" };
    Ok((event_type, data, cwd, pane_id))
}

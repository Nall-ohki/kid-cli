use tokio::net::{UnixListener, UnixStream};
use tokio::io::AsyncBufReadExt;
use anyhow::{Result, Context};
use std::fs;
use std::path::Path;
use crate::daemon::engine;

const SOCKET_PATH: &str = "/home/kid/.kid_watch.sock";

pub async fn listen(primary_pane_id: String) -> Result<()> {
    if Path::new(SOCKET_PATH).exists() {
        fs::remove_file(SOCKET_PATH).context("Could not remove old socket file")?;
    }

    let listener = UnixListener::bind(SOCKET_PATH).context("Could not bind to Unix socket")?;
    println!("Listening on Unix socket: {}", SOCKET_PATH);

    let engine = engine::Engine::new(primary_pane_id);

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

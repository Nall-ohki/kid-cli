use tokio::net::UnixStream;
use tokio::io::AsyncWriteExt;
use anyhow::Result;

pub async fn run(event_type: &str, data: &str, pane_id: Option<&str>) -> Result<()> {
    let socket_path = crate::daemon::socket::get_socket_path()?;
    let mut stream = None;
    let mut last_err = None;

    for _ in 0..5 {
        match UnixStream::connect(&socket_path).await {
            Ok(s) => {
                stream = Some(s);
                break;
            }
            Err(e) => {
                last_err = Some(e);
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
        }
    }

    let mut stream = stream.ok_or_else(|| {
        anyhow::anyhow!(
            "Could not connect to kid watch daemon socket after retries: {}. Path: {:?}",
            last_err.map(|e| e.to_string()).unwrap_or_default(),
            socket_path
        )
    })?;
    
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/home/kid"));
    let pane = pane_id.map(|s| s.to_string()).unwrap_or_else(|| {
        std::env::var("TMUX_PANE").unwrap_or_else(|_| "unknown".to_string())
    });

    let msg = format!("{}:{}:{}:{}\n", event_type, data, cwd.to_string_lossy(), pane);
    stream.write_all(msg.as_bytes()).await?;
    
    // Wait for ACK
    use tokio::io::AsyncReadExt;
    let mut buf = [0u8; 3];
    let _ = stream.read(&mut buf).await;
    
    Ok(())
}

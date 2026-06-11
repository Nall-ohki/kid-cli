use anyhow::Result;
use crate::daemon::pane;
use crate::config::messages::Config;
use rand::seq::SliceRandom;
use rand::thread_rng;

/// Primary message delivery — used by the personality system.
/// Sends a message with mood metadata to the companion pane.
/// The companion parses "MOOD:<mood>:<message>" to style the speech bubble.
pub async fn show_companion_message(text: &str, mood: &str) -> Result<()> {
    let formatted = format!("MOOD:{}:{}", mood, text);
    pane::show_message(&formatted).await
}

/// Legacy: trigger a greeting message based on cwd context.
/// Kept for backward compatibility during personality migration.
pub async fn trigger_greeting(config: &Config, cwd: &str, insight: Option<String>) -> Result<()> {
    // 0. If we have a contextual brain insight, it takes TOP priority
    if let Some(msg) = insight {
        return pane::show_message(&msg).await;
    }

    // 1. Check for specific context match (e.g. apps/gcompris)
    for (pattern, message) in &config.cd.context {
        if match_path(cwd, pattern) {
            return pane::show_message(message).await;
        }
    }

    // 2. Fallback to generic CD pool or HOME pool
    let pool = if cwd == "/home/kid" {
        &config.home.pool
    } else {
        &config.cd.pool
    };

    let msg = {
        let mut rng = thread_rng();
        pool.choose(&mut rng).cloned()
    };

    if let Some(m) = msg {
        pane::show_message(&m).await?;
    }

    Ok(())
}

/// Legacy: trigger a discovery message for launcher commands.
/// Kept for backward compatibility during personality migration.
pub async fn trigger_discovery(config: &Config, cmd: &str, insight: Option<String>) -> Result<()> {
    // Brain insight takes priority for discovery too
    if let Some(msg) = insight {
        return pane::show_message(&msg).await;
    }

    let msg = config.discovery.template
        .replace("{cmd}", cmd)
        .replace("{icon}", "⭐");
    
    pane::show_message(&msg).await
}

fn match_path(path: &str, pattern: &str) -> bool {
    let path = path.trim_end_matches('/');
    let pattern = pattern.trim_end_matches('/');
    
    if let Some(suffix) = pattern.strip_prefix("*") {
        return path.ends_with(suffix);
    }
    path == pattern
}

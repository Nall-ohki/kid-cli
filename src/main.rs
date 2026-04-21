use clap::{Parser, Subcommand};
use std::env;

mod commands;
mod terminal;
mod dispatch;
mod config;
mod daemon;

#[derive(Parser)]
#[command(name = "kid")]
#[command(about = "Unified control binary for the kid environment", long_about = None)]
#[command(disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show help sections in a beautiful interactive TUI
    Help {
        /// Section to show (basic, files, tools, fun, all)
        #[arg(default_value = "basic")]
        section: String,
    },
    /// Show a styled message with a sigil
    Msg {
        /// Message level (error, warn, info, ok)
        level: commands::msg::MsgLevel,
        /// Message text
        text: String,
    },
    /// Internal companion TUI display
    Companion,
    /// Install/bootstrap the kid environment
    Install {
        /// Only install safebin symlinks
        #[arg(long)]
        safebin: bool,
    },
    /// Start the companion daemon
    Watch {
        /// Run in background
        #[arg(long)]
        daemon: bool,
    },
    /// Fire an event to the daemon
    Event {
        /// Event type (pre, post)
        event_type: String,
        /// Event data (command or exit code)
        data: String,
        /// Originating pane ID
        pane_id: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let full_program_path = args[0].clone();
    let program_name = std::path::Path::new(&full_program_path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or(full_program_path.clone());

    // eprintln!("DEBUG: program_name='{}'", program_name);

    let is_busybox = match program_name.as_str() {
        "kid" | "target" | "kid-binary" | "companion" => false,
        "kid-run" => {
            let full_args = args[1..].join(" ");
            if full_args.contains("--companion") && full_args.contains("--bottom") {
                eprintln!("Cannot use both --companion and --bottom together");
                std::process::exit(1);
            }
            if args.len() == 1 {
                println!("Usage: kid-run [COMMAND]");
                std::process::exit(1);
            }
            false
        },
        _ => true,
    };

    if is_busybox {
        if let Err(e) = dispatch::handle_busybox(&program_name, &args[1..]).await {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        return Ok(());
    }

    // 2. Direct Subcommand Dispatch
    let cli = Cli::parse();

    let res = match cli.command {
        Some(Commands::Msg { level, text }) => {
            commands::msg::run(level, &text);
            Ok(())
        }
        Some(Commands::Companion) => {
            commands::companion::run().await
        }
        Some(Commands::Install { safebin }) => {
            commands::install::run(safebin)
        }
        Some(Commands::Watch { daemon }) => {
            if daemon {
                crate::daemon::start()
            } else {
                let pane_id = std::env::var("TMUX_PANE").unwrap_or_else(|_| "unknown".to_string());
                crate::daemon::run_server(pane_id).await
            }
        }
        Some(Commands::Event { event_type, data, pane_id }) => {
            commands::event::run(&event_type, &data, pane_id.as_deref()).await
        }
        Some(Commands::Help { section }) => {
            commands::help::run(&section)
        }
        None => {
            // If called as 'kid' without command, show interactive help
            commands::help::run("basic")
        }
    };

    if let Err(e) = res {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

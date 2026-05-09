use clap::{Parser, Subcommand};
use std::env;

mod commands;
mod terminal;
mod dispatch;
mod config;
mod daemon;
mod characters;


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
    /// Install/bootstrap the kid environment (internal container tool)
    Install {
        /// Install global system symlinks and configs
        #[arg(long)]
        system: bool,
        /// Install user-specific apps and directories
        #[arg(long)]
        user: bool,
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
    /// Browse and view character assets
    Characters,

    /// Administrative commands for system management (Requires Sudo)
    Admin {
        #[command(subcommand)]
        command: AdminCommands,
    },
}

#[derive(Subcommand)]
enum AdminCommands {
    /// Initialize the system globally
    Init {
        /// Skip the initial Docker image build
        #[arg(long)]
        skip_build: bool,
    },
    /// Deploy latest code and rebuild image
    Deploy {
        /// Skip the Docker image rebuild
        #[arg(long)]
        skip_build: bool,
    },
    /// Manage individual kid environments
    Kid {
        #[command(subcommand)]
        command: KidManagementCommands,
    },
}

#[derive(Subcommand)]
enum KidManagementCommands {
    /// Create a new kid user
    Create {
        /// Username of the child
        name: String,
    },
    /// Safely delete a kid user and their container
    Delete {
        /// Username of the child
        name: String,
    },
    /// Wipe settings but keep creations for a kid
    Reset {
        /// Username of the child
        name: String,
    },
    /// List all managed kid users
    List,
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

    // Context detection
    let is_inside = std::path::Path::new("/kid/bin/kid").exists();

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

    // Permission enforcement for Admin commands
    if let Some(Commands::Admin { .. }) = &cli.command {
        if is_inside {
            eprintln!("Error: Administrative commands are not allowed inside the Kid Environment.");
            std::process::exit(1);
        }
        #[cfg(unix)]
        {
            if !nix::unistd::getuid().is_root() {
                eprintln!("Error: This command requires root privileges (sudo).");
                std::process::exit(1);
            }
        }
    }

    let res = match cli.command {
        Some(Commands::Msg { level, text }) => {
            commands::msg::run(level, &text);
            Ok(())
        }
        Some(Commands::Companion) => {
            commands::companion::run().await
        }
        Some(Commands::Install { system, user }) => {
            commands::install::run(system, user)
        }
        Some(Commands::Admin { command }) => {
            match command {
                AdminCommands::Init { skip_build } => commands::admin::system_init(skip_build),
                AdminCommands::Deploy { skip_build } => commands::admin::deploy(skip_build),
                AdminCommands::Kid { command } => match command {
                    KidManagementCommands::Create { name } => commands::admin::create_kid(&name),
                    KidManagementCommands::Delete { name } => commands::admin::delete_kid(&name),
                    KidManagementCommands::Reset { name } => commands::admin::reset_kid(&name),
                    KidManagementCommands::List => commands::admin::list_kids(),
                }
            }
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
        Some(Commands::Characters) => {
            commands::characters::run().await
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

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
    /// Manually trigger the global panic/kiosk exit logic
    Panic,
    /// Explicitly launch an application via the kid unified launcher
    Launch {
        /// Application name
        name: String,
        /// Any remaining arguments
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Run companion personality scenario simulations (Scenario Runner TUI)
    Scenarios,

    /// Administrative commands for system management (Requires Sudo)
    Admin {
        #[command(subcommand)]
        command: AdminCommands,
    },
}

#[derive(Subcommand)]
enum AdminCommands {
    /// Initialize the system globally
    Init,
    /// Deploy latest code (updates /opt/kid-cli)
    Deploy {
        /// Optional path to a pre-built Docker image tarball to load
        #[arg(long)]
        image: Option<String>,
        /// Internal flag to prevent infinite recursion during self-update
        #[arg(long, hide = true)]
        no_rebuild: bool,
    },
    /// Explicitly build the Docker environment image
    Build,
    /// Show detailed system and environment status
    Status,
    /// Abort running applications or terminate a kid session
    Abort {
        /// Optional kid username to terminate entirely
        name: Option<String>,
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
        #[cfg(unix)]
        {
            if !nix::unistd::getuid().is_root() {
                eprintln!("Error: Administrative commands require root privileges (sudo).");
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
                AdminCommands::Init => commands::admin::system_init(),
                AdminCommands::Deploy { image, no_rebuild } => commands::admin::deploy(image, no_rebuild),
                AdminCommands::Build => commands::admin::build_docker(),
                AdminCommands::Status => commands::admin::system_status(),
                AdminCommands::Abort { name } => commands::admin::abort_session(name.as_deref()),
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
        Some(Commands::Scenarios) => {
            commands::scenarios::run().await
        }
        Some(Commands::Panic) => {
            crate::daemon::input::execute_kiosk_exit();
            Ok(())
        }
        Some(Commands::Launch { name, args }) => {
            let config_dir = config::get_config_dir()?;
            let commands_config = config::commands::Config::load(config_dir.join("commands.toml"))?;
            
            if let Some(launcher) = commands_config.launchers.get(&name) {
                if !launcher.enabled {
                    return Err(anyhow::anyhow!("Command '{}' is currently disabled.", name));
                }
                commands::launch::run(&name, launcher, &args).await
            } else if let Some(game) = commands_config.games.get(&name) {
                if !game.enabled {
                    return Err(anyhow::anyhow!("Game '{}' is currently disabled.", name));
                }
                if let Some(system) = commands_config.systems.get(&game.system) {
                    let rom_path = format!("{}/{}", system.rom_dir, game.rom);
                    let home_str = home::home_dir()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "/home/kid".to_string());
                    let binary = system.template
                        .replace("{rom_dir}", &system.rom_dir)
                        .replace("{rom_path}", &rom_path)
                        .replace("{rom}", &game.rom)
                        .replace("{home}", &home_str)
                        .replace("/home/kid", &home_str);

                    let mut launcher = config::commands::LauncherConfig::default();
                    launcher.binary = Some(binary);
                    launcher.gui = true;
                    launcher.pane = "none".to_string();
                    
                    commands::launch::run(&name, &launcher, &args).await
                } else {
                    Err(anyhow::anyhow!("System '{}' not found for game '{}'", game.system, name))
                }
            } else {
                Err(anyhow::anyhow!("Application '{}' is not registered in the launchers or games config.", name))
            }
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

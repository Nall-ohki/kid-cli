use crate::config;
use crate::terminal::{styled_message, MessageLevel};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::os::unix::fs::symlink;

pub fn run(system: bool, user: bool) -> Result<()> {
    if !is_inside_infrastructure() {
        styled_message(MessageLevel::Info, "This command is for internal container environment setup.");
        styled_message(MessageLevel::Info, "To initialize the system globally, use: sudo kid admin init");
        styled_message(MessageLevel::Info, "To create a new kid environment, use: sudo kid admin kid create <name>");
        return Ok(());
    }

    let config_dir = config::get_config_dir().context("Could not get config directory")?;
    
    // 1. Bootstrap TOMLs if needed (System concern)
    if system {
        bootstrap_tomls(&config_dir)?;
        install_symlinks(&config_dir)?;
    }

    // 2. User-specific structure (User concern)
    if user {
        styled_message(MessageLevel::Info, "Creating user-specific directory structure...");
        create_structure()?;
    }

    styled_message(MessageLevel::Ok, "Installation step complete!");
    Ok(())
}

fn is_inside_infrastructure() -> bool {
    // Check for the canonical binary path that only exists in the Docker environment
    Path::new("/kid/bin/kid").exists()
}

fn create_structure() -> Result<()> {
    let home = home::home_dir().context("Could not get home directory")?;
    
    let dirs = [
        "apps",
        "tools",
        "creations/pictures",
        "creations/programs",
        "creations/games",
    ];

    for d in dirs {
        let path = home.join(d);
        if !path.exists() {
            fs::create_dir_all(&path)?;
            styled_message(MessageLevel::Ok, &format!("Created ~/{}", d));
        }
    }

    // Link shell and tmux configs
    let config_zsh = home.join(".config/zsh");
    let zshrc = home.join(".zshrc");
    let target_zshrc = config_zsh.join("zshrc.zsh");
    if target_zshrc.exists() {
        if zshrc.exists() || zshrc.is_symlink() {
            fs::remove_file(&zshrc)?;
        }
        symlink(&target_zshrc, &zshrc)?;
        styled_message(MessageLevel::Ok, "Linked ~/.zshrc");
    }

    let tmux_conf = home.join(".tmux.conf");
    let target_tmux = config_zsh.join("tmux.conf");
    if target_tmux.exists() {
        if tmux_conf.exists() || tmux_conf.is_symlink() {
            fs::remove_file(&tmux_conf)?;
        }
        symlink(&target_tmux, &tmux_conf)?;
        styled_message(MessageLevel::Ok, "Linked ~/.tmux.conf");
    }
    install_apps(&home)?;

    Ok(())
}

fn install_apps(home: &std::path::Path) -> Result<()> {
    let apps = [
        "tuxpaint",
        "gcompris",
        "scratch",
        "tuxmath",
        "tuxtype",
        "klettres",
    ];

    for name in apps {
        let app_dir = home.join("apps").join(name);
        fs::create_dir_all(&app_dir)?;
        
        let wrapper_path = app_dir.join(name);
        
        let content = format!(
            "#!/bin/zsh\n\
             # Restricted App Wrapper\n\n\
             # Delegate to the unified kid-cli launcher\n\
             exec /kid/bin/kid launch {}\n",
            name
        );
        
        fs::write(&wrapper_path, content)?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&wrapper_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&wrapper_path, perms)?;
        }
        
        styled_message(MessageLevel::Ok, &format!("Installed app wrapper: ~/apps/{}/{}", name, name));
    }
    
    Ok(())
}

fn bootstrap_tomls(config_dir: &Path) -> Result<()> {
    let files = [
        ("commands.toml", config::commands::get_default_toml()),
    ];

    for (name, content) in files {
        let path = config_dir.join(name);
        let should_write = if !path.exists() {
            true
        } else {
            // If it exists but is empty, bootstrap it
            fs::metadata(&path).map(|m| m.len() == 0).unwrap_or(false)
        };

        if should_write {
            fs::write(&path, content.trim_start()).with_context(|| format!("Could not write {}", name))?;
            styled_message(MessageLevel::Ok, &format!("Created/Reset {}", name));
        }
    }
    Ok(())
}

fn install_symlinks(config_dir: &Path) -> Result<()> {
    styled_message(MessageLevel::Info, "Installing busybox symlinks...");

    let commands_config = config::commands::Config::load(config_dir.join("commands.toml"))?;
    let kid_bin = "/kid/bin/kid"; // Canonical path in Docker

    // A. Validators -> /kid/wrap/bin
    let validators = ["cd", "home", "exit", "clear"];
    for v in validators {
        create_symlink(kid_bin, &format!("/kid/wrap/bin/{}", v))?;
    }

    // B, C. Launchers and Passthroughs -> /kid/wrap/bin
    for name in commands_config.launchers.keys() {
        create_symlink(kid_bin, &format!("/kid/wrap/bin/{}", name))?;
    }
    for name in commands_config.passthroughs.keys() {
        create_symlink(kid_bin, &format!("/kid/wrap/bin/{}", name))?;
    }

    // D. Standard Proxies (Ensure these always exist for tests and stability)
    let emergency_proxies = [
        "ls", "cat", "less", "file", "touch", "echo", "mkdir", "rmdir", 
        "pwd", "wc", "head", "tail", "grep", "cal", "rm", "mv", "cp", 
        "help", "clear", "reset", "whoami", "date", "groups", "id", "uv"
    ];
    for p in emergency_proxies {
        create_symlink(kid_bin, &format!("/kid/allow/bin/{}", p))?;
    }

    // E. Blocks -> /kid/restricted/bin AND their real locations
    for name in &commands_config.blocks.commands {
        create_symlink(kid_bin, &format!("/kid/restricted/bin/{}", name))?;
        
        // Also capture at common system locations if we are root
        let system_paths = [
            format!("/bin/{}", name),
            format!("/usr/bin/{}", name),
            format!("/usr/sbin/{}", name),
        ];
        
        for p in system_paths {
            let path = std::path::Path::new(&p);
            if path.exists() && !path.is_symlink() {
                // Try to capture it. This might fail if we don't have perms,
                // but in Docker build we usually are root.
                let _ = create_symlink(kid_bin, &p);
            }
        }
    }

    // E. Legacy Symlinks -> /kid/bin
    let legacy = ["kid-run", "kid-error", "kid-warn", "kid-ls", "help", "kid-msg-pipe"];
    for l in legacy {
        create_symlink(kid_bin, &format!("/kid/bin/{}", l))?;
    }

    styled_message(MessageLevel::Ok, "Symlinks installed!");
    Ok(())
}

fn create_symlink(src: &str, dst: &str) -> Result<()> {
    let dst_path = Path::new(dst);
    if dst_path.exists() || dst_path.is_symlink() {
        fs::remove_file(dst_path).context(format!("Could not remove existing link {}", dst))?;
    }
    
    // Ensure parent dir exists
    if let Some(parent) = dst_path.parent() {
        fs::create_dir_all(parent)?;
    }

    symlink(src, dst_path).context(format!("Could not create symlink {} -> {}", dst, src))?;
    Ok(())
}

use crate::config;
use crate::terminal::{styled_message, MessageLevel};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::os::unix::fs::symlink;

pub fn run(system: bool, user: bool) -> Result<()> {
    let config_dir = config::get_config_dir().unwrap_or_else(|_| {
        let global = Path::new("/opt/kid-cli/config");
        if global.exists() {
            global.to_path_buf()
        } else {
            Path::new("/kid/config").to_path_buf()
        }
    });
    
    // 1. Bootstrap TOMLs if needed (System concern)
    if system {
        bootstrap_tomls(&config_dir)?;
        install_symlinks(&config_dir)?;
    }

    // 2. User-specific structure (User concern)
    if user {
        styled_message(MessageLevel::Info, "Creating user-specific directory structure...");
        bootstrap_tomls(&config_dir)?;
        let commands_config = config::commands::Config::load(config_dir.join("commands.toml"))?;
        create_structure(&commands_config)?;
    }

    styled_message(MessageLevel::Ok, "Installation step complete!");
    Ok(())
}

fn create_structure(commands_config: &config::commands::Config) -> Result<()> {
    let home = home::home_dir().context("Could not get home directory")?;
    
    let old_tools = home.join("tools");
    let new_tools = home.join(".tools");
    if old_tools.exists() && !new_tools.exists() {
        if let Err(e) = fs::rename(&old_tools, &new_tools) {
            styled_message(MessageLevel::Error, &format!("Failed to migrate tools to .tools: {}", e));
        } else {
            styled_message(MessageLevel::Ok, "Migrated tools to .tools");
        }
    }
    
    let dirs = [
        "apps",
        ".tools",
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
    let global_config = Path::new("/opt/kid-cli/config");
    let config_zsh = home.join(".config/zsh");
    
    let zshrc = home.join(".zshrc");
    let target_zshrc = if global_config.join("zshrc.zsh").exists() {
        global_config.join("zshrc.zsh")
    } else {
        config_zsh.join("zshrc.zsh")
    };
    if target_zshrc.exists() {
        if zshrc.exists() || zshrc.is_symlink() {
            fs::remove_file(&zshrc)?;
        }
        symlink(&target_zshrc, &zshrc)?;
        styled_message(MessageLevel::Ok, "Linked ~/.zshrc");
    }

    let tmux_conf = home.join(".tmux.conf");
    let target_tmux = if global_config.join("tmux.conf").exists() {
        global_config.join("tmux.conf")
    } else {
        config_zsh.join("tmux.conf")
    };
    if target_tmux.exists() {
        if tmux_conf.exists() || tmux_conf.is_symlink() {
            fs::remove_file(&tmux_conf)?;
        }
        symlink(&target_tmux, &tmux_conf)?;
        styled_message(MessageLevel::Ok, "Linked ~/.tmux.conf");
    }

    let target_foot = if global_config.join("foot.ini").exists() {
        global_config.join("foot.ini")
    } else {
        config_zsh.join("foot.ini")
    };
    if target_foot.exists() {
        let config_foot = home.join(".config/foot");
        if !config_foot.exists() {
            fs::create_dir_all(&config_foot)?;
        }
        let foot_ini = config_foot.join("foot.ini");
        if foot_ini.exists() || foot_ini.is_symlink() {
            fs::remove_file(&foot_ini)?;
        }
        symlink(&target_foot, &foot_ini)?;
        styled_message(MessageLevel::Ok, "Linked ~/.config/foot/foot.ini");
    }

    install_apps(&home, commands_config)?;
    configure_mame(&home)?;
    configure_retroarch(&home)?;

    Ok(())
}

fn install_apps(home: &std::path::Path, commands_config: &config::commands::Config) -> Result<()> {
    let mut apps = Vec::new();
    
    for (name, launcher) in &commands_config.launchers {
        if launcher.enabled && launcher.gui {
            apps.push(name.clone());
        }
    }
    for (name, game) in &commands_config.games {
        if game.enabled {
            apps.push(name.clone());
        }
    }

    for name in &apps {
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

fn configure_mame(home: &Path) -> Result<()> {
    let mame_dir = home.join(".mame");
    let cfg_dir = mame_dir.join("cfg");
    fs::create_dir_all(&cfg_dir)?;

    let ini_path = mame_dir.join("mame.ini");
    let ini_content = "uimodekey NONE\n";
    fs::write(&ini_path, ini_content)?;

    let cfg_path = cfg_dir.join("default.cfg");
    let cfg_content = r#"<?xml version="1.0"?>
<mameconfig version="10">
    <system name="default">
        <input>
            <port type="UI_CONFIGURE">
                <newseq type="standard">NONE</newseq>
            </port>
            <port type="UI_CANCEL">
                <newseq type="standard">NONE</newseq>
            </port>
            <port type="UI_TOGGLE_UI">
                <newseq type="standard">NONE</newseq>
            </port>
        </input>
    </system>
</mameconfig>"#;
    fs::write(&cfg_path, cfg_content)?;

    styled_message(MessageLevel::Ok, "Installed MAME kiosk configuration");
    Ok(())
}

fn configure_retroarch(home: &Path) -> Result<()> {
    let retroarch_dir = home.join(".config").join("retroarch");
    fs::create_dir_all(&retroarch_dir)?;

    let cfg_path = retroarch_dir.join("retroarch.cfg");
    let cfg_content = "menu_driver = \"null\"\n\
                       input_menu_toggle = \"nul\"\n\
                       input_driver = \"sdl2\"\n";
    fs::write(&cfg_path, cfg_content)?;

    let mouse_cfg_path = retroarch_dir.join("mouse.cfg");
    // Libretro device 2 = Mouse. Device 258 = Joypad w/ Analog. Port 1 is index 0.
    let mouse_cfg_content = "input_player1_mouse_index = \"0\"\ninput_libretro_device_p1 = \"2\"\n";
    fs::write(&mouse_cfg_path, mouse_cfg_content)?;

    styled_message(MessageLevel::Ok, "Installed RetroArch kiosk configuration");
    Ok(())
}

fn bootstrap_tomls(config_dir: &Path) -> Result<()> {
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }

    let files = [
        ("commands.toml", config::commands::get_default_toml()),
        ("personality.toml", config::personality::get_default_toml()),
        ("messages.toml", config::messages::get_default_toml()),
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

    // B, C. Launchers, Games, and Passthroughs -> /kid/wrap/bin
    for (name, launcher) in &commands_config.launchers {
        if launcher.enabled {
            create_symlink(kid_bin, &format!("/kid/wrap/bin/{}", name))?;
        }
    }
    for (name, game) in &commands_config.games {
        if game.enabled {
            create_symlink(kid_bin, &format!("/kid/wrap/bin/{}", name))?;
        }
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

    // E. Blocks -> /kid/restricted/bin
    for name in &commands_config.blocks.commands {
        create_symlink(kid_bin, &format!("/kid/restricted/bin/{}", name))?;
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

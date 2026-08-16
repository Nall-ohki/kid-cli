use crate::terminal::{styled_message, MessageLevel};
use anyhow::Result;
// use which::which;
use std::process::Command;
use std::os::unix::process::CommandExt;
use std::fs;
use std::path::Path;

const GLOBAL_PATH: &str = "/opt/kid-cli";
const BINARY_PATH: &str = "/usr/local/bin/kid";
const SYSTEM_GROUP: &str = "kid-users";

pub fn system_init() -> Result<()> {
    styled_message(MessageLevel::Info, "Initializing Kid-CLI System globally...");

    // 1. Create global directory
    if !Path::new(GLOBAL_PATH).exists() {
        fs::create_dir_all(GLOBAL_PATH)?;
        styled_message(MessageLevel::Ok, &format!("Created {}", GLOBAL_PATH));
    }

    // 2. Sync Repository
    let repo_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
    let global_has_repo = Path::new(GLOBAL_PATH).join("Cargo.toml").exists();
    if !global_has_repo {
        if repo_root.join("Cargo.toml").exists() && repo_root != Path::new(GLOBAL_PATH) {
            styled_message(MessageLevel::Info, "Ensuring repository at /opt/kid-cli...");
            if Path::new(GLOBAL_PATH).exists() && !Path::new(GLOBAL_PATH).join(".git").exists() {
                fs::remove_dir_all(GLOBAL_PATH)?;
            }

            if !Path::new(GLOBAL_PATH).exists() {
                Command::new("git")
                    .args(&["clone", "https://github.com/Nall-ohki/kid-cli.git", GLOBAL_PATH])
                    .status()?;
            }
        } else if !Path::new(GLOBAL_PATH).exists() {
            Command::new("git")
                .args(&["clone", "https://github.com/Nall-ohki/kid-cli.git", GLOBAL_PATH])
                .status()?;
        }
    }

    // 3. Create system and hardware groups
    if cfg!(target_os = "linux") {
        for group in &[SYSTEM_GROUP, "render", "input", "video", "tty", "docker"] {
            let _ = Command::new("groupadd").arg("-f").arg(group).status();
        }
        styled_message(MessageLevel::Ok, &format!("System group '{}' ensured.", SYSTEM_GROUP));
        if Path::new("/var/run/docker.sock").exists() {
            let _ = Command::new("chmod").args(&["666", "/var/run/docker.sock"]).status();
        }
    }

    // 4. Install and Symlink binary
    let install_bin_dir = Path::new(GLOBAL_PATH).join("bin");
    let install_bin_path = install_bin_dir.join("kid");
    
    if !install_bin_dir.exists() {
        fs::create_dir_all(&install_bin_dir)?;
    }

    let current_exe = std::env::current_exe()?;
    if current_exe != install_bin_path {
        if install_bin_path.exists() {
            let _ = fs::remove_file(&install_bin_path);
        }
        fs::copy(&current_exe, &install_bin_path)?;
    }
    styled_message(MessageLevel::Ok, &format!("Installed binary to {}", install_bin_path.display()));

    if Path::new(BINARY_PATH).exists() || Path::new(BINARY_PATH).is_symlink() {
        let _ = fs::remove_file(BINARY_PATH);
    }
    std::os::unix::fs::symlink(&install_bin_path, BINARY_PATH)?;
    styled_message(MessageLevel::Ok, &format!("Symlinked binary to {}", BINARY_PATH));

    // 5. Setup /kid infrastructure and system symlinks
    fs::create_dir_all("/kid/bin")?;
    fs::create_dir_all("/kid/wrap/bin")?;
    fs::create_dir_all("/kid/allow/bin")?;
    fs::create_dir_all("/kid/restricted/bin")?;
    fs::create_dir_all("/kid/emulation/disks")?;

    let kid_bin_link = Path::new("/kid/bin/kid");
    if kid_bin_link.exists() || kid_bin_link.is_symlink() {
        let _ = fs::remove_file(kid_bin_link);
    }
    std::os::unix::fs::symlink(&install_bin_path, kid_bin_link)?;

    // Link ROMs if assets/roms exists
    let roms_src = Path::new(GLOBAL_PATH).join("assets/roms");
    let roms_dst = Path::new("/kid/emulation/disks");
    if roms_src.exists() {
        let apple2gs_src = roms_src.join("apple2gs");
        let apple2gs_dst = roms_dst.join("apple2gs");
        if apple2gs_src.exists() && !apple2gs_dst.exists() {
            let _ = std::os::unix::fs::symlink(&apple2gs_src, &apple2gs_dst);
        }
        let snes_src = roms_src.join("snes");
        let snes_dst = roms_dst.join("snes");
        if snes_src.exists() && !snes_dst.exists() {
            let _ = std::os::unix::fs::symlink(&snes_src, &snes_dst);
        }
    }

    // Link global config /kid/config -> /opt/kid-cli/config
    let config_src = Path::new(GLOBAL_PATH).join("config");
    let config_dst = Path::new("/kid/config");
    if config_src.exists() {
        if config_dst.exists() || config_dst.is_symlink() {
            let _ = fs::remove_file(config_dst);
        }
        let _ = std::os::unix::fs::symlink(&config_src, config_dst);
    }

    // Run system installation to create /kid/wrap/bin, /kid/allow/bin, /kid/restricted/bin
    let _ = crate::commands::install::run(true, false);

    // 6. Install Global Launcher in /etc/zsh/zprofile
    install_global_launcher()?;

    styled_message(MessageLevel::Ok, "System initialization complete!");
    Ok(())
}

pub fn deploy(image_path: Option<String>, no_rebuild: bool) -> Result<()> {
    styled_message(MessageLevel::Info, "Deploying updates to /opt/kid-cli...");

    let repo_path = Path::new(GLOBAL_PATH);
    if !repo_path.exists() {
        return Err(anyhow::anyhow!("System not initialized. Run 'kid admin init' first."));
    }

    // 1. Update Code
    if repo_path.join(".git").exists() {
        styled_message(MessageLevel::Info, "Pulling latest code from Git...");
        let status = Command::new("git")
            .arg("-C").arg(GLOBAL_PATH)
            .arg("pull")
            .status()?;
        
        if !status.success() {
            return Err(anyhow::anyhow!("Git pull failed"));
        }
    }

    // 2. Smart Rebuild Detection
    let current_hash = env!("GIT_HASH");
    let head_output = Command::new("git")
        .arg("-C").arg(GLOBAL_PATH)
        .args(&["rev-parse", "HEAD"])
        .output()?;
    let head_hash = String::from_utf8_lossy(&head_output.stdout).trim().to_string();

    if head_hash != current_hash && head_hash != "unknown" && !no_rebuild {
        styled_message(MessageLevel::Info, &format!("Version mismatch (Local: {} vs Repo: {}). Rebuilding...", &current_hash[..7], &head_hash[..7]));
        
        let status = Command::new("cargo")
            .current_dir(GLOBAL_PATH)
            .env("RUSTUP_HOME", "/usr/local/rustup")
            .env("CARGO_HOME", "/usr/local/cargo")
            .env("PATH", "/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin")
            .arg("build")
            .arg("--release")
            .status()?;
        
        if status.success() {
            styled_message(MessageLevel::Ok, "Binary rebuilt. Delegating to new version...");
            
            let src_binary = format!("{}/target/release/kid", GLOBAL_PATH);
            let dest_binary = format!("{}/bin/kid", GLOBAL_PATH);

            // Unlink the old binary first to avoid "Text file busy"
            if Path::new(&dest_binary).exists() {
                let _ = fs::remove_file(&dest_binary);
            }

            // Sync the new binary to the global path
            fs::copy(&src_binary, &dest_binary)?;
            
            // Re-exec ourselves with --no-rebuild to continue the deploy
            // We use the absolute path to the global binary to be safe
            let mut args: Vec<String> = std::env::args().collect();
            args.push("--no-rebuild".to_string());
            
            let err = Command::new(&dest_binary)
                .args(&args[1..])
                .exec();
            
            return Err(anyhow::Error::from(err).context("Failed to delegate to new binary"));
        } else {
            styled_message(MessageLevel::Warn, "Binary rebuild failed. Continuing with current version.");
        }
    } else if head_hash == current_hash {
        styled_message(MessageLevel::Info, "Binary is already up-to-date with repository HEAD.");
    }

    // 3. Optional: Load Image
    if let Some(path) = image_path {
        styled_message(MessageLevel::Info, &format!("Loading Docker image from {}...", path));
        let status = Command::new("docker")
            .args(&["load", "-i", &path])
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to load Docker image"));
        }
        styled_message(MessageLevel::Ok, "Docker image loaded successfully.");
    }

    // 4. Ensure global launcher in /etc/zsh/zprofile is updated
    install_global_launcher()?;

    styled_message(MessageLevel::Ok, "Deployment successful!");
    Ok(())
}

pub fn build_docker() -> Result<()> {
    styled_message(MessageLevel::Info, "Building Docker environment image...");
    let status = Command::new("docker")
        .args(&["compose", "-f", &format!("{}/docker-compose.yml", GLOBAL_PATH), "build"])
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Docker build failed"));
    }

    styled_message(MessageLevel::Ok, "Docker build complete!");
    Ok(())
}

pub fn create_kid(name: &str) -> Result<()> {
    styled_message(MessageLevel::Info, &format!("Creating kid user: {}", name));

    if cfg!(target_os = "linux") {
        // 1. Create Linux user (ignore if already exists)
        let _ = Command::new("useradd")
            .args(&["-m", "-s", "/bin/zsh", name])
            .status();
        
        // Ensure user is in groups
        for group in &[SYSTEM_GROUP, "video", "render", "input", "tty", "docker"] {
            let _ = Command::new("groupadd").arg("-f").arg(group).status();
            let _ = Command::new("usermod").args(&["-aG", group, name]).status();
        }
        
        if Path::new("/var/run/docker.sock").exists() {
            let _ = Command::new("chmod").args(&["666", "/var/run/docker.sock"]).status();
        }
    }

    // 3. Create creations directory and .zshrc (to silence new user prompt)
    let home = format!("/home/{}", name);
    let creations = format!("{}/creations", home);
    let zshrc = format!("{}/.zshrc", home);
    
    fs::create_dir_all(&creations)?;
    fs::write(&zshrc, "# Kid Environment Shell Config\n")?;
    
    if cfg!(target_os = "linux") {
        // Set ownership (user:kid-users)
        let status = Command::new("chown")
            .arg("-R")
            .arg(&format!("{}:{}", name, SYSTEM_GROUP))
            .arg(&home)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to set permissions on creations directory"));
        }

        // 4. Run 'kid install --user' as the new user to populate apps/tools
        styled_message(MessageLevel::Info, "Populating kid environment (apps/tools)...");
        let status = Command::new("sudo")
            .args(&["-u", name, "/usr/local/bin/kid", "install", "--user"])
            .status()?;
        if !status.success() {
            styled_message(MessageLevel::Warn, "Environment population partially failed.");
        }
    }

    styled_message(MessageLevel::Ok, &format!("User '{}' created and provisioned.", name));
    Ok(())
}

pub fn delete_kid(name: &str) -> Result<()> {
    styled_message(MessageLevel::Info, &format!("Deleting kid user: {}", name));

    // 1. Stop and remove container/volumes
    let project_name = format!("kid-{}", name);
    let status = Command::new("docker")
        .args(&["compose", "-f", &format!("{}/docker-compose.yml", GLOBAL_PATH), "-p", &project_name, "down", "-v"])
        .status()?;
    if !status.success() {
        styled_message(MessageLevel::Warn, "Could not fully remove Docker resources (maybe they don't exist yet).");
    }

    if cfg!(target_os = "linux") {
        // 2. Remove Linux user
        let status = Command::new("userdel")
            .args(&["-r", name])
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to delete Linux user '{}'", name));
        }
    }

    styled_message(MessageLevel::Ok, &format!("User '{}' and all associated data deleted.", name));
    Ok(())
}

pub fn reset_kid(name: &str) -> Result<()> {
    styled_message(MessageLevel::Info, &format!("Performing robust reset for kid: {}", name));

    let backup_path = format!("/tmp/kid_backup_{}.tar.gz", name);

    // 1. Safety Backup
    styled_message(MessageLevel::Info, "Creating safety backup of user home...");
    let status = Command::new("tar")
        .args(&["-czf", &backup_path, "-C", "/home", name])
        .status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Backup failed. Aborting reset for safety."));
    }

    // 2. Re-provision
    delete_kid(name)?;
    create_kid(name)?;

    // 3. Restore creations/
    styled_message(MessageLevel::Info, "Restoring creations...");
    let temp_extract = format!("/tmp/kid_restore_{}", name);
    fs::create_dir_all(&temp_extract)?;
    
    Command::new("tar").args(&["-xzf", &backup_path, "-C", &temp_extract]).status()?;
    
    let src_creations = format!("{}/{}/creations/.", temp_extract, name);
    let dst_creations = format!("/home/{}/creations/", name);
    
    Command::new("cp").args(&["-a", &src_creations, &dst_creations]).status()?;
    
    // Fix ownership again
    Command::new("chown").args(&["-R", &format!("{}:{}", name, SYSTEM_GROUP), &dst_creations]).status()?;

    // Cleanup temp
    let _ = fs::remove_dir_all(&temp_extract);
    
    styled_message(MessageLevel::Ok, &format!("User '{}' has been reset. Creations restored from {}", name, backup_path));
    Ok(())
}

pub fn list_kids() -> Result<()> {
    styled_message(MessageLevel::Info, "Managed Kid Users:");

    let output = Command::new("getent").arg("group").arg(SYSTEM_GROUP).output()?;
    let line = String::from_utf8_lossy(&output.stdout);
    
    // Format: group:x:gid:user1,user2
    let members = line.split(':').last().unwrap_or("").trim();
    
    if members.is_empty() {
        println!("  (No kid users found)");
        return Ok(());
    }

    for name in members.split(',') {
        let project_name = format!("kid-{}", name);
        let status_output = Command::new("docker")
            .args(&["ps", "--filter", &format!("name={}", project_name), "--format", "{{.Status}}"])
            .output()?;
        let status = String::from_utf8_lossy(&status_output.stdout).trim().to_string();
        
        let status_str = if status.is_empty() { "Inactive" } else { &status };
        println!("  - {:<15} [{}]", name, status_str);
    }

    Ok(())
}

pub fn system_status() -> Result<()> {
    use crate::terminal::styled_message;
    use crate::terminal::MessageLevel;

    println!("\n====================================================");
    println!("          KID-CLI SYSTEM STATUS REPORT             ");
    println!("====================================================\n");

    // 1. Core Installation
    println!("--- Core System ---");
    println!("    └─ Version:   {}", env!("GIT_HASH"));
    println!("    └─ Built:     {}", env!("BUILD_TIME"));

    let global_exists = Path::new(GLOBAL_PATH).exists();
    if global_exists {
        styled_message(MessageLevel::Ok, &format!("Global Path: {} [Exists]", GLOBAL_PATH));
    } else {
        styled_message(MessageLevel::Error, &format!("Global Path: {} [Missing]", GLOBAL_PATH));
    }

    let bin_exists = Path::new(BINARY_PATH).exists();
    if bin_exists {
        styled_message(MessageLevel::Ok, &format!("Binary Link: {} [OK]", BINARY_PATH));
    } else {
        styled_message(MessageLevel::Warn, &format!("Binary Link: {} [Missing from /usr/local/bin]", BINARY_PATH));
    }

    // 2. Dependencies
    println!("\n--- Dependencies ---");
    check_dep("git", &["--version"]);
    check_dep("docker", &["--version"]);
    check_dep("rustc", &["--version"]);

    if cfg!(target_os = "linux") {
        let group_check = Command::new("getent").arg("group").arg(SYSTEM_GROUP).output()?;
        if group_check.status.success() {
            styled_message(MessageLevel::Ok, &format!("System Group: '{}' [Exists]", SYSTEM_GROUP));
        } else {
            styled_message(MessageLevel::Error, &format!("System Group: '{}' [Missing]", SYSTEM_GROUP));
        }
    }

    // 3. Docker Image Status
    println!("\n--- Docker Assets ---");
    let img_check = Command::new("docker").args(&["images", "-q", "kid-env"]).output()?;
    if !img_check.stdout.is_empty() {
        styled_message(MessageLevel::Ok, "Base Image: 'kid-env' [Ready]");
    } else {
        styled_message(MessageLevel::Warn, "Base Image: 'kid-env' [Not Built - will build JIT]");
    }

    // 4. Managed Kids
    println!("\n--- Managed Kids ---");
    list_kids()?;

    println!("\n====================================================");
    Ok(())
}

fn check_dep(cmd: &str, args: &[&str]) {
    let envs = [
        ("RUSTUP_HOME", "/usr/local/rustup"),
        ("CARGO_HOME", "/usr/local/cargo"),
    ];

    // 1. Try standard PATH
    let mut output = Command::new(cmd)
        .args(args)
        .envs(envs.iter().cloned())
        .output();

    // 2. If failed, try common absolute paths (e.g. for /usr/local/bin under sudo)
    if output.is_err() || !output.as_ref().unwrap().status.success() {
        let common_paths = ["/usr/local/bin", "/usr/bin", "/bin"];
        for path in common_paths {
            let full_cmd = format!("{}/{}", path, cmd);
            if let Ok(out) = Command::new(&full_cmd)
                .args(args)
                .envs(envs.iter().cloned())
                .output() {
                if out.status.success() {
                    output = Ok(out);
                    break;
                }
            }
        }
    }

    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).split('\n').next().unwrap_or("").to_string();
            styled_message(MessageLevel::Ok, &format!("{}: {}", cmd, version));
        }
        _ => {
            styled_message(MessageLevel::Error, &format!("{}: Not found or error", cmd));
        }
    }
}

fn install_global_launcher() -> Result<()> {
    let profile_path = "/etc/zsh/zprofile";
    let shim = format!(
        "\n# --- Kid-CLI Global Launcher ---\n\
        if [[ -t 0 && -z \"$SKIP_KID\" ]]; then\n  \
          if id -nG | grep -q \"{0}\"; then\n    \
            export KID_CREATIONS_DIR=\"$HOME/creations\"\n    \
            if [[ -z \"$SSH_CONNECTION\" && -z \"$DISPLAY\" && -z \"$WAYLAND_DISPLAY\" ]]; then\n      \
              exec cage foot\n    \
            fi\n  \
          fi\n\
        fi\n",
        SYSTEM_GROUP
    );

    let mut content = if Path::new(profile_path).exists() {
        fs::read_to_string(profile_path)?
    } else {
        String::new()
    };

    if let Some(start_idx) = content.find("# --- Kid-CLI Global Launcher ---") {
        if let Some(end_idx) = content[start_idx..].find("\nfi\nfi\n") {
            let actual_end = start_idx + end_idx + 7;
            content.replace_range(start_idx..actual_end, "");
        } else if let Some(end_idx) = content[start_idx..].find("fi\nfi") {
            let actual_end = start_idx + end_idx + 5;
            content.replace_range(start_idx..actual_end, "");
        } else {
            content.truncate(start_idx);
        }
    }

    content.push_str(&shim);
    fs::write(profile_path, content)?;
    styled_message(MessageLevel::Ok, &format!("Installed/updated global launcher in {}", profile_path));

    Ok(())
}

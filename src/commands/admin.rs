use crate::terminal::{styled_message, MessageLevel};
use anyhow::Result;
use which::which;
use std::process::Command;
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

    // 2. Copy current repo to global path if we are not already there
    let repo_root = std::env::current_dir()?;
    
    if repo_root != Path::new(GLOBAL_PATH) {
        // Safety check: ensure we are in the project root by checking for Cargo.toml
        if !repo_root.join("Cargo.toml").exists() {
            return Err(anyhow::anyhow!(
                "Error: Current directory ({:?}) does not look like the Kid-CLI project root.\n\
                 Please 'cd' into the project folder before running 'admin init'.",
                repo_root
            ));
        }

        // Use rsync to copy excluding build artifacts and git
        let rsync_path = which("rsync")
            .or_else(|_| which("/usr/bin/rsync"))
            .or_else(|_| which("/usr/local/bin/rsync"))
            .unwrap_or_else(|_| std::path::PathBuf::from("rsync"));

        styled_message(MessageLevel::Info, &format!("Using rsync: {:?}", rsync_path));
        styled_message(MessageLevel::Info, &format!("Source dir: {:?}", repo_root));

        let status = Command::new(rsync_path)
            .current_dir(&repo_root)
            .args(&["-a", "--exclude", "target", "--exclude", ".git", ".", GLOBAL_PATH])
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to copy files to /opt/kid-cli"));
        }
    }

    // 3. Create kid-users group
    if cfg!(target_os = "linux") {
        let status = Command::new("groupadd").arg("-f").arg(SYSTEM_GROUP).status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to create system group"));
        }
        styled_message(MessageLevel::Ok, &format!("System group '{}' ensured.", SYSTEM_GROUP));
    } else {
        styled_message(MessageLevel::Warn, &format!("Skipping group creation ({} is not Linux).", std::env::consts::OS));
    }

    // 4. Build initial image and run system install
    styled_message(MessageLevel::Info, "Building initial Docker image and installing system symlinks...");
    let status = Command::new("docker")
        .args(&["compose", "-f", &format!("{}/docker-compose.yml", GLOBAL_PATH), "build"])
        .status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to build initial Docker image"));
    }

    // 5. Install and Symlink binary
    let install_bin_dir = Path::new(GLOBAL_PATH).join("bin");
    let install_bin_path = install_bin_dir.join("kid");
    
    if !install_bin_dir.exists() {
        fs::create_dir_all(&install_bin_dir)?;
    }

    let current_exe = std::env::current_exe()?;
    if install_bin_path.exists() {
        let _ = fs::remove_file(&install_bin_path);
    }
    fs::copy(&current_exe, &install_bin_path)?;
    styled_message(MessageLevel::Ok, &format!("Installed binary to {}", install_bin_path.display()));

    if Path::new(BINARY_PATH).exists() || Path::new(BINARY_PATH).is_symlink() {
        let _ = fs::remove_file(BINARY_PATH);
    }
    std::os::unix::fs::symlink(&install_bin_path, BINARY_PATH)?;
    styled_message(MessageLevel::Ok, &format!("Symlinked binary to {}", BINARY_PATH));

    // 6. Install Global Launcher in /etc/zsh/zprofile
    install_global_launcher()?;

    styled_message(MessageLevel::Ok, "System initialization complete!");
    Ok(())
}

pub fn deploy() -> Result<()> {
    styled_message(MessageLevel::Info, "Deploying updates...");

    let repo_path = Path::new(GLOBAL_PATH);
    if !repo_path.exists() {
        return Err(anyhow::anyhow!("System not initialized. Run 'kid admin init' first."));
    }

    // 1. Pull latest
    styled_message(MessageLevel::Info, "Pulling latest code...");
    let output = Command::new("git").arg("-C").arg(GLOBAL_PATH).args(&["rev-parse", "HEAD"]).output()?;
    let prev_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let status = Command::new("git").arg("-C").arg(GLOBAL_PATH).arg("pull").status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Git pull failed"));
    }

    // 2. Rebuild
    styled_message(MessageLevel::Info, "Rebuilding Docker image...");
    let status = Command::new("docker")
        .args(&["compose", "-f", &format!("{}/docker-compose.yml", GLOBAL_PATH), "build"])
        .status()?;

    if !status.success() {
        styled_message(MessageLevel::Error, "Build failed! Rolling back...");
        Command::new("git").arg("-C").arg(GLOBAL_PATH).args(&["reset", "--hard", &prev_hash]).status()?;
        return Err(anyhow::anyhow!("Deployment failed and rolled back."));
    }

    styled_message(MessageLevel::Ok, "Deployment successful!");
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
        let _ = Command::new("usermod")
            .args(&["-aG", &format!("docker,{}", SYSTEM_GROUP), name])
            .status();
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

fn install_global_launcher() -> Result<()> {
    let profile_path = "/etc/zsh/zprofile";
    let shim = format!(
        "\n# --- Kid-CLI Global Launcher ---\n\
        if [[ -t 0 && -z \"$SKIP_KID\" ]]; then\n  \
          if id -nG | grep -q \"{0}\"; then\n    \
            export KID_CREATIONS_DIR=\"$HOME/creations\"\n    \
            docker compose -f \"{1}/docker-compose.yml\" -p \"kid-$USER\" up -d >/dev/null 2>&1\n    \
            exec docker compose -f \"{1}/docker-compose.yml\" -p \"kid-$USER\" exec kid /bin/zsh -l\n  \
          fi\n\
        fi\n",
        SYSTEM_GROUP, GLOBAL_PATH
    );

    let content = if Path::new(profile_path).exists() {
        fs::read_to_string(profile_path)?
    } else {
        String::new()
    };

    if !content.contains("Kid-CLI Global Launcher") {
        let mut file = fs::OpenOptions::new().append(true).create(true).open(profile_path)?;
        use std::io::Write;
        file.write_all(shim.as_bytes())?;
        styled_message(MessageLevel::Ok, &format!("Installed global launcher in {}", profile_path));
    } else {
        styled_message(MessageLevel::Info, "Global launcher already present.");
    }

    Ok(())
}

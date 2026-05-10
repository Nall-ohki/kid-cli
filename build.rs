use std::process::Command;

fn main() {
    // Get the current git hash
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .ok();

    let git_hash = match output {
        Some(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "unknown".to_string(),
    };

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    
    // Also bake in a build timestamp for status reporting
    let timestamp = Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
        
    println!("cargo:rustc-env=BUILD_TIME={}", timestamp);
    
    // Ensure we re-run if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}

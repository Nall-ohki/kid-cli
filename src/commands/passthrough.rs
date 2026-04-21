use anyhow::Result;
use crate::commands::event;

pub async fn run(name: &str, real_binary: &str, args: &[String]) -> Result<()> {
    // 1. Notify daemon (state capture)
    let full_cmd = if args.is_empty() {
        name.to_string()
    } else {
        format!("{} {}", name, args.join(" "))
    };
    
    // Fire-and-forget event
    let _ = event::run("exec", &full_cmd, None).await;

    // 2. Transhumanistically replace ourselves with the real binary
    // Using nix::unistd::execv for a true passthrough
    use nix::unistd::execv;
    use std::ffi::CString;

    let path = CString::new(real_binary)?;
    let mut c_args = vec![CString::new(name)?];
    for arg in args {
        c_args.push(CString::new(arg.as_str())?);
    }

    execv(&path, &c_args)?;

    Ok(())
}

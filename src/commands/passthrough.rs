use anyhow::Result;

pub async fn run(name: &str, real_binary: &str, args: &[String]) -> Result<()> {
    // Transhumanistically replace ourselves with the real binary
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

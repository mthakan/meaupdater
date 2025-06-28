// src/policy.rs
use anyhow::{bail, Context, Result};
use std::process::Command;

/// Testable command generator
pub fn build_install_command(pkgs: &[String]) -> String {
    format!("apt update && apt install --only-upgrade -y {}", pkgs.join(" "))
}

/// Installs selected packages with `pkexec`.
pub fn install_packages(pkgs: &[String]) -> Result<()> {
    if pkgs.is_empty() {
        bail!("No package selected");
    }
    let cmd = build_install_command(pkgs);
    let status = Command::new("pkexec")
        .arg("sh")
        .arg("-c")
        .arg(&cmd)
        .status()
        .context("`pkexec` failed to start")?;
    if !status.success() {
        bail!("`apt install` error code {}", status);
    }
    Ok(())
}

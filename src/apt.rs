// src/apt.rs

use crate::model::{PackageUpdate, UpdateType};
use anyhow::{Context, Result};
use std::process::Command;

/// Function that parses apt list output (testable).
pub fn parse_apt_list_output(s: &str) -> Vec<PackageUpdate> {
    let mut packages = Vec::new();

    for (i, line) in s.lines().enumerate() {
        // First line "Listing..." and skip blank lines
        if i == 0 || line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        // Sample line:
        // bash/stable 5.1-2+deb11u1 amd64 [upgradable from: 5.1-2]
        let name = parts[0]
            .split('/')
            .next()
            .unwrap_or("")
            .to_string();

        let new_version = parts[1].to_string();

        // From parts like "[upgradable", "from:", "5.1-2]"
        // Next track after "from:" current_version
        let current_version = if let Some(idx) = parts.iter().position(|p| *p == "from:") {
            parts.get(idx + 1)
                .map(|p| p.trim_end_matches(']').to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        // Allocate security update by checking repository name ("pkg/repo" in parts[0])
        let repo = parts[0];
        let update_type = if repo.contains("/security") {
            UpdateType::Security
        } else {
            UpdateType::Software
        };

        packages.push(PackageUpdate {
            name,
            current_version,
            new_version,
            update_type,
        });
    }

    packages
}

/// Gets upgradeable packages on the system.
pub fn get_upgradable_packages() -> Result<Vec<PackageUpdate>> {
    let output = Command::new("apt")
        .args(&["list", "--upgradable"])
        .env("LANG", "C")
        .output()
        .context("`apt list --upgradable` failed to run")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_apt_list_output(&stdout))
}

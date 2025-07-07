// src/apt.rs

use crate::model::{PackageUpdate, UpdateType};
use anyhow::{Context, Result};
use std::process::Command;
use std::collections::HashMap;

/// Converts size to readable format
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{:.0} {}", size, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}


fn determine_update_type(package_name: &str, repository: &str) -> UpdateType {
    
    if is_kernel_package(package_name) {
        return UpdateType::Kernel;
    }
    
    
    if repository.contains("/security") || 
       repository.contains("-security") ||
       repository.contains("security.") {
        return UpdateType::Security;
    }
    
    
    let security_packages = [
        "openssl", "ssl", "gnutls", "ssh", "openssh", "curl", "wget",
        "firefox", "chromium", "thunderbird", "libreoffice",
        "apache", "nginx", "php", "mysql", "postgresql",
        "bind9", "dnsutils", "iptables", "ufw", "fail2ban",
        "systemd", "sudo", "polkit", "pam", "login", "passwd",
        "gpg", "gnupg", "ca-certificates", "certbot",
        "kernel", "linux-", "firmware", "microcode",
        "libc", "glibc", "zlib", "expat", "libxml", "libpng",
        "jpeg", "tiff", "git", "subversion", "rsync"
    ];
    
    let package_lower = package_name.to_lowercase();
    for security_pkg in &security_packages {
        if package_lower.contains(security_pkg) {
            return UpdateType::Security;
        }
    }
    
    
    UpdateType::Software
}


fn is_kernel_package(package_name: &str) -> bool {
    let kernel_keywords = [
        "linux-image",
        "linux-headers", 
        "linux-modules",
        "linux-firmware",
        "linux-generic",
        "linux-lowlatency", 
        "linux-oem",
        "linux-hwe",
        "linux-virtual",
        "linux-tools",
        "linux-cloud-tools",
        "linux-signed",
        "linux-restricted-modules"
    ];
    
    let package_lower = package_name.to_lowercase();
    for keyword in &kernel_keywords {
        if package_lower.starts_with(keyword) {
            return true;
        }
    }
    
    false
}


pub fn get_package_sizes(package_names: &[String]) -> HashMap<String, String> {
    let mut sizes = HashMap::new();
    
    if package_names.is_empty() {
        return sizes;
    }
    
    // Get all packages with a single apt show command
    let output = Command::new("apt")
        .arg("show")
        .args(package_names)
        .env("LANG", "C")
        .output();
    
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut current_package = String::new();
        
        for line in stdout.lines() {
            let line = line.trim();
            
            if line.starts_with("Package:") {
                if let Some(pkg_name) = line.split_whitespace().nth(1) {
                    current_package = pkg_name.to_string();
                }
            } else if line.starts_with("Size:") && !current_package.is_empty() {
                if let Some(size_str) = line.split_whitespace().nth(1) {
                    if let Ok(size_bytes) = size_str.parse::<u64>() {
                        sizes.insert(current_package.clone(), format_size(size_bytes));
                    }
                }
                
                current_package.clear();
            }
        }
    }
    
    // If apt show doesn't get the size, try apt-cache show
    let missing_packages: Vec<String> = package_names.iter()
        .filter(|pkg| !sizes.contains_key(*pkg))
        .cloned()
        .collect();
    
    if !missing_packages.is_empty() {
        if let Ok(output) = Command::new("apt-cache")
            .arg("show")
            .args(&missing_packages)
            .env("LANG", "C")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut current_package = String::new();
            
            for line in stdout.lines() {
                let line = line.trim();
                
                if line.starts_with("Package:") {
                    if let Some(pkg_name) = line.split_whitespace().nth(1) {
                        current_package = pkg_name.to_string();
                    }
                } else if line.starts_with("Size:") && !current_package.is_empty() {
                    if let Some(size_str) = line.split_whitespace().nth(1) {
                        if let Ok(size_bytes) = size_str.parse::<u64>() {
                            sizes.insert(current_package.clone(), format_size(size_bytes));
                        }
                    }
                    current_package.clear();
                } else if line.starts_with("Installed-Size:") && !current_package.is_empty() && !sizes.contains_key(&current_package) {
                    // If Size is not available use Installed-Size (in KB)
                    if let Some(size_str) = line.split_whitespace().nth(1) {
                        if let Ok(size_kb) = size_str.parse::<u64>() {
                            sizes.insert(current_package.clone(), format_size(size_kb * 1024));
                        }
                    }
                    current_package.clear();
                }
            }
        }
    }
    
    // Default value for packages whose size is not found
    for pkg_name in package_names {
        if !sizes.contains_key(pkg_name) {
            sizes.insert(pkg_name.clone(), "N/A".to_string());
        }
    }
    
    sizes
}

/// Function that parses the apt list output
pub fn parse_apt_list_output(s: &str) -> Vec<PackageUpdate> {
    let mut packages = Vec::new();
    let mut package_names = Vec::new();

    // First collect all package information
    for (i, line) in s.lines().enumerate() {
        // First line "Listing..." and skip empty lines
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

        
        
        let current_version = if let Some(idx) = parts.iter().position(|p| *p == "from:") {
            parts.get(idx + 1)
                .map(|p| p.trim_end_matches(']').to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        
        let repo = parts[0];
        let update_type = determine_update_type(&name, repo);

        packages.push(PackageUpdate {
            name: name.clone(),
            current_version,
            new_version,
            update_type,
            size: String::new(), 
        });
        
        package_names.push(name);
    }

    
    let sizes = get_package_sizes(&package_names);
    
    
    for pkg in &mut packages {
        if let Some(size) = sizes.get(&pkg.name) {
            pkg.size = size.clone();
        } else {
            pkg.size = "N/A".to_string();
        }
    }

    packages
}


pub fn get_upgradable_packages() -> Result<Vec<PackageUpdate>> {
    let output = Command::new("apt")
        .args(&["list", "--upgradable"])
        .env("LANG", "C")
        .output()
        .context("Could not run `apt list --upgradable`")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_apt_list_output(&stdout))
}

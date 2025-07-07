// src/kernel_manager.rs

use anyhow::{Context, Result, bail};
use std::fs;
use std::process::Command;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
pub enum KernelType {
    LTS,
    Mainline,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct KernelInfo {
    pub version: String,
    pub full_version: String,
    pub kernel_type: KernelType,
    pub is_installed: bool,
    pub is_current: bool,
    pub major_version: String, 
    pub package_name: String,
    pub size: String,
}

impl KernelInfo {
    pub fn new(package_name: &str, version: &str, is_installed: bool) -> Self {
        let full_version = version.to_string();
        let major_version = Self::extract_major_version(version);
        let kernel_type = Self::determine_kernel_type(version);
        
        Self {
            version: version.to_string(),
            full_version,
            kernel_type,
            is_installed,
            is_current: false,
            major_version,
            package_name: package_name.to_string(),
            size: "N/A".to_string(),
        }
    }
    
    fn extract_major_version(version: &str) -> String {

        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() >= 2 {
            format!("{}.{}", parts[0], parts[1])
        } else {
            version.to_string()
        }
    }
    
    fn determine_kernel_type(version: &str) -> KernelType {

        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() >= 2 {
            if let Ok(major) = parts[0].parse::<i32>() {
                if let Ok(minor) = parts[1].parse::<i32>() {
                    // LTS Versions
                    match (major, minor) {
                        (6, 12) => return KernelType::LTS, // New LTS (Aralık 2024)
                        (6, 6) => return KernelType::LTS,  // LTS
                        (6, 1) => return KernelType::LTS,  // LTS
                        (5, 15) => return KernelType::LTS, // LTS
                        (5, 10) => return KernelType::LTS, // LTS
                        (5, 4) => return KernelType::LTS,  // LTS
                        (4, 19) => return KernelType::LTS, // Old LTS
                        _ => {}
                    }
                }
            }
        }
        

        KernelType::Mainline
    }
}


static LAST_KERNEL_CHECK: Mutex<Option<u64>> = Mutex::new(None);
static KERNEL_CACHE: Mutex<Option<Vec<KernelInfo>>> = Mutex::new(None);
const KERNEL_CACHE_DURATION: u64 = 900; 

pub fn needs_kernel_check() -> bool {
    if let Ok(last_check_guard) = LAST_KERNEL_CHECK.lock() {
        if let Some(last_check) = *last_check_guard {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            return current_time - last_check > KERNEL_CACHE_DURATION;
        }
    }
    true
}

pub fn update_kernel_check_time() {
    if let Ok(mut last_check_guard) = LAST_KERNEL_CHECK.lock() {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        *last_check_guard = Some(current_time);
    }
}

pub fn get_cached_kernels() -> Option<Vec<KernelInfo>> {
    if let Ok(cache_guard) = KERNEL_CACHE.lock() {
        cache_guard.clone()
    } else {
        None
    }
}

pub fn set_kernel_cache(kernels: Vec<KernelInfo>) {
    if let Ok(mut cache_guard) = KERNEL_CACHE.lock() {
        *cache_guard = Some(kernels);
    }
}


pub fn get_current_kernel() -> Result<String> {
    let output = Command::new("uname")
        .arg("-r")
        .output()
        .context("Could not get current kernel version")?;
    if !output.status.success() {
        bail!("uname command failed");
    }
    
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(version)
}


fn kernels_match(kernel1: &str, kernel2: &str) -> bool {
    if kernel1 == kernel2 {
        return true;
    }
    

    let clean1 = kernel1.split('/').next().unwrap_or(kernel1);
    let clean2 = kernel2.split('/').next().unwrap_or(kernel2);
    if clean1 == clean2 {
        return true;
    }
    

    let base1 = clean1.replace("-unsigned", "").replace("-dbg", "");
    let base2 = clean2.replace("-unsigned", "").replace("-dbg", "");
    

    base1 == base2
}


pub fn get_installed_kernels() -> Result<Vec<KernelInfo>> {
    let output = Command::new("dpkg")
        .args(&["--list", "linux-image-*"])
        .env("LANG", "C")
        .output()
        .context("Could not get installed kernel list")?;
        
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut kernels = Vec::new();
    let current_kernel = get_current_kernel().unwrap_or_default();
    let mut seen_versions = std::collections::HashSet::new();
    
    for line in stdout.lines() {
        if line.contains("linux-image-") && (line.starts_with("ii") || line.starts_with("hi")) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let package_name = parts[1];
                let version = parts[2];

                if package_name.contains("-unsigned") || 
                   package_name.contains("-dbg") ||
                   package_name.contains("-headers") {
                    continue;
                }
                

                if let Some(kernel_version) = extract_kernel_version_from_package(package_name) {

                    let clean_version = kernel_version.split('/').next().unwrap_or(&kernel_version).to_string();
                    

                    if seen_versions.contains(&clean_version) {
                        continue;
                    }
                    seen_versions.insert(clean_version.clone());
                    
                    let mut kernel = KernelInfo::new(package_name, &clean_version, true);
                    let current_clean = current_kernel.split('/').next().unwrap_or(&current_kernel);
                    kernel.is_current = current_clean == clean_version;
                    kernels.push(kernel);
                }
            }
        }
    }
    
    Ok(kernels)
}


pub fn get_available_kernels() -> Result<Vec<KernelInfo>> {
    let output = Command::new("apt")
        .args(&["search", "^linux-image-[0-9]"])
        .env("LANG", "C")
        .output()
        .context("Could not get available kernel list")?;
        
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut kernels = Vec::new();
    let installed_kernels = get_installed_kernels().unwrap_or_default();
    let current_kernel = get_current_kernel().unwrap_or_default();
    let mut seen_versions = std::collections::HashSet::new();
    
    for line in stdout.lines() {
        if line.contains("linux-image-") && !line.starts_with("WARNING") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(package_part) = parts.first() {
                let package_name = package_part.trim_end_matches('/');
                
                if let Some(kernel_version) = extract_kernel_version_from_package(package_name) {

                    if package_name.contains("-unsigned") || 
                       package_name.contains("-dbg") ||
                       package_name.contains("-headers") {
                        continue;
                    }
                    

                    let clean_version = kernel_version.split('/').next().unwrap_or(&kernel_version).to_string();
                    

                    if seen_versions.contains(&clean_version) {
                        continue;
                    }
                    seen_versions.insert(clean_version.clone());
                    

                    let is_installed = installed_kernels.iter()
                        .any(|k| {
                            let k_clean = k.version.split('/').next().unwrap_or(&k.version);
                            k.package_name == package_name || 
                            (k_clean == clean_version && 
                             !k.package_name.contains("-unsigned") && 
                             !k.package_name.contains("-dbg"))
                        });
                    
                    let mut kernel = KernelInfo::new(package_name, &clean_version, is_installed);

                    let current_clean = current_kernel.split('/').next().unwrap_or(&current_kernel);
                    kernel.is_current = current_clean == clean_version;
                    kernels.push(kernel);
                }
            }
        }
    }
    

    if !current_kernel.is_empty() {
        let current_clean = current_kernel.split('/').next().unwrap_or(&current_kernel);
        let current_in_list = kernels.iter().any(|k| k.is_current || k.version == current_clean);
        if !current_in_list {

            let estimated_package = format!("linux-image-{}", current_clean);
            let mut current_kernel_info = KernelInfo::new(&estimated_package, current_clean, true);
            current_kernel_info.is_current = true;
            kernels.push(current_kernel_info);
        }
    }
    

    for installed in installed_kernels {
        if installed.package_name.contains("-unsigned") || 
           installed.package_name.contains("-dbg") ||
           installed.package_name.contains("-headers") {
            continue;
        }
        
        let installed_clean = installed.version.split('/').next().unwrap_or(&installed.version);
        let exists_in_list = kernels.iter().any(|k| 
            k.package_name == installed.package_name ||
            k.version == installed_clean
        );
        
        if !exists_in_list {
            let mut clean_installed = installed.clone();
            clean_installed.version = installed_clean.to_string();
            kernels.push(clean_installed);
        }
    }
    
    Ok(kernels)
}


fn extract_kernel_version_from_package(package_name: &str) -> Option<String> {

    if package_name.starts_with("linux-image-") {
        let version_part = package_name.strip_prefix("linux-image-")?;

        if version_part.chars().next()?.is_ascii_digit() {
            Some(version_part.to_string())
        } else {
            None
        }
    } else {
        None
    }
}


pub fn group_kernels_by_major_version(kernels: Vec<KernelInfo>) -> HashMap<String, Vec<KernelInfo>> {
    let mut groups: HashMap<String, Vec<KernelInfo>> = HashMap::new();
    
    for kernel in kernels {
        groups.entry(kernel.major_version.clone()).or_insert_with(Vec::new).push(kernel);
    }
    

    for group in groups.values_mut() {
        group.sort_by(|a, b| {

            version_compare(&b.version, &a.version) 
        });
    }
    
    groups
}


fn version_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<&str> = a.split(&['.', '-'][..]).collect();
    let b_parts: Vec<&str> = b.split(&['.', '-'][..]).collect();
    
    let max_len = a_parts.len().max(b_parts.len());
    
    for i in 0..max_len {
        let a_part = a_parts.get(i).unwrap_or(&"0");
        let b_part = b_parts.get(i).unwrap_or(&"0");
        

        match (a_part.parse::<i32>(), b_part.parse::<i32>()) {
            (Ok(a_num), Ok(b_num)) => {
                match a_num.cmp(&b_num) {
                    std::cmp::Ordering::Equal => continue,
                    other => return other,
                }
            }
            _ => {

                match a_part.cmp(b_part) {
                    std::cmp::Ordering::Equal => continue,
                    other => return other,
                }
            }
        }
    }
    
    std::cmp::Ordering::Equal
}


pub fn get_kernel_sizes(package_names: &[String]) -> HashMap<String, String> {
    let mut sizes = HashMap::new();
    
    if package_names.is_empty() {
        return sizes;
    }
    

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
    

    for pkg_name in package_names {
        if !sizes.contains_key(pkg_name) {
            sizes.insert(pkg_name.clone(), "~50 MB".to_string()); 
        }
    }
    
    sizes
}


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


pub fn install_kernel(package_name: &str) -> Result<()> {
    let status = Command::new("pkexec")
        .args(&["apt", "install", "-y", package_name])
        .status()
        .context("Kernel install command could not be executed")?;
        
    if !status.success() {
        bail!("Kernel installation failed");
    }
    
    Ok(())
}


pub fn remove_kernel_with_autoremove(package_name: &str, current_kernel: &str) -> Result<()> {

    if let Some(kernel_version) = extract_kernel_version_from_package(package_name) {
        if kernels_match(current_kernel, &kernel_version) {
            bail!("The current running kernel cannot be removed.");
        }
    }
    
    println!("🗑️ Kernel is being removed: {}", package_name);
    

    let kernel_packages = find_related_kernel_packages(package_name)?;
    
    println!("📋 Packages to remove: {:?}", kernel_packages);
    

    for package in &kernel_packages {
        println!("🗑️ Removing: {}", package);
        
        let status = Command::new("pkexec")
            .args(&["apt", "remove", "--purge", "-y", package])
            .status()
            .context("Uninstall kernel command failed to execute")?;
            
        if !status.success() {
            println!("⚠️ {} package removal failed, continuing...", package);
        }
    }
    

    println!("🧹 Orphaned packages are being cleaned...");
    let autoremove_status = Command::new("pkexec")
        .args(&["apt", "autoremove", "-y"])
        .status()
        .context("Autoremove command failed to execute")?;
        
    if !autoremove_status.success() {
        println!("⚠️ Autoremove operation failed");
    }
    
    println!("✅ Kernel removal completed");
    Ok(())
}


fn find_related_kernel_packages(main_package: &str) -> Result<Vec<String>> {
    let mut packages = Vec::new();
    

    packages.push(main_package.to_string());
    

    if let Some(kernel_version) = extract_kernel_version_from_package(main_package) {

        let package_types = [
            "linux-image",
            "linux-headers", 
            "linux-modules",
            "linux-modules-extra",
            "linux-image-unsigned",
            "linux-headers-generic",
        ];
        

        let dpkg_output = Command::new("dpkg")
            .args(&["-l"])
            .output()
            .context("dpkg list retrieval failed")?;
        
        let stdout = String::from_utf8_lossy(&dpkg_output.stdout);
        
        for line in stdout.lines() {
            if line.starts_with("ii") { 
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let package_name = parts[1];
                    

                    for package_type in &package_types {
                        let expected_package = format!("{}-{}", package_type, kernel_version);
                        if package_name == expected_package && !packages.contains(&package_name.to_string()) {
                            packages.push(package_name.to_string());
                            println!("🔍 Related package found: {}", package_name);
                        }
                    }
                }
            }
        }
    }
    
    println!("📋 Total {} related packages found", packages.len());
    Ok(packages)
}


fn detect_linux_mint_de() -> Option<String> {

    if let Ok(current_desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        let desktop = current_desktop.to_lowercase();
        if desktop.contains("cinnamon") {
            return Some("Cinnamon".to_string());
        } else if desktop.contains("xfce") {
            return Some("Xfce".to_string());
        } else if desktop.contains("mate") {
            return Some("MATE".to_string());
        } else if desktop.contains("kde") || desktop.contains("plasma") {
            return Some("KDE".to_string());
        } else if desktop.contains("gnome") {
            return Some("GNOME".to_string());
        }
    }
    //2025 mthakan

    if let Ok(desktop_session) = std::env::var("DESKTOP_SESSION") {
        let session = desktop_session.to_lowercase();
        if session.contains("cinnamon") {
            return Some("Cinnamon".to_string());
        } else if session.contains("xfce") {
            return Some("Xfce".to_string());
        } else if session.contains("mate") {
            return Some("MATE".to_string());
        }
    }
    

    let de_processes = [
        ("cinnamon", "Cinnamon"),
        ("xfce4-session", "Xfce"),
        ("mate-session", "MATE"),
        ("plasmashell", "KDE"),
        ("gnome-shell", "GNOME"),
    ];
    
    for (process, de_name) in &de_processes {
        if let Ok(output) = Command::new("pgrep").arg(process).output() {
            if output.status.success() && !output.stdout.is_empty() {
                return Some(de_name.to_string());
            }
        }
    }
    

    let de_packages = [
        ("cinnamon", "Cinnamon"),
        ("xfce4", "Xfce"),
        ("mate-desktop", "MATE"),
        ("plasma-desktop", "KDE"),
        ("gnome-shell", "GNOME"),
    ];
    
    for (package, de_name) in &de_packages {
        if let Ok(output) = Command::new("dpkg").args(&["-l", package]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.lines().any(|line| line.starts_with("ii")) {
                    return Some(de_name.to_string());
                }
            }
        }
    }
    
    None
}


fn detect_distribution_name() -> String {

    let grub_cfg_paths = [
        "/boot/grub/grub.cfg",
        "/boot/grub2/grub.cfg", 
        "/boot/efi/EFI/debian/grub.cfg",
        "/boot/efi/EFI/ubuntu/grub.cfg"
    ];
    

    for path in &grub_cfg_paths {
        if std::path::Path::new(path).exists() {
            if let Ok(grub_content) = fs::read_to_string(path) {
                for line in grub_content.lines() {
                    let line = line.trim();
                    

                    if line.starts_with("submenu ") && line.contains("Advanced options for") {
                        if let Some(title_start) = line.find("'") {
                            if let Some(title_end) = line.rfind("'") {
                                let submenu_title = line[title_start+1..title_end].to_string();
                                

                                if let Some(distro_part) = submenu_title.strip_prefix("Advanced options for ") {
                                    println!("🎯 Distribution detected from GRUB: {}", distro_part);
                                    return distro_part.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    

    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        let mut name = String::new();
        let mut id = String::new();
        let mut version = String::new();
        
        for line in content.lines() {
            if line.starts_with("NAME=") {
                name = line.trim_start_matches("NAME=").trim_matches('"').to_string();
            } else if line.starts_with("ID=") {
                id = line.trim_start_matches("ID=").trim_matches('"').to_string();
            } else if line.starts_with("VERSION=") {
                version = line.trim_start_matches("VERSION=").trim_matches('"').to_string();
            }
        }
        

        if id == "lmde" || name.to_lowercase().contains("lmde") {

            if !version.is_empty() {
                return format!("LMDE {}", version);
            } else {
                return "LMDE".to_string();
            }
        }
        

        if id == "linuxmint" || name.to_lowercase().contains("linux mint") {
            if let Some(de) = detect_linux_mint_de() {

                let version_parts: Vec<&str> = version.split_whitespace().collect();
                let version_str = version.as_str();
                let version_number = version_parts.first().unwrap_or(&version_str);
                
                return format!("Linux Mint {} {}", version_number, de);
            } else {

                return format!("Linux Mint {}", version);
            }
        }
        

        if !name.is_empty() {
            return normalize_distro_name(&name);
        }
    }
    

    if let Ok(output) = Command::new("lsb_release").args(&["-si"]).output() {
        if output.status.success() {
            let distro_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !distro_name.is_empty() {

                if distro_name.to_lowercase().contains("mint") {
                    if let Some(de) = detect_linux_mint_de() {

                        if let Ok(version_output) = Command::new("lsb_release").args(&["-sr"]).output() {
                            if version_output.status.success() {
                                let version_string = String::from_utf8_lossy(&version_output.stdout);
                                let version = version_string.trim();
                                return format!("Linux Mint {} {}", version, de);
                            }
                        }
                        return format!("Linux Mint {}", de);
                    }
                }
                return normalize_distro_name(&distro_name);
            }
        }
    }
    

    if std::path::Path::new("/etc/debian_version").exists() {
        return "Debian GNU/Linux".to_string();
    }
    

    "Debian GNU/Linux".to_string()
}


fn normalize_distro_name(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "debian" => "Debian GNU/Linux".to_string(),
        "linuxmint" | "linux mint" => {

            if let Some(de) = detect_linux_mint_de() {
                format!("Linux Mint {}", de)
            } else {
                "Linux Mint".to_string()
            }
        },
        "ubuntu" => "Ubuntu".to_string(),
        "kali" | "kali gnu/linux" => "Kali GNU/Linux".to_string(),
        "elementary" | "elementary os" => "elementary OS".to_string(),
        _ => {

            if name.to_lowercase().contains("lmde") {
                return name.to_string(); 
            }

            else if name.contains("mint") || name.contains("Mint") {
                if let Some(de) = detect_linux_mint_de() {
                    format!("Linux Mint {}", de)
                } else {
                    "Linux Mint".to_string()
                }
            } else if name.contains("debian") || name.contains("Debian") {
                "Debian GNU/Linux".to_string()
            } else {
                name.to_string()
            }
        }
    }
}


fn is_lmde_system() -> bool {

    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("ID=") {
                let id = line.trim_start_matches("ID=").trim_matches('"').to_string();
                if id == "lmde" {
                    return true;
                }
            }
            if line.starts_with("NAME=") {
                let name = line.trim_start_matches("NAME=").trim_matches('"').to_string();
                if name.to_lowercase().contains("lmde") {
                    return true;
                }
            }
        }
    }
    

    if let Ok(output) = Command::new("lsb_release").args(&["-si"]).output() {
        if output.status.success() {
            let distro_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if distro_name.to_lowercase().contains("lmde") {
                return true;
            }
        }
    }
    
    false
}


pub fn set_default_kernel(kernel_version: &str) -> Result<()> {

    let clean_version = kernel_version.split('/').next().unwrap_or(kernel_version);
    
    println!("🔍 Updating GRUB settings...");
    println!("📋 Target kernel: {}", clean_version);
    

    if is_lmde_system() {
        println!("🐧 LMDE sistemi algılandı, özel format kullanılıyor...");
        let lmde_entry = format!("Advanced options for LMDE 6 Faye>LMDE 6 Faye, with Linux {}", clean_version);
        println!("🎯 LMDE special entry: {}", lmde_entry);
        return set_grub_default_via_config(&lmde_entry, clean_version);
    }
    
    println!("🔍 Updating GRUB settings...");
    println!("📋 Target kernel: {}", clean_version);
    

    let grub_cfg_paths = [
        "/boot/grub/grub.cfg",
        "/boot/grub2/grub.cfg", 
        "/boot/efi/EFI/debian/grub.cfg",
        "/boot/efi/EFI/ubuntu/grub.cfg"
    ];
    
    let mut grub_cfg_path = None;
    for path in &grub_cfg_paths {
        if std::path::Path::new(path).exists() {
            grub_cfg_path = Some(*path);
            println!("📁 GRUB config found: {}", path);
            break;
        }
    }
    

    if let Some(cfg_path) = grub_cfg_path {
        if let Ok(grub_content) = fs::read_to_string(cfg_path) {
            println!("🔍 Analyzing GRUB menu structure...");
            
            let mut found_entry = None;
            let mut current_submenu = String::new();
            let mut in_advanced_options = false;
            
            for line in grub_content.lines() {
                let line = line.trim();
                

                if line.starts_with("submenu ") {
                    if let Some(title_start) = line.find("'") {
                        if let Some(title_end) = line.rfind("'") {
                            current_submenu = line[title_start+1..title_end].to_string();
                            

                            let lower_submenu = current_submenu.to_lowercase();
                            if lower_submenu.contains("advanced") || 
                               lower_submenu.contains("gelişmiş") ||
                               lower_submenu.contains("options") ||
                               lower_submenu.contains("seçenekler") {
                                in_advanced_options = true;
                                println!("📂 Advanced submenu found: {}", current_submenu);
                            }
                        }
                    }
                }
                

                if in_advanced_options && line.starts_with("menuentry ") && line.contains(&clean_version) {
                    if let Some(title_start) = line.find("'") {
                        if let Some(title_end) = line.rfind("'") {
                            let entry_title = line[title_start+1..title_end].to_string();
                            let full_path = format!("{}>{}", current_submenu, entry_title);
                            found_entry = Some(full_path.clone());
                            println!("🎯 Kernel entry found: {}", full_path);
                            break;
                        }
                    }
                }
                

                if line == "}" && in_advanced_options {
                    in_advanced_options = false;
                }
            }
            

            if let Some(entry) = found_entry {
                return set_grub_default_via_config(&entry, clean_version);
            }
        }
    }
    

    println!("⚠️ Automatic analysis failed, scanning submenu names from GRUB config...");
    
    if let Some(cfg_path) = grub_cfg_path {
        if let Ok(grub_content) = fs::read_to_string(cfg_path) {

            let mut real_distro_name = None;
            
            for line in grub_content.lines() {
                let line = line.trim();
                

                if line.starts_with("submenu ") && line.contains("Advanced options for") {
                    if let Some(title_start) = line.find("'") {
                        if let Some(title_end) = line.rfind("'") {
                            let submenu_title = line[title_start+1..title_end].to_string();
                            

                            if let Some(distro_part) = submenu_title.strip_prefix("Advanced options for ") {
                                real_distro_name = Some(distro_part.to_string());
                                println!("🎯 Real distribution name detected from GRUB: {}", distro_part);
                                break;
                            }
                        }
                    }
                }
            }
            

            if let Some(distro_name) = real_distro_name {

                let mut in_advanced_submenu = false;
                let mut found_main_entry = None;
                
                for sub_line in grub_content.lines() {
                    let sub_line = sub_line.trim();
                    

                    if sub_line.starts_with("submenu ") && sub_line.contains("Advanced options for") {
                        in_advanced_submenu = true;
                        continue;
                    }
                    

                    if in_advanced_submenu && sub_line.starts_with("menuentry ") && 
                       sub_line.contains(&clean_version) && 
                       !sub_line.to_lowercase().contains("recovery") {
                        if let Some(entry_start) = sub_line.find("'") {
                            if let Some(entry_end) = sub_line.rfind("'") {
                                let entry_title = sub_line[entry_start+1..entry_end].to_string();
                                found_main_entry = Some(entry_title);
                                break;
                            }
                        }
                    }
                    

                    if in_advanced_submenu && sub_line == "}" {
                        break;
                    }
                }
                

                if let Some(main_entry) = found_main_entry {
                    let grub_entry = format!("Advanced options for {}>{}", distro_name, main_entry);
                    println!("🎯 Full entry found from GRUB config: {}", grub_entry);
                    return set_grub_default_via_config(&grub_entry, clean_version);
                } else {

                    let manual_entry = format!("Advanced options for {}>{}, with Linux {}", distro_name, distro_name, clean_version);
                    println!("🔧 Manual entry with real distribution name: {}", manual_entry);
                    return set_grub_default_via_config(&manual_entry, clean_version);
                }
            }
        }
    }
    

    let distro_name = detect_distribution_name();
    let manual_entry = format!("Advanced options for {}>{}, with Linux {}", distro_name, distro_name, clean_version);
    println!("🔧 Trying manual entry as a last resort: {}", manual_entry);
    
    set_grub_default_via_config(&manual_entry, clean_version)
}

fn set_grub_default_via_config(entry: &str, kernel_version: &str) -> Result<()> {
    let grub_file = "/etc/default/grub";
    
    println!("🔧 Updating /etc/default/grub file...");
    

    let combined_script = format!(r#"
# Create backup
cp {grub_file} {grub_file}.backup-$(date +%Y%m%d-%H%M%S)

# Delete old GRUB_DEFAULT lines
sed -i '/^GRUB_DEFAULT=/d' {grub_file}
sed -i '/^GRUB_SAVEDEFAULT=/d' {grub_file}
sed -i '/^GRUB_TIMEOUT=/d' {grub_file}
sed -i '/^GRUB_HIDDEN_TIMEOUT=/d' {grub_file}
sed -i '/^#GRUB_HIDDEN_TIMEOUT=/d' {grub_file}
sed -i '/^GRUB_TIMEOUT_STYLE=/d' {grub_file}

# Add new settings
echo 'GRUB_DEFAULT="{entry}"' >> {grub_file}
echo 'GRUB_SAVEDEFAULT=false' >> {grub_file}
echo 'GRUB_TIMEOUT=10' >> {grub_file}
echo 'GRUB_TIMEOUT_STYLE=menu' >> {grub_file}

echo "✅ GRUB settings updated"
"#, grub_file = grub_file, entry = entry);


    let status = Command::new("pkexec")
        .args(&["sh", "-c", &combined_script])
        .status();
        
    match status {
        Ok(exit_status) => {
            if exit_status.success() {
                println!("🔧 GRUB_DEFAULT is set: {}", entry);
            } else {
                bail!("GRUB config update failed");
            }
        }
        Err(e) => {
            bail!("GRUB config update error: {}", e);
        }
    }
    

    println!("🔄 Updating GRUB config...");
    
    let update_commands = [
        "update-grub",
        "/usr/sbin/update-grub", 
        "/usr/bin/update-grub",
        "grub-mkconfig -o /boot/grub/grub.cfg",
        "/usr/sbin/grub-mkconfig -o /boot/grub/grub.cfg",
        "grub2-mkconfig -o /boot/grub2/grub.cfg",
        "/usr/sbin/grub2-mkconfig -o /boot/grub2/grub.cfg",
    ];
    
    let mut update_success = false;
    
    for cmd_str in &update_commands {

        let first_part = cmd_str.split_whitespace().next().unwrap_or("");
        if let Ok(which_output) = Command::new("which").arg(first_part).output() {
            if which_output.status.success() {
                println!("   Trying: {}", cmd_str);
                
                let cmd_parts: Vec<&str> = cmd_str.split_whitespace().collect();
                let status = Command::new("pkexec")
                    .args(&cmd_parts)
                    .status();
                    
                if let Ok(status) = status {
                    if status.success() {
                        println!("   ✅ Successful: {}", cmd_str);
                        update_success = true;
                        break;
                    }
                }
            }
        }
    }
    
    if update_success {
        println!("✅ Default kernel set successfully!");
        println!("🔄 Kernel '{}' will be selected automatically when the system is rebooted", kernel_version);
        

        println!("\n🧪 Test command:");
        println!("   sudo grub-reboot \"{}\"", entry);
        println!("   sudo reboot");
        
        Ok(())
    } else {
        println!("\n🛠️ Manual commands:");
        println!("1. Check GRUB settings:");
        println!("   cat /etc/default/grub | grep GRUB_DEFAULT");
        println!();
        println!("2. Manual GRUB update:");
        println!("   sudo update-grub");
        println!("   # veya");
        println!("   sudo grub-mkconfig -o /boot/grub/grub.cfg");
        println!();
        println!("3. One-time test:");
        println!("   sudo grub-reboot \"{}\"", entry);
        
        bail!("GRUB config update failed")
    }
}

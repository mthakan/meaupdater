// src/driver_manager.rs

use anyhow::{Context, Result, bail};
use std::fs;
use std::process::Command;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DriverType {
    Graphics,      
    Network,         
    Audio,         
    Bluetooth,     
    Chipset,       
    Storage,       
    Input,         
    Other,         
}

#[derive(Debug, Clone, PartialEq)]
pub enum DriverLicense {
    Free,          
    NonFree,       
    Unknown,       
}

#[derive(Debug, Clone)]
pub struct DriverInfo {
    pub name: String,
    pub description: String,
    pub package_name: String,
    pub version: String,
    pub driver_type: DriverType,
    pub license: DriverLicense,
    pub vendor: String,
    pub device_id: String,
    pub is_installed: bool,
    pub is_active: bool,
    pub is_recommended: bool,
    pub modalias: Option<String>,
}

impl DriverInfo {
    pub fn new(
        name: String,
        description: String,
        package_name: String,
        version: String,
        driver_type: DriverType,
        license: DriverLicense,
        vendor: String,
        device_id: String,
    ) -> Self {
        Self {
            name,
            description,
            package_name,
            version,
            driver_type,
            license,
            vendor,
            device_id,
            is_installed: false,
            is_active: false,
            is_recommended: false,
            modalias: None,
        }
    }

    pub fn get_type_icon(&self) -> &'static str {
        match self.driver_type {
            DriverType::Graphics => "🎮",
            DriverType::Network => "🌐",
            DriverType::Audio => "🔊",
            DriverType::Bluetooth => "📡",
            DriverType::Chipset => "🔧",
            DriverType::Storage => "💾",
            DriverType::Input => "⌨️",
            DriverType::Other => "🔌",
        }
    }

    pub fn get_license_icon(&self) -> &'static str {
        match self.license {
            DriverLicense::Free => "🆓",
            DriverLicense::NonFree => "💰",
            DriverLicense::Unknown => "❓",
        }
    }

    pub fn get_status_icon(&self) -> &'static str {
        if self.is_active {
            "🟢"
        } else if self.is_installed {
            "🔵"
        } else {
            "⚪"
        }
    }
}

/// Hardware detection ve modalias parsing
pub fn detect_hardware() -> Result<Vec<String>> {
    let mut modaliases = Vec::new();
    
    // PCI devices
    if let Ok(entries) = fs::read_dir("/sys/bus/pci/devices") {
        for entry in entries.flatten() {
            let modalias_path = entry.path().join("modalias");
            if let Ok(modalias) = fs::read_to_string(modalias_path) {
                modaliases.push(modalias.trim().to_string());
            }
        }
    }
    
    // USB devices
    if let Ok(entries) = fs::read_dir("/sys/bus/usb/devices") {
        for entry in entries.flatten() {
            let modalias_path = entry.path().join("modalias");
            if let Ok(modalias) = fs::read_to_string(modalias_path) {
                modaliases.push(modalias.trim().to_string());
            }
        }
    }
    
    Ok(modaliases)
}

pub fn parse_lspci_output() -> Result<Vec<(String, String, String)>> {
    let output = Command::new("lspci")
        .arg("-nn")
        .output()
        .context("lspci command could not be executed")?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();
    
    println!("🔍 Analyzing lspci output...");
    
    for line in stdout.lines() {
        if let Some(device_id_start) = line.rfind('[') {
            if let Some(device_id_end) = line.rfind(']') {
                if device_id_start < device_id_end {
                    let device_id = &line[device_id_start + 1..device_id_end];
                    let remaining_line = &line[..device_id_start].trim();
                    
                    if let Some((bus_info, full_description)) = remaining_line.split_once(": ") {
                        let device_tuple = (
                            bus_info.to_string(),
                            full_description.to_string(), 
                            device_id.to_string()
                        );
                        
                        let desc_lower = full_description.to_lowercase();
                        if desc_lower.contains("vga") || desc_lower.contains("3d controller") || 
                           desc_lower.contains("display controller") || desc_lower.contains("graphics") {
                            println!("🎮 Graphics device found: {} - {}", full_description, device_id);
                        }
                        
                        devices.push(device_tuple);
                    }
                }
            }
        }
    }
    
    println!("✅ {} hardware device parsed", devices.len());
    Ok(devices)
}

pub fn get_installed_drivers() -> Result<HashMap<String, String>> {
    let output = Command::new("dpkg")
        .args(&["-l"])
        .output()
        .context("dpkg command failed to run")?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut installed = HashMap::new();
    
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[0] == "ii" {
            let package_name = parts[1];
            let version = parts[2];
            
            if is_driver_package(package_name) {
                installed.insert(package_name.to_string(), version.to_string());
            }
        }
    }
    
    Ok(installed)
}

fn is_driver_package(package_name: &str) -> bool {
    let driver_patterns = [
        "nvidia", "amdgpu", "intel", "radeon", "nouveau",
        "broadcom", "realtek", "atheros", "iwlwifi",
        "alsa", "pulseaudio", "bluetooth",
        "firmware", "microcode",
        "dkms", "driver"
    ];
    
    let package_lower = package_name.to_lowercase();
    driver_patterns.iter().any(|pattern| package_lower.contains(pattern))
}

pub fn create_driver_backup() -> Result<String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let backup_dir = format!("/tmp/meaupdater_driver_backup_{}", timestamp);
    fs::create_dir_all(&backup_dir)?;
    
    let installed_output = Command::new("dpkg")
        .args(&["-l"])
        .output()
        .context("Could not get list of installed packages")?;
    
    fs::write(
        format!("{}/installed_packages.txt", backup_dir),
        &installed_output.stdout
    )?;
    
    let modules_output = Command::new("lsmod")
        .output()
        .context("Could not get active modules list")?;
    
    fs::write(
        format!("{}/active_modules.txt", backup_dir),
        &modules_output.stdout
    )?;
    
    let hardware_output = Command::new("lspci")
        .arg("-nn")
        .output()
        .context("Could not get hardware list")?;
    
    fs::write(
        format!("{}/hardware_info.txt", backup_dir),
        &hardware_output.stdout
    )?;
    
    println!("✅ Driver backup created: {}", backup_dir);
    Ok(backup_dir)
}

pub fn detect_drivers() -> Result<Vec<DriverInfo>> {
    println!("🔍 Starting driver detection...");
    let mut drivers = Vec::new();
    
    println!("📦 Checking installed packages...");
    let installed_packages = get_installed_drivers()?;
    println!("✅ {} Installed driver package found", installed_packages.len());
    
    println!("🖥️ Checking hardware devices...");
    let hardware_devices = parse_lspci_output()?;
    println!("✅ {} hardware device found", hardware_devices.len());
    
    
    let nvidia_devices = detect_nvidia_hardware(&hardware_devices)?;
    if !nvidia_devices.is_empty() {
        println!("🎮 {} NVIDIA device detected", nvidia_devices.len());
        for (device_name, device_id) in nvidia_devices {
            println!("  - {}: {}", device_name, device_id);
            drivers.extend(detect_nvidia_drivers(&installed_packages, &device_name, &device_id)?);
        }
    }
    
    let amd_devices = detect_amd_hardware(&hardware_devices)?;
    if !amd_devices.is_empty() {
        println!("🎮 {} AMD device detected", amd_devices.len());
        for (device_name, device_id) in amd_devices {
            println!("  - {}: {}", device_name, device_id);
            drivers.extend(detect_amd_drivers(&installed_packages, &device_name, &device_id)?);
        }
    }
    
    let intel_devices = detect_intel_hardware(&hardware_devices)?;
    if !intel_devices.is_empty() {
        println!("🎮 {} Intel device detected", intel_devices.len());
        for (device_name, device_id) in intel_devices {
            println!("  - {}: {}", device_name, device_id);
            drivers.extend(detect_intel_drivers(&installed_packages, &device_name, &device_id)?);
        }
    }
    
    let network_drivers = detect_network_drivers(&installed_packages, &hardware_devices)?;
    if !network_drivers.is_empty() {
        println!("🌐 {} network driver found", network_drivers.len());
        drivers.extend(network_drivers);
    }
    
    if has_audio_hardware(&hardware_devices) {
        println!("🔊 Checking audio drivers...");
        drivers.extend(detect_audio_drivers(&installed_packages)?);
    }
    
    if has_bluetooth_hardware() {
        println!("📡 Checking Bluetooth drivers...");
        drivers.extend(detect_bluetooth_drivers(&installed_packages)?);
    }
    
    let firmware_drivers = detect_firmware_packages(&installed_packages, &hardware_devices)?;
    if !firmware_drivers.is_empty() {
        println!("💾 {} firmware package found", firmware_drivers.len());
        drivers.extend(firmware_drivers);
    }
    
    println!("✅ Total {} drivers found", drivers.len());
    Ok(drivers)
}

fn detect_nvidia_hardware(devices: &[(String, String, String)]) -> Result<Vec<(String, String)>> {
    let mut nvidia_devices = Vec::new();
    
    println!("🔍 Starting NVIDIA detection... Checking {} devices", devices.len());
    
    for (bus_info, desc, device_id) in devices {
        let desc_lower = desc.to_lowercase();
        
        if desc_lower.contains("nvidia") {
            println!("🎯 NVIDIA device: {} [{}]", desc, device_id);
        }
        
        if desc_lower.contains("nvidia") {
            if desc_lower.contains("audio") || 
               desc_lower.contains("high definition audio") ||
               desc_lower.contains("usb") ||
               desc_lower.contains("serial bus") ||
               desc_lower.contains("hdmi") {
                println!("❌ NVIDIA Audio/USB device skipped: {}", desc);
                continue;
            }
            
            if desc_lower.contains("vga") || 
               desc_lower.contains("3d") || 
               desc_lower.contains("display") || 
               desc_lower.contains("graphics") ||
               desc_lower.contains("geforce") ||
               desc_lower.contains("quadro") ||
               desc_lower.contains("tesla") {
                
                println!("✅ NVIDIA Graphics Card Detected: {} [{}]", desc, device_id);
                nvidia_devices.push((desc.clone(), device_id.clone()));
            } else {
                println!("❓ NVIDIA device but not graphics card: {}", desc);
            }
        }
    }
    
    if nvidia_devices.is_empty() {
        println!("❌ No NVIDIA graphics card found");
    } else {
        println!("🎮 Total {} NVIDIA graphics cards found", nvidia_devices.len());
    }
    
    Ok(nvidia_devices)
}

fn detect_amd_hardware(devices: &[(String, String, String)]) -> Result<Vec<(String, String)>> {
    let mut amd_devices = Vec::new();
    
    for (_, desc, device_id) in devices {
        let desc_lower = desc.to_lowercase();
        if (desc_lower.contains("amd") || desc_lower.contains("radeon") || 
            desc_lower.contains("ati") || desc_lower.contains("advanced micro devices")) && 
           (desc_lower.contains("vga") || desc_lower.contains("vga compatible") || 
            desc_lower.contains("3d controller") || desc_lower.contains("display controller") || 
            desc_lower.contains("graphics")) &&
           !desc_lower.contains("audio") && !desc_lower.contains("hdmi") &&
           !desc_lower.contains("nvidia") && !desc_lower.contains("sound") {
            
            println!("🎮 AMD Graphics Card Found: {} [{}]", desc, device_id);
            amd_devices.push((desc.clone(), device_id.clone()));
        }
    }
    
    Ok(amd_devices)
}

fn detect_intel_hardware(devices: &[(String, String, String)]) -> Result<Vec<(String, String)>> {
    let mut intel_devices = Vec::new();
    
    for (_, desc, device_id) in devices {
        let desc_lower = desc.to_lowercase();
        if desc_lower.contains("intel") && 
           (desc_lower.contains("vga") || desc_lower.contains("vga compatible") || 
            desc_lower.contains("3d controller") || desc_lower.contains("display controller") || 
            desc_lower.contains("graphics") || desc_lower.contains("integrated graphics")) &&
           !desc_lower.contains("audio") && !desc_lower.contains("hdmi") && 
           !desc_lower.contains("sound") {
            
            println!("🎮 Intel Graphics Card Found: {} [{}]", desc, device_id);
            intel_devices.push((desc.clone(), device_id.clone()));
        }
    }
    
    let intel_cpu = Command::new("lscpu")
        .output()
        .map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.lines().any(|line| line.to_lowercase().contains("intel"))
        })
        .unwrap_or(false);
    
    if intel_cpu {
        println!("🔧 Intel CPU Found (For Microcode)");
        intel_devices.push(("Intel CPU (Microcode)".to_string(), "CPU".to_string()));
    }
    
    Ok(intel_devices)
}

fn has_audio_hardware(devices: &[(String, String, String)]) -> bool {
    devices.iter().any(|(_, desc, _)| {
        let desc_lower = desc.to_lowercase();
        (desc_lower.contains("audio") || desc_lower.contains("sound")) &&
        !desc_lower.contains("hdmi") 
    }) ||
    std::path::Path::new("/proc/asound").exists()
}

fn has_bluetooth_hardware() -> bool {
    let pci_bluetooth = Command::new("lspci")
        .output()
        .map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.lines().any(|line| {
                line.to_lowercase().contains("bluetooth")
            })
        })
        .unwrap_or(false);
    
    let usb_bluetooth = Command::new("lsusb")
        .output()
        .map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.lines().any(|line| {
                line.to_lowercase().contains("bluetooth")
            })
        })
        .unwrap_or(false);
    
    let sys_bluetooth = std::path::Path::new("/sys/class/bluetooth").exists();
    
    let rfkill_bluetooth = Command::new("rfkill")
        .arg("list")
        .output()
        .map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.lines().any(|line| {
                line.to_lowercase().contains("bluetooth")
            })
        })
        .unwrap_or(false);
    
    pci_bluetooth || usb_bluetooth || sys_bluetooth || rfkill_bluetooth
}

fn detect_available_nvidia_packages() -> Result<Vec<String>> {
    let output = Command::new("apt")
        .args(&["search", "nvidia-driver"])
        .output()
        .context("Could not run apt search nvidia-driver command")?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut packages = Vec::new();
    
    for line in stdout.lines() {
        if !line.contains("WARNING") && line.contains('/') {
            if let Some(package_name) = line.split('/').next() {
                let clean_name = package_name.trim();
                if clean_name == "nvidia-driver" || 
                   clean_name == "nvidia-driver-full" ||
                   clean_name == "xserver-xorg-video-nvidia" {
                    packages.push(clean_name.to_string());
                }
            }
        }
    }
    
    packages.push("xserver-xorg-video-nouveau".to_string());
    
    packages.sort();
    packages.dedup();
    
    println!("🔍 Debian Trixie NVIDIA packages: {:?}", packages);
    Ok(packages)
}

fn detect_nvidia_drivers(installed: &HashMap<String, String>, device_name: &str, device_id: &str) -> Result<Vec<DriverInfo>> {
    let mut drivers = Vec::new();
    
    let debian_nvidia_packages = vec![
        ("nvidia-driver", "NVIDIA Driver (Metapackage)", DriverLicense::NonFree),
        ("nvidia-driver-full", "NVIDIA Full Driver Suite", DriverLicense::NonFree),
        ("xserver-xorg-video-nvidia", "NVIDIA Xorg Driver", DriverLicense::NonFree),
        ("xserver-xorg-video-nouveau", "Nouveau (Open Source)", DriverLicense::Free),
    ];
    
    for (package, desc, license) in debian_nvidia_packages {
        let package_installed = installed.contains_key(package) || 
            installed.iter().any(|(k, _)| k.contains(package) || k.starts_with(package));
        
        let is_active = if package.contains("nouveau") {
            Command::new("lsmod")
                .output()
                .map(|output| String::from_utf8_lossy(&output.stdout).contains("nouveau"))
                .unwrap_or(false)
        } else {
            Command::new("lsmod")
                .output()
                .map(|output| {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    stdout.contains("nvidia") && !stdout.contains("nouveau")
                })
                .unwrap_or(false) ||
            Command::new("nvidia-smi")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        };
        
        let package_available = Command::new("apt")
            .args(&["show", package])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        
        if !package_available {
            println!("⚠️ Package not available, skipping: {}", package);
            continue;
        }
        
        let is_truly_installed = package_installed || is_active;
        
        let version = if is_truly_installed {
            installed.get(package).unwrap_or(&"Active".to_string()).clone()
        } else {
            "Available".to_string()
        };
        
        let mut driver = DriverInfo::new(
            package.to_string(),
            format!("{} - {}", desc, device_name),
            package.to_string(),
            version,
            DriverType::Graphics,
            license.clone(),
            "NVIDIA".to_string(),
            device_id.to_string(),
        );
        
        driver.is_installed = is_truly_installed;
        driver.is_active = is_active;
        driver.is_recommended = package == "nvidia-driver";
        drivers.push(driver);
        
        println!("🎮 NVIDIA package: {} - Installed: {}, Active: {}, Available: {}", 
                package, package_installed, is_active, package_available);
    }
    
    Ok(drivers)
}

fn detect_amd_drivers(installed: &HashMap<String, String>, device_name: &str, device_id: &str) -> Result<Vec<DriverInfo>> {
    let mut drivers = Vec::new();
    
    let amd_packages = [
        ("amdgpu", "AMDGPU (Open Source)", DriverLicense::Free),
        ("radeon", "Radeon (Legacy Open Source)", DriverLicense::Free),
    ];
    
    for (package, desc, license) in &amd_packages {
        let is_active = Command::new("lsmod")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(package))
            .unwrap_or(false);
        
        let package_exists = installed.iter().any(|(k, _)| k.contains(package)) ||
            installed.iter().any(|(k, _)| k.contains(&format!("xserver-xorg-video-{}", package))) ||
            is_active; 
        
        let is_truly_installed = package_exists && is_active;
        
        let version = if is_truly_installed {
            "Built-in".to_string()
        } else {
            "Available".to_string()
        };
        
        let mut driver = DriverInfo::new(
            package.to_string(),
            format!("{} - {}", desc, device_name),
            format!("xserver-xorg-video-{}", package),
            version,
            DriverType::Graphics,
            license.clone(),
            "AMD".to_string(),
            device_id.to_string(),
        );
        
        driver.is_installed = is_truly_installed;
        driver.is_active = is_active;
        driver.is_recommended = *package == "amdgpu";
        drivers.push(driver);
    }
    
    Ok(drivers)
}

fn detect_intel_drivers(installed: &HashMap<String, String>, device_name: &str, device_id: &str) -> Result<Vec<DriverInfo>> {
    let mut drivers = Vec::new();
    
    if device_id == "CPU" {
        let is_microcode_installed = installed.contains_key("intel-microcode");
        let is_microcode_active = std::path::Path::new("/sys/devices/system/cpu/microcode").exists();
        let is_truly_installed = is_microcode_installed && is_microcode_active;
        
        let version = if is_truly_installed {
            installed.get("intel-microcode").unwrap_or(&"Active".to_string()).clone()
        } else {
            "Available".to_string()
        };
        
        let mut microcode_driver = DriverInfo::new(
            "intel-microcode".to_string(),
            "Intel Microcode Updates".to_string(),
            "intel-microcode".to_string(),
            version,
            DriverType::Chipset,
            DriverLicense::NonFree,
            "Intel".to_string(),
            device_id.to_string(),
        );
        
        microcode_driver.is_installed = is_truly_installed;
        microcode_driver.is_active = is_microcode_active;
        microcode_driver.is_recommended = true;
        drivers.push(microcode_driver);
    } else {
        let is_active = Command::new("lsmod")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains("i915"))
            .unwrap_or(false);
        
        let mut driver = DriverInfo::new(
            "intel".to_string(),
            format!("Intel Graphics (Built-in) - {}", device_name),
            "xserver-xorg-video-intel".to_string(),
            if is_active { "Built-in" } else { "Available" }.to_string(),
            DriverType::Graphics,
            DriverLicense::Free,
            "Intel".to_string(),
            device_id.to_string(),
        );
        
        driver.is_installed = is_active; 
        driver.is_active = is_active;
        driver.is_recommended = true;
        drivers.push(driver);
    }
    
    Ok(drivers)
}

fn detect_network_drivers(installed: &HashMap<String, String>, devices: &[(String, String, String)]) -> Result<Vec<DriverInfo>> {
    let mut drivers = Vec::new();
    
    for (_, desc, device_id) in devices {
        let desc_lower = desc.to_lowercase();
        
        if desc_lower.contains("realtek") && desc_lower.contains("ethernet") {
            println!("✅ Realtek Ethernet card detected: {}", desc);
            
            let is_active = Command::new("lsmod").output()
                .map(|output| {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    stdout.contains("r8169") || stdout.contains("r8168")
                })
                .unwrap_or(false);
            
            let mut driver = DriverInfo::new(
                "r8168-dkms".to_string(),
                format!("Realtek Ethernet Driver - {}", desc),
                "r8168-dkms".to_string(),
                if is_active { "Built-in" } else { "Available" }.to_string(),
                DriverType::Network,
                DriverLicense::Free,
                "Realtek".to_string(),
                device_id.clone(),
            );
            
            driver.is_installed = is_active;
            driver.is_active = is_active;
            driver.is_recommended = true;
            drivers.push(driver);
        }
        
        if desc_lower.contains("realtek") && 
           (desc_lower.contains("wireless") || desc_lower.contains("wi-fi") || 
            desc_lower.contains("802.11") || desc_lower.contains("wlan")) {
            println!("✅ Realtek WiFi card detected: {}", desc);
            
            let realtek_wifi_packages = [
                ("rtl8192eu-dkms", "Realtek RTL8192EU WiFi"),
                ("rtl8821ce-dkms", "Realtek RTL8821CE WiFi"),
            ];
            
            for (package, package_desc) in &realtek_wifi_packages {
                let package_installed = installed.contains_key(*package);
                let is_active = Command::new("lsmod").output()
                    .map(|output| {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if package.contains("rtl8192eu") {
                            stdout.contains("rtl8192eu")
                        } else if package.contains("rtl8821ce") {
                            stdout.contains("rtl8821ce")
                        } else {
                            false
                        }
                    })
                    .unwrap_or(false);
                
                let is_truly_installed = package_installed && is_active;
                
                let mut driver = DriverInfo::new(
                    package.to_string(),
                    format!("{} - {}", package_desc, desc),
                    package.to_string(),
                    if is_truly_installed { 
                        installed.get(*package).unwrap_or(&"Active".to_string()).clone() 
                    } else { 
                        "Available".to_string() 
                    },
                    DriverType::Network,
                    DriverLicense::Free,
                    "Realtek".to_string(),
                    device_id.clone(),
                );
                
                driver.is_installed = is_truly_installed;
                driver.is_active = is_active;
                drivers.push(driver);
            }
        }
    }
    
    let has_broadcom_wifi = devices.iter().any(|(_, desc, _)| {
        let desc_lower = desc.to_lowercase();
        desc_lower.contains("broadcom") && 
        (desc_lower.contains("wireless") || desc_lower.contains("wi-fi") || 
         desc_lower.contains("802.11") || desc_lower.contains("wlan"))
    });
    
    if has_broadcom_wifi {
        println!("✅ Broadcom WiFi card detected");
        let broadcom_packages = [
            ("broadcom-sta-dkms", "Broadcom STA (Proprietary)", DriverLicense::NonFree),
            ("b43-fwcutter", "B43 Firmware Cutter (Open Source)", DriverLicense::Free),
        ];
        
        for (package, desc, license) in &broadcom_packages {
            let package_installed = installed.contains_key(*package);
            let is_active = if *package == "broadcom-sta-dkms" {
                Command::new("lsmod").output()
                    .map(|output| String::from_utf8_lossy(&output.stdout).contains("wl"))
                    .unwrap_or(false)
            } else {
                Command::new("lsmod").output()
                    .map(|output| String::from_utf8_lossy(&output.stdout).contains("b43"))
                    .unwrap_or(false)
            };
            
            let is_truly_installed = package_installed && is_active;
            
            let mut driver = DriverInfo::new(
                package.to_string(),
                desc.to_string(),
                package.to_string(),
                if is_truly_installed {
                    installed.get(*package).unwrap_or(&"Active".to_string()).clone()
                } else {
                    "Available".to_string()
                },
                DriverType::Network,
                license.clone(),
                "Broadcom".to_string(),
                "".to_string(),
            );
            
            driver.is_installed = is_truly_installed;
            driver.is_active = is_active;
            driver.is_recommended = *package == "broadcom-sta-dkms";
            drivers.push(driver);
        }
    }
    
    let has_intel_wifi = devices.iter().any(|(_, desc, _)| {
        let desc_lower = desc.to_lowercase();
        desc_lower.contains("intel") && 
        (desc_lower.contains("wireless") || desc_lower.contains("wi-fi") || 
         desc_lower.contains("802.11") || desc_lower.contains("centrino"))
    });
    
    if has_intel_wifi {
        println!("✅ Intel WiFi card detected");
        let is_active = Command::new("lsmod").output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains("iwlwifi"))
            .unwrap_or(false);
        
        let mut driver = DriverInfo::new(
            "iwlwifi".to_string(),
            "Intel WiFi Driver".to_string(),
            "iwlwifi".to_string(),
            if is_active { "Built-in" } else { "Available" }.to_string(),
            DriverType::Network,
            DriverLicense::Free,
            "Intel".to_string(),
            "".to_string(),
        );
        
        driver.is_installed = is_active;
        driver.is_active = is_active;
        driver.is_recommended = true;
        drivers.push(driver);
    }
    
    Ok(drivers)
}

fn detect_audio_drivers(installed: &HashMap<String, String>) -> Result<Vec<DriverInfo>> {
    let mut drivers = Vec::new();
    
    let audio_packages = [
        ("alsa-base", "ALSA Sound System", DriverLicense::Free),
        ("pulseaudio", "PulseAudio", DriverLicense::Free),
        ("pipewire", "PipeWire", DriverLicense::Free),
    ];
    
    for (package, desc, license) in &audio_packages {
        let package_installed = installed.contains_key(*package) ||
            installed.iter().any(|(k, _)| k.starts_with(package) || k.contains(&format!("{}-", package)));
        
        let is_active = if *package == "pulseaudio" {
            // 2025 mthakan
            Command::new("pgrep")
                .arg("-x") 
                .arg("pulseaudio")
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else if *package == "pipewire" {
            Command::new("pgrep")
                .arg("-x")   
                .arg("pipewire")
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else if *package == "alsa-base" {

            std::path::Path::new("/proc/asound").exists()
        } else {
            false
        };
        

        let is_truly_installed = package_installed || is_active;
        

        let consistent_active = if package_installed { is_active } else { false };
        
        println!("🔊 {} - Package: {}, Active: {}, Installed: {}, Consistent Active: {}", 
                package, package_installed, is_active, is_truly_installed, consistent_active);
        
        let version = if is_truly_installed {
            installed.get(*package).unwrap_or(&"Active".to_string()).clone()
        } else {
            "Available".to_string()
        };
        
        let mut driver = DriverInfo::new(
            package.to_string(),
            desc.to_string(),
            package.to_string(),
            version,
            DriverType::Audio,
            license.clone(),
            "Generic".to_string(),
            "".to_string(),
        );
        
        driver.is_installed = is_truly_installed; 
        driver.is_active = consistent_active;     
        driver.is_recommended = *package == "pulseaudio";
        drivers.push(driver);
    }
    
    Ok(drivers)
}


fn detect_bluetooth_drivers(installed: &HashMap<String, String>) -> Result<Vec<DriverInfo>> {
    let mut drivers = Vec::new();
    
    println!("✅ Bluetooth device detected");
    
    let bluetooth_packages = [
        ("bluez", "BlueZ Bluetooth Stack", DriverLicense::Free),
        ("bluetooth", "Bluetooth Support", DriverLicense::Free),
    ];
    
    for (package, desc, license) in &bluetooth_packages {
        let package_installed = installed.contains_key(*package);
        

        let is_active = Command::new("systemctl")
            .args(&["is-active", "bluetooth"])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "active")
            .unwrap_or(false);
        
        let is_truly_installed = package_installed && is_active;
        
        let version = if is_truly_installed {
            installed.get(*package).unwrap_or(&"Active".to_string()).clone()
        } else {
            "Available".to_string()
        };
        
        let mut driver = DriverInfo::new(
            package.to_string(),
            desc.to_string(),
            package.to_string(),
            version,
            DriverType::Bluetooth,
            license.clone(),
            "Generic".to_string(),
            "".to_string(),
        );
        
        driver.is_installed = is_truly_installed;
        driver.is_active = is_active;
        driver.is_recommended = *package == "bluez";
        drivers.push(driver);
    }
    
    Ok(drivers)
}


fn detect_firmware_packages(installed: &HashMap<String, String>, devices: &[(String, String, String)]) -> Result<Vec<DriverInfo>> {
    let mut drivers = Vec::new();
    

    let has_realtek_needs_firmware = devices.iter().any(|(_, desc, _)| {
        let desc_lower = desc.to_lowercase();
        desc_lower.contains("realtek") && 
        (desc_lower.contains("wireless") || desc_lower.contains("wi-fi") || 
         desc_lower.contains("802.11") || desc_lower.contains("wlan"))
    });
    
    if has_realtek_needs_firmware {
        let package_installed = installed.contains_key("firmware-realtek");
        
        let mut driver = DriverInfo::new(
            "firmware-realtek".to_string(),
            "Realtek WiFi Firmware".to_string(),
            "firmware-realtek".to_string(),
            if package_installed {
                installed.get("firmware-realtek").unwrap_or(&"Active".to_string()).clone()
            } else {
                "Available".to_string()
            },
            DriverType::Other,
            DriverLicense::NonFree,
            "Realtek".to_string(),
            "".to_string(),
        );
        driver.is_installed = package_installed;
        driver.is_active = package_installed; 
        driver.is_recommended = true;
        drivers.push(driver);
    }
    

    let has_intel_wifi = devices.iter().any(|(_, desc, _)| {
        let desc_lower = desc.to_lowercase();
        desc_lower.contains("intel") && 
        (desc_lower.contains("wireless") || desc_lower.contains("wi-fi") || 
         desc_lower.contains("802.11") || desc_lower.contains("centrino"))
    });
    
    if has_intel_wifi {
        let package_installed = installed.contains_key("firmware-iwlwifi");
        
        let mut driver = DriverInfo::new(
            "firmware-iwlwifi".to_string(),
            "Intel WiFi Firmware".to_string(),
            "firmware-iwlwifi".to_string(),
            if package_installed {
                installed.get("firmware-iwlwifi").unwrap_or(&"Active".to_string()).clone()
            } else {
                "Available".to_string()
            },
            DriverType::Other,
            DriverLicense::NonFree,
            "Intel".to_string(),
            "".to_string(),
        );
        driver.is_installed = package_installed;
        driver.is_active = package_installed;
        driver.is_recommended = true;
        drivers.push(driver);
    }
    

    let has_atheros = devices.iter().any(|(_, desc, _)| {
        let desc_lower = desc.to_lowercase();
        desc_lower.contains("atheros") || 
        (desc_lower.contains("qualcomm") && desc_lower.contains("atheros"))
    });
    
    if has_atheros {
        let package_installed = installed.contains_key("firmware-atheros");
        
        let mut driver = DriverInfo::new(
            "firmware-atheros".to_string(),
            "Atheros Firmware".to_string(),
            "firmware-atheros".to_string(),
            if package_installed {
                installed.get("firmware-atheros").unwrap_or(&"Active".to_string()).clone()
            } else {
                "Available".to_string()
            },
            DriverType::Other,
            DriverLicense::NonFree,
            "Atheros".to_string(),
            "".to_string(),
        );
        driver.is_installed = package_installed;
        driver.is_active = package_installed;
        driver.is_recommended = true;
        drivers.push(driver);
    }
    

    let general_firmware = [
        ("firmware-linux", "Linux Firmware (Free)", DriverLicense::Free),
        ("firmware-linux-nonfree", "Linux Firmware (Non-free)", DriverLicense::NonFree),
    ];
    
    for (package, desc, license) in &general_firmware {
        let package_installed = installed.contains_key(*package);
        
        let mut driver = DriverInfo::new(
            package.to_string(),
            desc.to_string(),
            package.to_string(),
            if package_installed {
                installed.get(*package).unwrap_or(&"Active".to_string()).clone()
            } else {
                "Available".to_string()
            },
            DriverType::Other,
            license.clone(),
            "Various".to_string(),
            "".to_string(),
        );
        driver.is_installed = package_installed;
        driver.is_active = package_installed;
        driver.is_recommended = *package == "firmware-linux-nonfree";
        drivers.push(driver);
    }
    
    Ok(drivers)
}


pub fn detect_active_drivers() -> Result<Vec<String>> {
    let output = Command::new("lsmod")
        .output()
        .context("lsmod command could not be executed")?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut active_modules = Vec::new();
    
    for line in stdout.lines().skip(1) { 
        if let Some(module_name) = line.split_whitespace().next() {
            active_modules.push(module_name.to_string());
        }
    }
    
    Ok(active_modules)
}


pub fn install_driver(package_name: &str) -> Result<()> {

    if package_name.contains("nvidia") {
        println!("🔍 Checking non-free repository for NVIDIA driver...");
        
        let sources_check = Command::new("grep")
            .args(&["-r", "non-free", "/etc/apt/sources.list", "/etc/apt/sources.list.d/"])
            .output();
        
        let has_nonfree = sources_check
            .map(|output| !output.stdout.is_empty())
            .unwrap_or(false);
        
        if !has_nonfree {
            println!("❌ Non-free repository is not active!");
            bail!("Non-free repository required for NVIDIA drivers.\n\nTo enable:\nsudo apt edit-sources\n\nAnd add 'non-free' at the end of the line.");
        }
        
        println!("✅ Non-free repository active");
    }
    

    println!("🔍 Checking package availability: {}", package_name);
    let search_result = Command::new("apt")
        .args(&["search", package_name])
        .output()
        .context("Could not run apt search command")?;
    
    let search_output = String::from_utf8_lossy(&search_result.stdout);
    if !search_output.contains(package_name) {
        bail!("Package not found: {}\n\nCheck available packages:\napt search nvidia-driver", package_name);
    }
    
    println!("✅ Package found: {}", package_name);
    

    println!("💾 Creating driver backup...");
    let _backup_dir = create_driver_backup()?;
    

    println!("🔄 Updating package database...");
    let update_status = Command::new("pkexec")
        .args(&["apt", "update"])
        .status()
        .context("apt update command could not be executed")?;
    
    if !update_status.success() {
        println!("⚠️ apt update failed, continuing...");
    }
    

    println!("📦 Driver is being installed: {}", package_name);
    let status = Command::new("pkexec")
        .args(&["apt", "install", "-y", package_name])
        .status()
        .context("Driver install command failed to execute")?;
    
    if !status.success() {
        bail!("Driver installation failed. Check the package name or repository settings.");
    }
    
    println!("✅ Driver successfully installed: {}", package_name);
    Ok(())
}


pub fn remove_driver(package_name: &str) -> Result<()> {

    let _backup_dir = create_driver_backup()?;
    
    let status = Command::new("pkexec")
        .args(&["apt", "remove", "--purge", "-y", package_name])
        .status()
        .context("Uninstall driver command failed to execute")?;
    
    if !status.success() {
        bail!("Driver uninstall failed");
    }
    
    Ok(())
}


pub fn group_drivers_by_type(drivers: Vec<DriverInfo>) -> HashMap<DriverType, Vec<DriverInfo>> {
    let mut groups: HashMap<DriverType, Vec<DriverInfo>> = HashMap::new();
    
    for driver in drivers {
        groups.entry(driver.driver_type.clone()).or_insert_with(Vec::new).push(driver);
    }
    

    for group in groups.values_mut() {
        group.sort_by(|a, b| a.name.cmp(&b.name));
    }
    
    groups
}


pub fn filter_drivers(drivers: &[DriverInfo], type_filter: &str, license_filter: &str, status_filter: &str) -> Vec<DriverInfo> {
    drivers.iter()
        .filter(|driver| {

            let type_match = match type_filter {
                "graphics" => matches!(driver.driver_type, DriverType::Graphics),
                "network" => matches!(driver.driver_type, DriverType::Network),
                "audio" => matches!(driver.driver_type, DriverType::Audio),
                "bluetooth" => matches!(driver.driver_type, DriverType::Bluetooth),
                "chipset" => matches!(driver.driver_type, DriverType::Chipset),
                "other" => matches!(driver.driver_type, DriverType::Other),
                _ => true, 
            };
            

            let license_match = match license_filter {
                "free" => matches!(driver.license, DriverLicense::Free),
                "nonfree" => matches!(driver.license, DriverLicense::NonFree),
                _ => true, 
            };
            

            let status_match = match status_filter {
                "installed" => driver.is_installed,
                "active" => driver.is_active,
                "available" => !driver.is_installed,
                _ => true, 
            };
            
            type_match && license_match && status_match
        })
        .cloned()
        .collect()
}


pub fn rescan_hardware() -> Result<()> {
    println!("🔍 Rescanning hardware...");
    

    let _ = Command::new("udevadm")
        .args(&["trigger", "--subsystem-match=usb"])
        .output();
    

    let _ = Command::new("udevadm")
        .args(&["trigger", "--subsystem-match=pci"])
        .output();
    

    let _ = Command::new("udevadm")
        .arg("settle")
        .output();
    
    println!("✅ Hardware rescan completed");
    Ok(())
}


impl DriverType {
    pub fn display_name(&self) -> &'static str {
        match self {
            DriverType::Graphics => "GPU",
            DriverType::Network => "Network Cards",
            DriverType::Audio => "Audio Cards",
            DriverType::Bluetooth => "Bluetooth",
            DriverType::Chipset => "Chipset",
            DriverType::Storage => "Storage",
            DriverType::Input => "Input Devices",
            DriverType::Other => "Other",
        }
    }
}

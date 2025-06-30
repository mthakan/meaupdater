// src/repo_manager.rs

use anyhow::{Context, Result, bail};
use std::fs;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Repository {
    pub name: String,
    pub uri: String,
    pub distribution: String,
    pub components: String,
    pub enabled: bool,
    pub is_source: bool,
    pub file_path: Option<String>, // To keep track of which file it came from
    pub line_number: Option<usize>, // To keep track of which line it is on
}

impl Repository {
    pub fn to_sources_list_line(&self) -> String {
        let repo_type = if self.is_source { "deb-src" } else { "deb" };
        let comment = if self.enabled { "" } else { "# " };
        format!("{}{} {} {} {}", comment, repo_type, self.uri, self.distribution, self.components)
    }

    pub fn from_sources_list_line(line: &str) -> Option<Repository> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') && !line.contains("deb") {
            return None;
        }

        let enabled = !line.starts_with('#');
        let clean_line = if line.starts_with('#') {
            line.trim_start_matches('#').trim()
        } else {
            line
        };

        let parts: Vec<&str> = clean_line.split_whitespace().collect();
        if parts.len() < 4 {
            return None;
        }

        let is_source = parts[0] == "deb-src";
        let uri = parts[1].to_string();
        let distribution = parts[2].to_string();
        let components = parts[3..].join(" ");
        
        // Remove the repository name from the URI.
        let name = uri.split('/').last()
            .unwrap_or(&uri)
            .replace("http://", "")
            .replace("https://", "");

        Some(Repository {
            name,
            uri,
            distribution,
            components,
            enabled,
            is_source,
            file_path: None,
            line_number: None,
        })
    }
}

pub fn get_repositories() -> Result<Vec<Repository>> {
    let mut repositories = Vec::new();
    
    // Read the /etc/apt/sources.list file
    if let Ok(content) = fs::read_to_string("/etc/apt/sources.list") {
        for (line_num, line) in content.lines().enumerate() {
            if let Some(mut repo) = Repository::from_sources_list_line(line) {
                repo.file_path = Some("/etc/apt/sources.list".to_string());
                repo.line_number = Some(line_num);
                repositories.push(repo);
            }
        }
    }
    
    // Read the files in the /etc/apt/sources.list.d/ directory.
    if let Ok(entries) = fs::read_dir("/etc/apt/sources.list.d/") {
        for entry in entries.flatten() {
            if let Some(path) = entry.path().to_str() {
                if path.ends_with(".list") {
                    if let Ok(content) = fs::read_to_string(&entry.path()) {
                        for (line_num, line) in content.lines().enumerate() {
                            if let Some(mut repo) = Repository::from_sources_list_line(line) {
                                repo.file_path = Some(path.to_string());
                                repo.line_number = Some(line_num);
                                repositories.push(repo);
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(repositories)
}

pub fn add_repository(uri: &str, distribution: &str, components: &str) -> Result<()> {
    let repo = Repository {
        name: "Custom Repository".to_string(),
        uri: uri.to_string(),
        distribution: distribution.to_string(),
        components: components.to_string(),
        enabled: true,
        is_source: false,
        file_path: None,
        line_number: None,
    };
    
    let line = format!("{}\n", repo.to_sources_list_line());
    let temp_file = "/tmp/new_repo.list";
    fs::write(temp_file, line)?;
    
    let status = Command::new("pkexec")
        .arg("sh")
        .arg("-c")
        .arg(&format!("mv {} /etc/apt/sources.list.d/", temp_file))
        .status()
        .context("Repository could not be added")?;
        
    if !status.success() {
        bail!("Repository add operation failed");
    }
    
    Ok(())
}

pub fn remove_repository(repo: &Repository) -> Result<()> {
    if let Some(file_path) = &repo.file_path {
        let content = fs::read_to_string(file_path)?;
        let lines: Vec<&str> = content.lines().collect();
        
        if let Some(line_num) = repo.line_number {
            if line_num < lines.len() {
                let mut new_lines = lines.clone();
                new_lines.remove(line_num);
                let new_content = new_lines.join("\n");
                
                let temp_file = "/tmp/temp_sources.list";
                fs::write(temp_file, new_content)?;
                
                let status = Command::new("pkexec")
                    .arg("sh")
                    .arg("-c")
                    .arg(&format!("mv {} {}", temp_file, file_path))
                    .status()
                    .context("Repository could not be deleted")?;
                    
                if !status.success() {
                    bail!("Repository deletion failed");
                }
            }
        }
    }
    
    Ok(())
}

pub fn toggle_repository(repo: &Repository) -> Result<()> {
    if let Some(file_path) = &repo.file_path {
        let content = fs::read_to_string(file_path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        
        if let Some(line_num) = repo.line_number {
            if line_num < lines.len() {
                let line = &lines[line_num];
                
                // Toggle the row
                let new_line = if line.trim_start().starts_with('#') {
                    // Remove comment (enable)
                    line.trim_start_matches('#').trim_start().to_string()
                } else {
                    // Add comment (disable)
                    format!("# {}", line)
                };
                
                lines[line_num] = new_line;
                let new_content = lines.join("\n");
                
                let temp_file = "/tmp/temp_sources.list";
                fs::write(temp_file, new_content)?;
                
                let status = Command::new("pkexec")
                    .arg("sh")
                    .arg("-c")
                    .arg(&format!("mv {} {}", temp_file, file_path))
                    .status()
                    .context("Repository status could not be changed")?;
                    
                if !status.success() {
                    bail!("Repository state change operation failed");
                }
            }
        }
    }
    
    Ok(())
}

pub fn edit_repository(old_repo: &Repository, new_uri: &str, new_distribution: &str, new_components: &str) -> Result<()> {
    if let Some(file_path) = &old_repo.file_path {
        let content = fs::read_to_string(file_path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        
        if let Some(line_num) = old_repo.line_number {
            if line_num < lines.len() {
                let new_repo = Repository {
                    name: old_repo.name.clone(),
                    uri: new_uri.to_string(),
                    distribution: new_distribution.to_string(),
                    components: new_components.to_string(),
                    enabled: old_repo.enabled,
                    is_source: old_repo.is_source,
                    file_path: old_repo.file_path.clone(),
                    line_number: old_repo.line_number,
                };
                
                lines[line_num] = new_repo.to_sources_list_line();
                let new_content = lines.join("\n");
                
                let temp_file = "/tmp/temp_sources.list";
                fs::write(temp_file, new_content)?;
                
                let status = Command::new("pkexec")
                    .arg("sh")
                    .arg("-c")
                    .arg(&format!("mv {} {}", temp_file, file_path))
                    .status()
                    .context("Repository could not be edited")?;
                    
                if !status.success() {
                    bail!("Repository edit operation failed");
                }
            }
        }
    }
    
    Ok(())
}

pub fn update_repositories() -> Result<()> {
    let status = Command::new("pkexec")
        .arg("apt")
        .arg("update")
        .status()
        .context("apt update failed to run")?;
        
    if !status.success() {
        bail!("apt update failed");
    }
    
    Ok(())
}

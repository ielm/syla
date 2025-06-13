use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, Table};
use std::path::PathBuf;

use crate::config::Config;
use crate::git;
use crate::docker;

pub async fn run(detailed: bool, workspace_root: Option<PathBuf>) -> Result<()> {
    let config = Config::load(workspace_root)?;
    
    println!("{}", "Workspace Status".bold());
    println!("Root: {}\n", config.workspace_root.display());

    // Repository status
    println!("{}", "Repositories:".bold());
    let mut table = Table::new();
    table.set_header(vec!["Repository", "Path", "Branch", "Status"]);

    let repos = config.get_all_repositories();
    for (name, repo) in repos {
        let repo_path = config.workspace_root.join(&repo.path);
        
        let (exists, branch, status) = if repo_path.exists() {
            match git::status(&repo_path).await {
                Ok(git_status) => {
                    let status = if git_status.has_changes {
                        format!("{} changes", git_status.changed_files).yellow().to_string()
                    } else {
                        "Clean".green().to_string()
                    };
                    (true, git_status.branch, status)
                }
                Err(_) => (true, "unknown".to_string(), "Not a git repo".red().to_string()),
            }
        } else {
            (false, "-".to_string(), "Not cloned".red().to_string())
        };

        if exists || detailed {
            table.add_row(vec![
                Cell::new(name),
                Cell::new(&repo.path),
                Cell::new(branch),
                Cell::new(status),
            ]);
        }
    }
    
    println!("{}", table);

    // Service status
    println!("\n{}", "Services:".bold());
    let mut service_table = Table::new();
    service_table.set_header(vec!["Service", "Status", "Port", "Health"]);

    // Check Docker first
    match docker::check_docker().await {
        Ok(_) => {
            // Check each service
            for (name, repo) in config.get_all_repositories() {
                if !repo.ports.is_empty() {
                    let health = if let Some(health_check) = &repo.health_check {
                        match check_health(health_check).await {
                            Ok(true) => "Healthy".green().to_string(),
                            Ok(false) => "Unhealthy".red().to_string(),
                            Err(_) => "Unknown".yellow().to_string(),
                        }
                    } else {
                        "-".dimmed().to_string()
                    };

                    service_table.add_row(vec![
                        Cell::new(name),
                        Cell::new("Running"), // TODO: Actually check if running
                        Cell::new(repo.ports.join(", ")),
                        Cell::new(health),
                    ]);
                }
            }
        }
        Err(e) => {
            println!("{} Docker not available: {}", "Warning:".yellow(), e);
        }
    }

    let has_services = config.get_all_repositories()
        .iter()
        .any(|(_, repo)| !repo.ports.is_empty());
        
    if has_services {
        println!("{}", service_table);
    } else {
        println!("{}", "No services configured".dimmed());
    }

    // Infrastructure status
    if detailed {
        println!("\n{}", "Infrastructure:".bold());
        let mut infra_table = Table::new();
        infra_table.set_header(vec!["Component", "Type", "Status"]);

        for (name, infra) in &config.manifest.infrastructure {
            let status = match &infra.infra_type[..] {
                "external" => {
                    if let Some(health_check) = &infra.health_check {
                        match check_health(health_check).await {
                            Ok(true) => "Running".green().to_string(),
                            Ok(false) => "Stopped".red().to_string(),
                            Err(_) => "Unknown".yellow().to_string(),
                        }
                    } else {
                        "Unknown".yellow().to_string()
                    }
                }
                "system" => {
                    // TODO: Check system dependencies
                    "Available".green().to_string()
                }
                _ => "Unknown".yellow().to_string(),
            };

            infra_table.add_row(vec![
                Cell::new(name),
                Cell::new(&infra.infra_type),
                Cell::new(status),
            ]);
        }

        println!("{}", infra_table);
    }

    Ok(())
}

async fn check_health(health_check: &str) -> Result<bool> {
    if health_check.starts_with("http://") || health_check.starts_with("https://") {
        // HTTP health check
        match reqwest::get(health_check).await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    } else {
        // Command health check
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(health_check)
            .output()
            .await?;
        Ok(output.status.success())
    }
}
use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::Confirm;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::process::Command;

use crate::config::Config;
use crate::git;

pub async fn run(platform: Option<String>, yes: bool, force: bool, workspace_root: Option<PathBuf>) -> Result<()> {
    let config = Config::load(workspace_root)?;
    
    println!("{}", "Initializing Syla workspace...".bold());
    println!("Workspace root: {}\n", config.workspace_root.display());

    // Get repositories to clone
    let repos = if let Some(platform_name) = platform {
        println!("Cloning repositories for platform: {}", platform_name.cyan());
        config.get_platform_repositories(&platform_name)
            .ok_or_else(|| anyhow::anyhow!("Platform '{}' not found", platform_name))?
    } else {
        println!("Cloning all repositories");
        config.get_all_repositories()
    };

    if repos.is_empty() {
        println!("{}", "No repositories to clone".yellow());
        return Ok(());
    }

    // Show what will be cloned
    println!("\nRepositories to clone:");
    for (name, repo) in &repos {
        let repo_path = config.workspace_root.join(&repo.path);
        let status = if repo_path.exists() && force {
            " (will be re-cloned)".red().to_string()
        } else if repo_path.exists() {
            " (exists, will skip)".dimmed().to_string()
        } else {
            "".to_string()
        };
        
        println!("  {} {}{}", "*".cyan(), name, status);
        println!("    {} {}", "Path:".dimmed(), repo.path);
        println!("    {} {}", "URL:".dimmed(), repo.url);
    }
    println!();

    // Confirm
    if !yes {
        let prompt = if force {
            "Proceed with initialization? This will re-clone existing repositories and rebuild services."
        } else {
            "Proceed with initialization?"
        };
        
        let proceed = Confirm::new()
            .with_prompt(prompt)
            .default(true)
            .interact()?;
            
        if !proceed {
            println!("Aborted");
            return Ok(());
        }
    }

    // Clone repositories
    let pb = ProgressBar::new(repos.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-")
    );

    for (name, repo) in &repos {
        pb.set_message(format!("Cloning {}", name));
        
        let repo_path = config.workspace_root.join(&repo.path);
        
        // Check if already exists
        if repo_path.exists() && !force {
            pb.println(format!("{} {} already exists, skipping", "[OK]".green(), name));
            pb.inc(1);
            continue;
        } else if repo_path.exists() && force {
            pb.println(format!("{} Removing {} for re-clone", "[!]".yellow(), name));
            std::fs::remove_dir_all(&repo_path)
                .with_context(|| format!("Failed to remove {}", repo_path.display()))?;
        }

        // Create parent directory
        if let Some(parent) = repo_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        // Clone repository
        match git::clone(&repo.url, &repo_path, &repo.branch).await {
            Ok(_) => {
                pb.println(format!("{} Cloned {}", "[OK]".green(), name));
            }
            Err(e) => {
                pb.println(format!("{} Failed to clone {}: {}", "[X]".red(), name, e));
                if !yes {
                    return Err(e);
                }
            }
        }
        
        pb.inc(1);
    }

    pb.finish_with_message("Done");
    
    // Start Docker infrastructure
    println!("\n{}", "Setting up Docker infrastructure...".bold());
    start_docker_infrastructure(&config)?;
    
    // Build services
    println!("\n{}", "Building services...".bold());
    build_services(&config, &repos, force)?;
    
    // Run initial validation
    println!("\n{}", "Validating setup...".bold());
    validate_setup(&config)?;
    
    println!("\n{} Workspace initialized successfully!", "[OK]".green().bold());
    
    // Next steps
    println!("\n{}", "Next steps:".bold());
    println!("  {} Check status", "*".cyan());
    println!("    {}", "syla status".bright_black());
    println!("  {} Start development environment", "*".cyan());
    println!("    {}", "syla dev up".bright_black());
    println!("  {} Validate workspace", "*".cyan());
    println!("    {}", "syla dev validate".bright_black());

    Ok(())
}

fn start_docker_infrastructure(config: &Config) -> Result<()> {
    let docker_compose_path = config.workspace_root.join("docker-compose.yml");
    
    if !docker_compose_path.exists() {
        println!("{} docker-compose.yml not found, skipping", "[!]".yellow());
        return Ok(());
    }
    
    // Check if containers are already running
    let output = Command::new("docker")
        .args(&["compose", "ps", "-q"])
        .current_dir(&config.workspace_root)
        .output()
        .context("Failed to check Docker containers")?;
    
    if !output.stdout.is_empty() {
        println!("{} Docker containers already running", "[OK]".green());
        return Ok(());
    }
    
    // Start containers
    println!("Starting Docker containers...");
    let status = Command::new("docker")
        .args(&["compose", "up", "-d"])
        .current_dir(&config.workspace_root)
        .status()
        .context("Failed to start Docker containers")?;
    
    if status.success() {
        println!("{} Docker infrastructure started", "[OK]".green());
        
        // Wait for services to be ready
        std::thread::sleep(std::time::Duration::from_secs(3));
    } else {
        println!("{} Failed to start Docker containers", "[X]".red());
    }
    
    Ok(())
}

fn build_services(config: &Config, repos: &Vec<(String, &crate::config::RepositoryConfig)>, force: bool) -> Result<()> {
    for (name, repo) in repos {
        if repo.language == "rust" {
            let service_path = config.workspace_root.join(&repo.path);
            
            // Check if Cargo.toml exists
            if !service_path.join("Cargo.toml").exists() {
                continue;
            }
            
            // Check if already built
            let target_dir = service_path.join("target/release");
            if target_dir.exists() && target_dir.read_dir()?.any(|_| true) && !force {
                println!("{} {} already built", "[OK]".green(), name);
                continue;
            }
            
            println!("Building {}...", name);
            let status = Command::new("cargo")
                .args(&["build", "--release"])
                .current_dir(&service_path)
                .status()
                .with_context(|| format!("Failed to build {}", name))?;
            
            if status.success() {
                println!("{} Built {}", "[OK]".green(), name);
            } else {
                println!("{} Failed to build {}", "[X]".red(), name);
            }
        }
    }
    
    Ok(())
}

fn validate_setup(_config: &Config) -> Result<()> {
    // Check Redis connectivity
    let redis_status = Command::new("redis-cli")
        .args(&["-p", "6380", "ping"])
        .output()
        .context("Failed to check Redis")?;
    
    if redis_status.status.success() {
        println!("{} Redis is running", "[OK]".green());
    } else {
        println!("{} Redis is not accessible", "[!]".yellow());
    }
    
    // Check PostgreSQL connectivity
    let pg_status = Command::new("psql")
        .args(&[
            "-h", "localhost",
            "-p", "5434",
            "-U", "syla",
            "-d", "syla_dev",
            "-c", "SELECT 1"
        ])
        .env("PGPASSWORD", "syla_dev")
        .output();
    
    match pg_status {
        Ok(output) if output.status.success() => {
            println!("{} PostgreSQL is running", "[OK]".green());
        }
        _ => {
            println!("{} PostgreSQL is not accessible", "[!]".yellow());
        }
    }
    
    Ok(())
}
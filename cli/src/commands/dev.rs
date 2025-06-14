use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::collections::HashMap;
use tokio::time::interval;

use crate::config::Config;
use crate::services::{ProcessManager, ProcessConfig};
use crate::services::process_manager::RestartPolicy;
use crate::DevCommands;

pub async fn run(command: DevCommands, workspace_root: Option<PathBuf>) -> Result<()> {
    let config = Config::load(workspace_root)?;
    
    match command {
        DevCommands::Up { platform, detach } => {
            up(&config, platform, detach).await?;
        }
        DevCommands::Down { volumes } => {
            down(&config, volumes).await?;
        }
        DevCommands::Logs { service, follow, lines } => {
            logs(&config, &service, follow, lines).await?;
        }
        DevCommands::Restart { service } => {
            restart(&config, &service).await?;
        }
        DevCommands::Status { detailed } => {
            status(&config, detailed).await?;
        }
        DevCommands::Validate { fix, integration } => {
            validate(&config, fix, integration).await?;
        }
        DevCommands::Watch { services, build_only } => {
            watch(&config, services, build_only).await?;
        }
        DevCommands::BuildChanged { all } => {
            build_changed(&config, all).await?;
        }
    }
    Ok(())
}

async fn up(config: &Config, platform: Option<String>, detach: bool) -> Result<()> {
    println!("{}", "Starting development environment...".bold());
    
    // Check if we're in development mode
    let dev_mode = std::env::var("SYLA_DEV_MODE").unwrap_or_else(|_| "false".to_string()) == "true";
    
    // Start Docker infrastructure
    let docker_compose_path = config.workspace_root.join("docker-compose.yml");
    if docker_compose_path.exists() {
        println!("Starting Docker infrastructure...");
        
        let mut cmd = Command::new("docker");
        cmd.args(&["compose"]);
        
        // Add dev override if in dev mode
        if dev_mode {
            let dev_compose = config.workspace_root.join("docker-compose.dev.yml");
            if dev_compose.exists() {
                cmd.args(&["-f", "docker-compose.yml", "-f", "docker-compose.dev.yml"]);
            }
        }
        
        cmd.arg("up");
        if detach {
            cmd.arg("-d");
        }
        cmd.current_dir(&config.workspace_root);
        
        let status = cmd.status()
            .context("Failed to start Docker containers")?;
        
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to start Docker containers"));
        }
    }
    
    // Start services based on platform
    let repos = if let Some(platform_name) = platform {
        config.get_platform_repositories(&platform_name)
            .ok_or_else(|| anyhow::anyhow!("Platform '{}' not found", platform_name))?
    } else {
        config.get_all_repositories()
    };
    
    // Initialize ProcessManager
    let process_manager = ProcessManager::new(config.clone());
    
    // Start each service using ProcessManager
    for (name, repo) in repos {
        if !repo.ports.is_empty() && repo.language == "rust" {
            println!("Starting {}...", name);
            
            let service_path = config.workspace_root.join(&repo.path);
            let binary_name = repo.path.split('/').last().unwrap_or("service");
            let binary_path = service_path.join(format!("target/release/{}", binary_name));
            
            if !binary_path.exists() {
                println!("{} {} not built, skipping", "[!]".yellow(), name);
                continue;
            }
            
            // Create process configuration
            let mut env = HashMap::new();
            env.insert("RUST_LOG".to_string(), "info".to_string());
            
            // Extract port from the first port in the list
            if let Some(port) = repo.ports.first() {
                env.insert("PORT".to_string(), port.clone());
            }
            
            let process_config = ProcessConfig {
                name: name.clone(),
                command: binary_path.to_string_lossy().to_string(),
                args: vec![],
                working_dir: service_path,
                env,
                health_check_url: repo.health_check.clone(),
                health_check_interval: Duration::from_secs(10),
                startup_timeout: Duration::from_secs(30),
                restart_policy: RestartPolicy::OnFailure,
                log_file: Some(config.workspace_root.join(format!(".logs/{}.log", name))),
            };
            
            // Start the service
            match process_manager.start_service(process_config) {
                Ok(_) => println!("{} {} started on ports {:?}", "[OK]".green(), name, repo.ports),
                Err(e) => println!("{} Failed to start {}: {}", "[X]".red(), name, e),
            }
        }
    }
    
    println!("\n{} Development environment is ready!", "[OK]".green().bold());
    println!("Run {} to check status", "syla dev status".bright_black());
    
    Ok(())
}

async fn down(config: &Config, volumes: bool) -> Result<()> {
    println!("{}", "Stopping development environment...".bold());
    
    // Initialize ProcessManager to stop services
    let process_manager = ProcessManager::new(config.clone());
    
    // Stop all services
    println!("Stopping services...");
    if let Err(e) = process_manager.stop_all() {
        println!("{} Error stopping services: {}", "[!]".yellow(), e);
    } else {
        println!("{} All services stopped", "[OK]".green());
    }
    
    // Stop Docker containers
    let docker_compose_path = config.workspace_root.join("docker-compose.yml");
    if docker_compose_path.exists() {
        println!("Stopping Docker containers...");
        
        let mut cmd = Command::new("docker");
        cmd.args(&["compose", "down"]);
        if volumes {
            cmd.arg("-v");
        }
        cmd.current_dir(&config.workspace_root);
        
        let status = cmd.status()
            .context("Failed to stop Docker containers")?;
        
        if status.success() {
            println!("{} Docker containers stopped", "[OK]".green());
        }
    }
    
    println!("\n{} Development environment stopped", "[OK]".green().bold());
    
    Ok(())
}

async fn logs(config: &Config, service: &str, _follow: bool, _lines: usize) -> Result<()> {
    // Find the service
    let repos = config.get_all_repositories();
    let service_repo = repos.iter()
        .find(|(name, _)| name.contains(service))
        .ok_or_else(|| anyhow::anyhow!("Service '{}' not found", service))?;
    
    println!("Showing logs for {}...", service_repo.0);
    
    // TODO: Implement proper log viewing
    println!("{} Log viewing not yet implemented", "[!]".yellow());
    println!("Service path: {}", service_repo.1.path);
    
    Ok(())
}

async fn restart(config: &Config, service: &str) -> Result<()> {
    println!("Restarting {}...", service);
    
    // Initialize ProcessManager
    let process_manager = ProcessManager::new(config.clone());
    
    // Find the matching service
    let repos = config.get_all_repositories();
    let service_name = repos.iter()
        .find(|(name, _)| name.contains(service))
        .map(|(name, _)| name.clone());
    
    if let Some(name) = service_name {
        match process_manager.restart_service(&name) {
            Ok(_) => println!("{} {} restarted successfully", "[OK]".green(), name),
            Err(e) => println!("{} Failed to restart {}: {}", "[X]".red(), name, e),
        }
    } else {
        println!("{} Service '{}' not found", "[!]".yellow(), service);
    }
    
    Ok(())
}

async fn status(config: &Config, detailed: bool) -> Result<()> {
    println!("{}", "Development Environment Status".bold());
    println!();
    
    // Check Docker containers
    println!("{}", "Infrastructure:".cyan());
    let output = Command::new("docker")
        .args(&["compose", "ps"])
        .current_dir(&config.workspace_root)
        .output()
        .context("Failed to check Docker status")?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(2) {  // Skip header lines
            if !line.trim().is_empty() {
                println!("  {}", line);
            }
        }
    }
    
    // Check services
    println!("\n{}", "Services:".cyan());
    let repos = config.get_all_repositories();
    for (name, repo) in repos {
        if let Some(health_check) = &repo.health_check {
            let status = check_service_health(health_check).await;
            let status_icon = if status { "[OK]".green() } else { "[X]".red() };
            println!("  {} {}", status_icon, name);
            
            if detailed {
                println!("      Path: {}", repo.path);
                if !repo.ports.is_empty() {
                    println!("      Ports: {:?}", repo.ports);
                }
            }
        }
    }
    
    Ok(())
}

async fn validate(config: &Config, fix: bool, integration: bool) -> Result<()> {
    println!("{}", "Validating workspace setup...".bold());
    println!();
    
    let mut issues = Vec::new();
    
    // Check repositories
    println!("{} Checking repositories...", "->".dimmed());
    let repos = config.get_all_repositories();
    for (name, repo) in &repos {
        let repo_path = config.workspace_root.join(&repo.path);
        if !repo_path.exists() {
            issues.push(format!("Repository {} not cloned", name));
            if fix {
                println!("{} Cloning {}...", "[!]".yellow(), name);
                // TODO: Clone repository
            }
        } else {
            println!("{} {} exists", "[OK]".green(), name);
        }
    }
    
    // Check Docker
    println!("\n{} Checking Docker infrastructure...", "->".dimmed());
    let docker_status = Command::new("docker")
        .args(&["compose", "ps", "-q"])
        .current_dir(&config.workspace_root)
        .output()
        .context("Failed to check Docker")?;
    
    if docker_status.stdout.is_empty() {
        issues.push("Docker containers not running".to_string());
        if fix {
            println!("{} Starting Docker containers...", "[!]".yellow());
            Command::new("docker")
                .args(&["compose", "up", "-d"])
                .current_dir(&config.workspace_root)
                .status()?;
        }
    } else {
        println!("{} Docker containers running", "[OK]".green());
    }
    
    // Check service builds
    println!("\n{} Checking service builds...", "->".dimmed());
    for (name, repo) in &repos {
        if repo.language == "rust" {
            let service_path = config.workspace_root.join(&repo.path);
            let target_dir = service_path.join("target/release");
            
            if !target_dir.exists() {
                issues.push(format!("Service {} not built", name));
                if fix {
                    println!("{} Building {}...", "[!]".yellow(), name);
                    Command::new("cargo")
                        .args(&["build", "--release"])
                        .current_dir(&service_path)
                        .status()?;
                }
            } else {
                println!("{} {} built", "[OK]".green(), name);
            }
        }
    }
    
    // Run integration tests if requested
    if integration {
        println!("\n{} Running integration tests...", "->".dimmed());
        // TODO: Implement integration tests
        println!("{} Integration tests not yet implemented", "[!]".yellow());
    }
    
    // Summary
    println!("\n{}", "Validation Summary".bold());
    if issues.is_empty() {
        println!("{} No issues found!", "[OK]".green().bold());
    } else {
        println!("{} Found {} issues:", "[!]".yellow().bold(), issues.len());
        for issue in issues {
            println!("  - {}", issue);
        }
        if !fix {
            println!("\nRun with {} to fix issues", "--fix".bright_black());
        }
    }
    
    Ok(())
}

async fn check_service_health(url: &str) -> bool {
    // Simple HTTP health check
    match reqwest::get(url).await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

async fn watch(config: &Config, _services: Vec<String>, build_only: bool) -> Result<()> {
    println!("{}", "Starting file watcher...".bold());
    println!("Watching for changes (press Ctrl+C to stop)");
    
    // Use make watch if available
    let makefile = config.workspace_root.join("Makefile");
    if makefile.exists() {
        let mut cmd = Command::new("make");
        cmd.arg("dev-watch");
        cmd.current_dir(&config.workspace_root);
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        
        let status = cmd.status()
            .context("Failed to run make dev-watch")?;
            
        if !status.success() {
            return Err(anyhow::anyhow!("Watch command failed"));
        }
    } else {
        // Fallback to simple polling
        let mut interval = interval(Duration::from_secs(2));
        
        loop {
            interval.tick().await;
            
            // Detect changes
            let output = Command::new(&config.workspace_root.join("scripts/detect-changes.sh"))
                .output()
                .context("Failed to detect changes")?;
                
            let changed = String::from_utf8_lossy(&output.stdout);
            if !changed.trim().is_empty() {
                println!("\n{} Detected changes in: {}", "[*]".yellow(), changed.trim());
                
                // Build changed services
                for service in changed.split_whitespace() {
                    println!("Building {}...", service);
                    
                    let status = Command::new("make")
                        .arg(format!("{}-build", service))
                        .current_dir(&config.workspace_root)
                        .status()
                        .context("Failed to build service")?;
                        
                    if status.success() && !build_only {
                        // Restart service
                        let service_path = PathBuf::from(service);
                        let service_name = service_path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string();
                        println!("Restarting {}...", service_name);
                        restart(config, &service_name).await?;
                    }
                }
            }
        }
    }
    
    Ok(())
}

async fn build_changed(config: &Config, all: bool) -> Result<()> {
    println!("{}", "Building changed services...".bold());
    
    let mut cmd = Command::new("make");
    if all {
        cmd.arg("all");
    } else {
        cmd.arg("build-changed");
    }
    cmd.current_dir(&config.workspace_root);
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    
    let status = cmd.status()
        .context("Failed to run make build")?;
        
    if !status.success() {
        return Err(anyhow::anyhow!("Build failed"));
    }
    
    println!("{} Build complete", "✓".green());
    Ok(())
}
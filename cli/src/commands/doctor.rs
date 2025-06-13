use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use which::which;

use crate::config::Config;
use crate::docker;

pub async fn run(fix: bool, workspace_root: Option<PathBuf>) -> Result<()> {
    let config = Config::load(workspace_root)?;
    
    println!("{} {}", "[?]".cyan(), "Checking system health...".bold());
    println!();

    let mut all_good = true;

    // Check workspace
    print!("Workspace: ");
    if config.workspace_root.exists() {
        println!("{} ({})", "[OK]".green(), config.workspace_root.display());
    } else {
        println!("{} (not found)", "[X]".red());
        all_good = false;
    }

    // Check Git
    print!("Git: ");
    match which("git") {
        Ok(path) => {
            match tokio::process::Command::new("git")
                .arg("--version")
                .output()
                .await
            {
                Ok(output) => {
                    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    println!("{} ({} at {})", "[OK]".green(), version, path.display());
                }
                Err(e) => {
                    println!("{} (error: {})", "[X]".red(), e);
                    all_good = false;
                }
            }
        }
        Err(_) => {
            println!("{} (not found)", "[X]".red());
            all_good = false;
            if fix {
                println!("  {} Install git: https://git-scm.com/downloads", "->".dimmed());
            }
        }
    }

    // Check Docker
    print!("Docker: ");
    match docker::check_docker().await {
        Ok(version) => println!("{} ({})", "[OK]".green(), version),
        Err(e) => {
            println!("{} ({})", "[X]".red(), e);
            all_good = false;
            if fix {
                println!("  {} Install Docker: https://docs.docker.com/get-docker/", "->".dimmed());
            }
        }
    }

    // Check Rust
    print!("Rust: ");
    match which("cargo") {
        Ok(path) => {
            match tokio::process::Command::new("rustc")
                .arg("--version")
                .output()
                .await
            {
                Ok(output) => {
                    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    println!("{} ({} at {})", "[OK]".green(), version, path.display());
                }
                Err(e) => {
                    println!("{} (error: {})", "[X]".red(), e);
                    all_good = false;
                }
            }
        }
        Err(_) => {
            println!("{} (not found)", "[X]".red());
            all_good = false;
            if fix {
                println!("  {} Install Rust: https://rustup.rs/", "->".dimmed());
            }
        }
    }

    // Check configuration
    print!("Configuration: ");
    let config_path = config.workspace_root.join(".platform/config/repos.toml");
    if config_path.exists() {
        println!("{} (repos.toml)", "[OK]".green());
    } else {
        println!("{} (repos.toml not found)", "[X]".red());
        all_good = false;
    }

    // Summary
    println!();
    if all_good {
        println!("{} {}", "[OK]".green().bold(), "System ready!".bold());
    } else {
        println!("{} {}", "[X]".red().bold(), "Issues found".bold());
        if !fix {
            println!("\nRun {} to see fix suggestions", "syla doctor --fix".bright_black());
        }
    }

    Ok(())
}
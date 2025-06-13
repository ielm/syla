use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

mod commands;
mod config;
mod docker;
mod git;
mod platform;
mod services;

use commands::{dev, doctor, init, platform as platform_cmd, status};

#[derive(Parser)]
#[command(name = "syla")]
#[command(about = "Meta-platform CLI for DataCurve development")]
#[command(version)]
#[command(author)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to workspace root
    #[arg(short, long, global = true)]
    workspace: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize workspace by cloning all repositories
    Init {
        /// Only clone specific platform
        #[arg(short, long)]
        platform: Option<String>,

        /// Skip confirmation prompts
        #[arg(short = 'y', long)]
        yes: bool,

        /// Force re-initialization (re-clone repos, rebuild services)
        #[arg(short, long)]
        force: bool,
    },

    /// Show status of all repositories and services
    Status {
        /// Show detailed status
        #[arg(short, long)]
        detailed: bool,
    },

    /// Platform-specific operations
    Platform {
        #[command(subcommand)]
        command: PlatformCommands,
    },

    /// Development environment management
    Dev {
        #[command(subcommand)]
        command: DevCommands,
    },

    /// Check system health and dependencies
    Doctor {
        /// Fix issues if possible
        #[arg(long)]
        fix: bool,
    },

    /// Manage workspace configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Execute code using Syla platform
    Exec {
        /// File to execute
        file: PathBuf,

        /// Language (auto-detected if not specified)
        #[arg(short, long)]
        language: Option<String>,

        /// Use local Docker instead of platform
        #[arg(long)]
        local: bool,
    },
}

#[derive(Subcommand)]
enum PlatformCommands {
    /// List all platforms
    List,

    /// Show platform status
    Status {
        /// Platform name
        platform: String,
    },

    /// Start a platform
    Start {
        /// Platform name
        platform: String,

        /// Start with dependencies
        #[arg(long)]
        with_deps: bool,
    },

    /// Stop a platform
    Stop {
        /// Platform name
        platform: String,
    },

    /// Run platform tests
    Test {
        /// Platform name
        platform: String,

        /// Run integration tests
        #[arg(long)]
        integration: bool,
    },
}

#[derive(Subcommand)]
enum DevCommands {
    /// Start development environment
    Up {
        /// Platform to start (all if not specified)
        #[arg(short, long)]
        platform: Option<String>,

        /// Detached mode
        #[arg(short, long)]
        detach: bool,
    },

    /// Stop development environment
    Down {
        /// Remove volumes
        #[arg(short, long)]
        volumes: bool,
    },

    /// Show service logs
    Logs {
        /// Service path (e.g., syla/core/api-gateway)
        service: String,

        /// Follow log output
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "100")]
        lines: usize,
    },

    /// Restart a service
    Restart {
        /// Service path
        service: String,
    },

    /// Show development environment status
    Status {
        /// Show detailed status
        #[arg(short, long)]
        detailed: bool,
    },

    /// Validate workspace setup
    Validate {
        /// Fix issues if possible
        #[arg(long)]
        fix: bool,

        /// Run integration tests
        #[arg(long)]
        integration: bool,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,

        /// Configuration value
        value: String,
    },

    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose { "debug" } else { "info" };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Print header
    println!(
        "\n{} {}\n",
        "Syla".cyan().bold(),
        "Meta-Platform CLI".dimmed()
    );

    // Execute command
    match cli.command {
        Commands::Init {
            platform,
            yes,
            force,
        } => {
            init::run(platform, yes, force, cli.workspace).await?;
        }
        Commands::Status { detailed } => {
            status::run(detailed, cli.workspace).await?;
        }
        Commands::Platform { command } => {
            platform_cmd::run(command, cli.workspace).await?;
        }
        Commands::Dev { command } => {
            dev::run(command, cli.workspace).await?;
        }
        Commands::Doctor { fix } => {
            doctor::run(fix, cli.workspace).await?;
        }
        Commands::Config { command: _ } => {
            println!("Config command not yet implemented");
        }
        Commands::Exec {
            file: _,
            language: _,
            local: _,
        } => {
            println!("Exec command not yet implemented");
        }
    }

    Ok(())
}


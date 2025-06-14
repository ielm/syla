pub mod commands;
pub mod config;
pub mod docker;
pub mod git;
pub mod platform;
pub mod services;

// Re-export commonly used types
pub use config::Config;
pub type Result<T> = anyhow::Result<T>;

// Re-export command enums from main (they're defined there)
use clap::Subcommand;

#[derive(Subcommand)]
pub enum DevCommands {
    /// Start development environment
    Up {
        /// Platform to start (all if not specified)
        #[clap(short, long)]
        platform: Option<String>,

        /// Detached mode
        #[clap(short, long)]
        detach: bool,
    },

    /// Stop development environment
    Down {
        /// Remove volumes
        #[clap(short, long)]
        volumes: bool,
    },

    /// Show service logs
    Logs {
        /// Service path (e.g., syla/core/api-gateway)
        service: String,

        /// Follow log output
        #[clap(short, long)]
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
        #[clap(short, long)]
        detailed: bool,
    },

    /// Validate workspace setup
    Validate {
        /// Fix issues if possible
        #[clap(long)]
        fix: bool,

        /// Run integration tests
        #[clap(long)]
        integration: bool,
    },

    /// Watch for changes and auto-rebuild/restart
    Watch {
        /// Services to watch (all if not specified)
        #[clap(short, long)]
        services: Vec<String>,

        /// Build only, don't restart
        #[clap(long)]
        build_only: bool,
    },

    /// Build changed services
    BuildChanged {
        /// Force rebuild all
        #[clap(long)]
        all: bool,
    },
}

#[derive(Subcommand)]
pub enum PlatformCommands {
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
        #[clap(long)]
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
        #[clap(long)]
        integration: bool,
    },
}
use anyhow::{Context, Result};
use bollard::Docker;

pub async fn check_docker() -> Result<String> {
    let docker = Docker::connect_with_local_defaults()
        .context("Failed to connect to Docker")?;
    
    let version = docker.version().await
        .context("Failed to get Docker version")?;
    
    Ok(format!("Docker {}", version.version.unwrap_or_else(|| "unknown".to_string())))
}

pub async fn is_container_running(name: &str) -> Result<bool> {
    let docker = Docker::connect_with_local_defaults()?;
    
    match docker.inspect_container(name, None).await {
        Ok(info) => {
            Ok(info.state
                .and_then(|s| s.running)
                .unwrap_or(false))
        }
        Err(_) => Ok(false),
    }
}
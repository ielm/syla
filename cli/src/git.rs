use anyhow::{Context, Result};
use std::path::Path;
use tokio::process::Command;

pub async fn clone(url: &str, path: &Path, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .args(&["clone", "-b", branch, url, path.to_str().unwrap()])
        .output()
        .await
        .context("Failed to execute git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git clone failed: {}", stderr);
    }

    Ok(())
}

pub async fn status(repo_path: &Path) -> Result<GitStatus> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(&["status", "--porcelain", "-b"])
        .output()
        .await
        .context("Failed to execute git status")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git status failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    
    let branch = lines.get(0)
        .and_then(|line| line.strip_prefix("## "))
        .unwrap_or("unknown")
        .to_string();
    
    let has_changes = lines.len() > 1;

    Ok(GitStatus {
        branch,
        has_changes,
        changed_files: lines.len().saturating_sub(1),
    })
}

pub async fn pull(repo_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(&["pull", "--ff-only"])
        .output()
        .await
        .context("Failed to execute git pull")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git pull failed: {}", stderr);
    }

    Ok(())
}

#[derive(Debug)]
pub struct GitStatus {
    pub branch: String,
    pub has_changes: bool,
    pub changed_files: usize,
}
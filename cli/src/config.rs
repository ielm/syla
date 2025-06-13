use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoManifest {
    #[serde(default)]
    pub repositories: HashMap<String, RepositoryConfig>,
    #[serde(default)]
    pub infrastructure: HashMap<String, InfrastructureConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub url: String,
    pub path: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default)]
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check: Option<String>,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(rename = "type")]
    pub repo_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrastructureConfig {
    #[serde(rename = "type")]
    pub infra_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_image: Option<String>,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub environment: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_version: Option<String>,
}

fn default_branch() -> String {
    "main".to_string()
}

#[derive(Clone)]
pub struct Config {
    pub workspace_root: PathBuf,
    pub manifest: RepoManifest,
}

impl Config {
    pub fn load(workspace_root: Option<PathBuf>) -> Result<Self> {
        let workspace_root = if let Some(path) = workspace_root {
            path
        } else {
            // Try to find workspace root by looking for .platform directory
            let current_dir = std::env::current_dir()?;
            find_workspace_root(&current_dir)?
        };

        let manifest_path = workspace_root.join(".platform/config/repos.toml");
        let manifest_content = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read manifest at {}", manifest_path.display()))?;
        
        let manifest: RepoManifest = toml::from_str(&manifest_content)
            .context("Failed to parse repository manifest")?;

        Ok(Self {
            workspace_root,
            manifest,
        })
    }

    pub fn get_all_repositories(&self) -> Vec<(String, &RepositoryConfig)> {
        self.manifest.repositories
            .iter()
            .map(|(name, config)| (name.clone(), config))
            .collect()
    }

    pub fn get_platform_repositories(&self, platform: &str) -> Option<Vec<(String, &RepositoryConfig)>> {
        let repos: Vec<_> = self.manifest.repositories
            .iter()
            .filter(|(_, config)| {
                config.platform.as_ref().map_or(false, |p| p == platform)
            })
            .map(|(name, config)| (name.clone(), config))
            .collect();
            
        if repos.is_empty() {
            None
        } else {
            Some(repos)
        }
    }
}

fn find_workspace_root(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    
    loop {
        if current.join(".platform").exists() {
            return Ok(current);
        }
        
        if !current.pop() {
            anyhow::bail!(
                "Could not find workspace root. Make sure you're in a Syla workspace or use --workspace flag"
            );
        }
    }
}
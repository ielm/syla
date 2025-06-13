use std::process::Command;
use std::path::PathBuf;
use predicates::prelude::*;
use assert_cmd::Command as TestCommand;
use tempfile::TempDir;

#[test]
fn test_syla_version() {
    let mut cmd = TestCommand::cargo_bin("syla").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("syla"));
}

#[test]
fn test_syla_help() {
    let mut cmd = TestCommand::cargo_bin("syla").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Meta-platform CLI"));
}

#[test]
fn test_syla_doctor() {
    let mut cmd = TestCommand::cargo_bin("syla").unwrap();
    cmd.arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking system requirements"));
}

#[test]
fn test_syla_status_without_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = TestCommand::cargo_bin("syla").unwrap();
    cmd.arg("status")
        .arg("--workspace")
        .arg(temp_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read manifest"));
}

#[test]
fn test_dev_validate_command() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = TestCommand::cargo_bin("syla").unwrap();
    cmd.arg("dev")
        .arg("validate")
        .arg("--workspace")
        .arg(temp_dir.path())
        .assert()
        .failure();
}

#[cfg(test)]
mod workspace_tests {
    use super::*;
    use std::fs;

    fn create_test_workspace() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        
        // Create .platform directory structure
        let platform_dir = temp_dir.path().join(".platform/config");
        fs::create_dir_all(&platform_dir).unwrap();
        
        // Create a minimal repos.toml
        let repos_toml = r#"
[repositories."test.service"]
url = "https://github.com/test/service.git"
path = "test/service"
branch = "main"
language = "rust"
"#;
        fs::write(platform_dir.join("repos.toml"), repos_toml).unwrap();
        
        // Create docker-compose.yml
        let docker_compose = r#"
version: '3.8'
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
"#;
        fs::write(temp_dir.path().join("docker-compose.yml"), docker_compose).unwrap();
        
        temp_dir
    }

    #[test]
    fn test_syla_status_with_workspace() {
        let workspace = create_test_workspace();
        let mut cmd = TestCommand::cargo_bin("syla").unwrap();
        cmd.arg("status")
            .arg("--workspace")
            .arg(workspace.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("Repository Status"));
    }

    #[test]
    fn test_syla_init_dry_run() {
        let workspace = create_test_workspace();
        let mut cmd = TestCommand::cargo_bin("syla").unwrap();
        cmd.arg("init")
            .arg("--workspace")
            .arg(workspace.path())
            .arg("--yes")
            .env("DRY_RUN", "1")
            .assert()
            .success();
    }
}
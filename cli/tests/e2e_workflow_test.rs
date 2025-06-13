#[cfg(test)]
mod e2e_workflow_tests {
    use std::process::Command;
    use std::path::Path;
    use std::fs;
    use tempfile::TempDir;
    use assert_cmd::Command as TestCommand;
    use predicates::prelude::*;

    fn setup_test_workspace() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        
        // Create .platform directory structure
        let platform_dir = temp_dir.path().join(".platform/config");
        fs::create_dir_all(&platform_dir).unwrap();
        
        // Create a comprehensive repos.toml
        let repos_toml = r#"
[repositories."syla.core.api-gateway"]
url = "https://github.com/test/api-gateway.git"
path = "platforms/syla/core/api-gateway"
branch = "main"
language = "rust"
platform = "syla"
ports = ["8084"]
health_check = "http://localhost:8084/health"

[repositories."syla.core.execution-service"]
url = "https://github.com/test/execution-service.git"
path = "platforms/syla/core/execution-service"
branch = "main"
language = "rust"
platform = "syla"
ports = ["8083"]
health_check = "http://localhost:8083/health"
depends_on = ["redis"]

[infrastructure.redis]
type = "redis"
docker_image = "redis:7-alpine"
ports = ["16379:6379"]

[infrastructure.postgres]
type = "postgres"
docker_image = "postgres:15-alpine"
ports = ["15432:5432"]
environment = ["POSTGRES_PASSWORD=syla"]
"#;
        fs::write(platform_dir.join("repos.toml"), repos_toml).unwrap();
        
        // Create docker-compose.yml
        let docker_compose = r#"
version: '3.8'
services:
  redis:
    image: redis:7-alpine
    ports:
      - "16379:6379"
  
  postgres:
    image: postgres:15-alpine
    ports:
      - "15432:5432"
    environment:
      POSTGRES_PASSWORD: syla
"#;
        fs::write(temp_dir.path().join("docker-compose.yml"), docker_compose).unwrap();
        
        // Create .gitignore
        let gitignore = r#"
# Child repository directories
platforms/
.logs/
"#;
        fs::write(temp_dir.path().join(".gitignore"), gitignore).unwrap();
        
        temp_dir
    }

    #[test]
    fn test_full_dev_workflow() {
        let workspace = setup_test_workspace();
        let workspace_path = workspace.path().to_str().unwrap();
        
        // 1. Test doctor command
        let mut cmd = TestCommand::cargo_bin("syla").unwrap();
        cmd.arg("doctor")
            .arg("--workspace")
            .arg(workspace_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("All checks passed"));
        
        // 2. Test status command
        let mut cmd = TestCommand::cargo_bin("syla").unwrap();
        cmd.arg("status")
            .arg("--workspace")
            .arg(workspace_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("Repository Status"))
            .stdout(predicate::str::contains("syla.core.api-gateway"))
            .stdout(predicate::str::contains("Not cloned"));
        
        // 3. Test dev validate command
        let mut cmd = TestCommand::cargo_bin("syla").unwrap();
        cmd.arg("dev")
            .arg("validate")
            .arg("--workspace")
            .arg(workspace_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("Validating workspace setup"));
    }

    #[test]
    fn test_init_workflow_dry_run() {
        let workspace = setup_test_workspace();
        let workspace_path = workspace.path().to_str().unwrap();
        
        // Test init with dry run
        let mut cmd = TestCommand::cargo_bin("syla").unwrap();
        cmd.arg("init")
            .arg("--workspace")
            .arg(workspace_path)
            .arg("--yes")
            .env("DRY_RUN", "1")
            .assert()
            .success()
            .stdout(predicate::str::contains("Initializing workspace"))
            .stdout(predicate::str::contains("Cloning repositories"));
    }

    #[test]
    fn test_platform_commands() {
        let workspace = setup_test_workspace();
        let workspace_path = workspace.path().to_str().unwrap();
        
        // Test platform list
        let mut cmd = TestCommand::cargo_bin("syla").unwrap();
        cmd.arg("platform")
            .arg("list")
            .arg("--workspace")
            .arg(workspace_path)
            .assert()
            .success();
    }

    #[test]
    fn test_dev_status_detailed() {
        let workspace = setup_test_workspace();
        let workspace_path = workspace.path().to_str().unwrap();
        
        // Test detailed dev status
        let mut cmd = TestCommand::cargo_bin("syla").unwrap();
        cmd.arg("dev")
            .arg("status")
            .arg("--detailed")
            .arg("--workspace")
            .arg(workspace_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("Development Environment Status"));
    }
}
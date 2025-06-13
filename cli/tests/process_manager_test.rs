#[cfg(test)]
mod process_manager_tests {
    use syla::services::{ProcessManager, ProcessConfig};
    use syla::services::process_manager::RestartPolicy;
    use syla::config::Config;
    use std::time::Duration;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config() -> (Config, TempDir) {
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
        
        let config = Config::load(Some(temp_dir.path().to_path_buf())).unwrap();
        (config, temp_dir)
    }

    #[test]
    fn test_process_manager_creation() {
        let (config, _temp_dir) = create_test_config();
        let _pm = ProcessManager::new(config);
        // If we get here without panic, the test passes
    }

    #[test]
    fn test_start_nonexistent_service() {
        let (config, temp_dir) = create_test_config();
        let pm = ProcessManager::new(config);
        
        let process_config = ProcessConfig {
            name: "test-service".to_string(),
            command: "/nonexistent/binary".to_string(),
            args: vec![],
            working_dir: temp_dir.path().to_path_buf(),
            env: HashMap::new(),
            health_check_url: None,
            health_check_interval: Duration::from_secs(10),
            startup_timeout: Duration::from_secs(30),
            restart_policy: RestartPolicy::Never,
            log_file: None,
        };
        
        let result = pm.start_service(process_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_stop_nonexistent_service() {
        let (config, _temp_dir) = create_test_config();
        let pm = ProcessManager::new(config);
        
        // Stopping a non-existent service should not error
        let result = pm.stop_service("nonexistent", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_start_echo_command() {
        let (config, temp_dir) = create_test_config();
        let pm = ProcessManager::new(config);
        
        // Create a simple test script
        let script_path = temp_dir.path().join("test.sh");
        fs::write(&script_path, "#!/bin/bash\nwhile true; do echo 'test'; sleep 1; done").unwrap();
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms).unwrap();
        }
        
        let process_config = ProcessConfig {
            name: "test-echo".to_string(),
            command: script_path.to_string_lossy().to_string(),
            args: vec![],
            working_dir: temp_dir.path().to_path_buf(),
            env: HashMap::new(),
            health_check_url: None,
            health_check_interval: Duration::from_secs(10),
            startup_timeout: Duration::from_secs(30),
            restart_policy: RestartPolicy::Never,
            log_file: None,
        };
        
        let result = pm.start_service(process_config);
        assert!(result.is_ok());
        
        // Give the process time to start
        std::thread::sleep(Duration::from_millis(100));
        
        // Stop the service
        let stop_result = pm.stop_service("test-echo", false);
        assert!(stop_result.is_ok());
    }
}
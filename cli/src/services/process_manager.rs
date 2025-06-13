use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::path::PathBuf;
use std::io::{BufRead, BufReader};
use std::thread;
use std::fs::OpenOptions;
use std::io::Write;

use colored::*;

use anyhow::Result;
use crate::config::Config;
use crate::services::LogStreamer;

#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
    pub env: HashMap<String, String>,
    pub health_check_url: Option<String>,
    pub health_check_interval: Duration,
    pub startup_timeout: Duration,
    pub restart_policy: RestartPolicy,
    pub log_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RestartPolicy {
    Never,
    OnFailure,
    Always,
    UnlessStopped,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed(String),
    Restarting,
}

pub struct ServiceProcess {
    pub config: ProcessConfig,
    pub state: ProcessState,
    pub process: Option<Child>,
    pub started_at: Option<Instant>,
    pub restart_count: u32,
    pub last_health_check: Option<Instant>,
    pub health_status: HealthStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Unknown,
    Healthy,
    Unhealthy(String),
}

pub struct ProcessManager {
    services: Arc<Mutex<HashMap<String, ServiceProcess>>>,
    config: Config,
}

impl ProcessManager {
    pub fn new(config: Config) -> Self {
        Self {
            services: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    pub fn start_service(&self, process_config: ProcessConfig) -> Result<()> {
        let name = process_config.name.clone();
        println!("{} {}", "Starting service:".green(), name.bold());

        let mut services = self.services.lock().unwrap();
        
        // Check if already running
        if let Some(service) = services.get(&name) {
            if matches!(service.state, ProcessState::Running) {
                println!("{} {} is already running", "✓".green(), name);
                return Ok(());
            }
        }

        // Create and start the process
        let mut service = ServiceProcess {
            config: process_config.clone(),
            state: ProcessState::Starting,
            process: None,
            started_at: None,
            restart_count: 0,
            last_health_check: None,
            health_status: HealthStatus::Unknown,
        };

        match self.spawn_process(&process_config) {
            Ok(child) => {
                service.process = Some(child);
                service.state = ProcessState::Running;
                service.started_at = Some(Instant::now());
                
                println!("{} {} started successfully", "✓".green(), name.bold());
                
                // Start log streaming if configured
                if process_config.log_file.is_some() {
                    self.start_log_streaming(&name);
                }
                
                services.insert(name.clone(), service);
                
                // Start health monitoring
                self.start_health_monitoring(name);
                
                Ok(())
            }
            Err(e) => {
                service.state = ProcessState::Failed(e.to_string());
                services.insert(name, service);
                Err(e)
            }
        }
    }

    pub fn stop_service(&self, name: &str, force: bool) -> Result<()> {
        println!("{} {}", "Stopping service:".yellow(), name.bold());
        
        let mut services = self.services.lock().unwrap();
        
        if let Some(service) = services.get_mut(name) {
            if let ProcessState::Stopped = service.state {
                println!("{} {} is already stopped", "✓".green(), name);
                return Ok(());
            }
            
            service.state = ProcessState::Stopping;
            
            if let Some(mut process) = service.process.take() {
                if force {
                    process.kill()?;
                    println!("{} {} killed", "✓".yellow(), name);
                } else {
                    // Try graceful shutdown first
                    #[cfg(unix)]
                    {
                        use nix::sys::signal::{self, Signal};
                        use nix::unistd::Pid;
                        
                        if let Ok(pid) = process.id().try_into() {
                            let _ = signal::kill(Pid::from_raw(pid), Signal::SIGTERM);
                        }
                    }
                    
                    // Wait for graceful shutdown
                    thread::sleep(Duration::from_secs(5));
                    
                    match process.try_wait()? {
                        Some(_) => {
                            println!("{} {} stopped gracefully", "✓".green(), name);
                        }
                        None => {
                            process.kill()?;
                            println!("{} {} force killed", "✓".yellow(), name);
                        }
                    }
                }
                
                service.state = ProcessState::Stopped;
            }
            
            Ok(())
        } else {
            println!("{} Service {} not found", "⚠".yellow(), name);
            Ok(())
        }
    }

    pub fn restart_service(&self, name: &str) -> Result<()> {
        println!("{} {}", "Restarting service:".blue(), name.bold());
        
        let config = {
            let services = self.services.lock().unwrap();
            services.get(name).map(|s| s.config.clone())
        };
        
        if let Some(config) = config {
            self.stop_service(name, false)?;
            thread::sleep(Duration::from_secs(1));
            self.start_service(config)?;
            
            let mut services = self.services.lock().unwrap();
            if let Some(service) = services.get_mut(name) {
                service.restart_count += 1;
            }
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Service {} not found", name))
        }
    }

    pub fn get_service_status(&self, name: &str) -> Option<(ProcessState, HealthStatus)> {
        let services = self.services.lock().unwrap();
        services.get(name).map(|s| (s.state.clone(), s.health_status.clone()))
    }

    pub fn list_services(&self) -> Vec<(String, ProcessState, HealthStatus)> {
        let services = self.services.lock().unwrap();
        services.iter()
            .map(|(name, service)| {
                (name.clone(), service.state.clone(), service.health_status.clone())
            })
            .collect()
    }

    fn spawn_process(&self, config: &ProcessConfig) -> Result<Child> {
        let mut cmd = Command::new(&config.command);
        
        cmd.args(&config.args)
            .current_dir(&config.working_dir)
            .envs(&config.env)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // Create new process group
            cmd.process_group(0);
        }
        
        Ok(cmd.spawn()?)
    }

    fn start_health_monitoring(&self, name: String) {
        let services = self.services.clone();
        
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(10));
                
                let should_check = {
                    let services = services.lock().unwrap();
                    if let Some(service) = services.get(&name) {
                        matches!(service.state, ProcessState::Running)
                            && service.config.health_check_url.is_some()
                    } else {
                        false
                    }
                };
                
                if !should_check {
                    break;
                }
                
                // Perform health check
                let health_status = {
                    let services = services.lock().unwrap();
                    if let Some(service) = services.get(&name) {
                        if let Some(url) = &service.config.health_check_url {
                            match Self::check_health(url) {
                                Ok(()) => HealthStatus::Healthy,
                                Err(e) => HealthStatus::Unhealthy(e.to_string()),
                            }
                        } else {
                            HealthStatus::Unknown
                        }
                    } else {
                        break;
                    }
                };
                
                // Update health status
                let mut services = services.lock().unwrap();
                if let Some(service) = services.get_mut(&name) {
                    service.health_status = health_status;
                    service.last_health_check = Some(Instant::now());
                    
                    // Handle restart policy
                    if let HealthStatus::Unhealthy(_) = &service.health_status {
                        if matches!(service.config.restart_policy, RestartPolicy::OnFailure | RestartPolicy::Always) {
                            service.state = ProcessState::Restarting;
                            // Restart will be handled by another thread
                        }
                    }
                }
            }
        });
    }

    fn check_health(url: &str) -> Result<()> {
        let response = ureq::get(url)
            .timeout(Duration::from_secs(5))
            .call();
        
        match response {
            Ok(resp) if resp.status() >= 200 && resp.status() < 300 => Ok(()),
            Ok(resp) => Err(anyhow::anyhow!("Health check failed with status: {}", resp.status())),
            Err(e) => Err(anyhow::anyhow!("Health check failed: {}", e)),
        }
    }

    fn start_log_streaming(&self, _name: &str) {
        // TODO: Implement log streaming
        // This will be implemented in the next step
    }

    pub fn stop_all(&self) -> Result<()> {
        let services: Vec<String> = {
            let services = self.services.lock().unwrap();
            services.keys().cloned().collect()
        };
        
        for name in services {
            let _ = self.stop_service(&name, false);
        }
        
        Ok(())
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        let _ = self.stop_all();
    }
}
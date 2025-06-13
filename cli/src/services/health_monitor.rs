use std::time::{Duration, Instant};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub endpoint: String,
    pub interval: Duration,
    pub timeout: Duration,
    pub retries: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Unknown,
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

#[derive(Debug, Clone)]
pub struct ServiceHealth {
    pub name: String,
    pub status: HealthStatus,
    pub last_check: Option<Instant>,
    pub consecutive_failures: u32,
    pub uptime: Option<Duration>,
    pub response_time: Option<Duration>,
}

pub struct HealthMonitor {
    checks: HashMap<String, HealthCheck>,
    results: HashMap<String, ServiceHealth>,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            checks: HashMap::new(),
            results: HashMap::new(),
        }
    }

    pub fn add_check(&mut self, name: String, check: HealthCheck) {
        self.checks.insert(name.clone(), check);
        self.results.insert(name.clone(), ServiceHealth {
            name,
            status: HealthStatus::Unknown,
            last_check: None,
            consecutive_failures: 0,
            uptime: None,
            response_time: None,
        });
    }

    pub fn perform_check(&mut self, name: &str) -> Result<HealthStatus> {
        let check = self.checks.get(name)
            .ok_or_else(|| anyhow::anyhow!("Health check {} not found", name))?
            .clone();
        
        let start = Instant::now();
        let status = self.check_endpoint(&check)?;
        let response_time = start.elapsed();
        
        if let Some(health) = self.results.get_mut(name) {
            health.status = status.clone();
            health.last_check = Some(Instant::now());
            health.response_time = Some(response_time);
            
            match &status {
                HealthStatus::Healthy => {
                    health.consecutive_failures = 0;
                    if health.uptime.is_none() {
                        health.uptime = Some(Duration::from_secs(0));
                    }
                }
                HealthStatus::Unhealthy(_) | HealthStatus::Degraded(_) => {
                    health.consecutive_failures += 1;
                }
                _ => {}
            }
        }
        
        Ok(status)
    }

    pub fn check_all(&mut self) -> HashMap<String, HealthStatus> {
        let names: Vec<String> = self.checks.keys().cloned().collect();
        let mut results = HashMap::new();
        
        for name in names {
            if let Ok(status) = self.perform_check(&name) {
                results.insert(name, status);
            }
        }
        
        results
    }

    pub fn get_status(&self, name: &str) -> Option<&ServiceHealth> {
        self.results.get(name)
    }

    pub fn get_all_status(&self) -> &HashMap<String, ServiceHealth> {
        &self.results
    }

    fn check_endpoint(&self, check: &HealthCheck) -> Result<HealthStatus> {
        let response = ureq::get(&check.endpoint)
            .timeout(check.timeout)
            .call();
        
        match response {
            Ok(resp) => {
                let status = resp.status();
                if status >= 200 && status < 300 {
                    Ok(HealthStatus::Healthy)
                } else if status >= 500 {
                    Ok(HealthStatus::Unhealthy(format!("Server error: {}", status)))
                } else {
                    Ok(HealthStatus::Degraded(format!("Status: {}", status)))
                }
            }
            Err(ureq::Error::Status(code, _)) => {
                if code >= 500 {
                    Ok(HealthStatus::Unhealthy(format!("Server error: {}", code)))
                } else {
                    Ok(HealthStatus::Degraded(format!("Status: {}", code)))
                }
            }
            Err(e) => Ok(HealthStatus::Unhealthy(format!("Connection error: {}", e))),
        }
    }

    pub fn is_healthy(&self, name: &str) -> bool {
        self.results.get(name)
            .map(|h| matches!(h.status, HealthStatus::Healthy))
            .unwrap_or(false)
    }

    pub fn get_unhealthy_services(&self) -> Vec<String> {
        self.results.iter()
            .filter(|(_, health)| !matches!(health.status, HealthStatus::Healthy | HealthStatus::Unknown))
            .map(|(name, _)| name.clone())
            .collect()
    }
}
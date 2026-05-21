//! Optimization types and validation

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Optimization type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Optimization {
    CpuGovernor(String),
    TransparentHugePages(bool),
    NetworkTuning,
    KernelParam(String, String),
    IoScheduler(String, String), // device, scheduler
    ReadAhead(String, usize),    // device, KB
}

impl Optimization {
    /// Get human-readable description
    pub fn description(&self) -> String {
        match self {
            Optimization::CpuGovernor(gov) => {
                format!("Set CPU governor to '{}'", gov)
            }
            Optimization::TransparentHugePages(enable) => {
                if *enable {
                    "Enable transparent huge pages".to_string()
                } else {
                    "Disable transparent huge pages".to_string()
                }
            }
            Optimization::NetworkTuning => {
                "Apply network optimizations (TCP Fast Open, buffer sizes, etc.)".to_string()
            }
            Optimization::KernelParam(key, value) => {
                format!("Set {} = {}", key, value)
            }
            Optimization::IoScheduler(device, scheduler) => {
                format!("Set I/O scheduler for {} to '{}'", device, scheduler)
            }
            Optimization::ReadAhead(device, kb) => {
                format!("Set read-ahead for {} to {}KB", device, kb)
            }
        }
    }

    /// Get current value
    pub fn current_value(&self) -> String {
        match self {
            Optimization::CpuGovernor(_) => {
                crate::system_apply::detect_cpu_governor().unwrap_or_else(|_| "unknown".to_string())
            }
            Optimization::TransparentHugePages(_) => {
                if crate::system_apply::is_thp_enabled().unwrap_or(false) {
                    "enabled".to_string()
                } else {
                    "disabled".to_string()
                }
            }
            Optimization::NetworkTuning => "default".to_string(),
            Optimization::KernelParam(key, _) => {
                crate::system_apply::sysctl_get(key).unwrap_or_else(|_| "unknown".to_string())
            }
            Optimization::IoScheduler(device, _) => crate::system_apply::read_io_schedulers()
                .ok()
                .and_then(|s| s.get(device).cloned())
                .unwrap_or_else(|| "unknown".to_string()),
            Optimization::ReadAhead(_, _) => "default".to_string(),
        }
    }

    /// Get new value
    pub fn new_value(&self) -> String {
        match self {
            Optimization::CpuGovernor(gov) => gov.clone(),
            Optimization::TransparentHugePages(enable) => {
                if *enable {
                    "enabled".to_string()
                } else {
                    "disabled".to_string()
                }
            }
            Optimization::NetworkTuning => "optimized".to_string(),
            Optimization::KernelParam(_, value) => value.clone(),
            Optimization::IoScheduler(_, scheduler) => scheduler.clone(),
            Optimization::ReadAhead(_, kb) => format!("{}KB", kb),
        }
    }

    /// Validate optimization
    pub fn validate(&self) -> Result<()> {
        match self {
            Optimization::CpuGovernor(gov) => {
                let available = crate::system_apply::get_available_governors()?;
                if !available.is_empty() && !available.contains(&gov.to_string()) {
                    return Err(anyhow!(
                        "Governor '{}' not available. Available: {:?}",
                        gov,
                        available
                    ));
                }
                Ok(())
            }
            Optimization::TransparentHugePages(_) => {
                // Check if THP is supported
                let path = std::path::Path::new("/sys/kernel/mm/transparent_hugepage/enabled");
                if !path.exists() {
                    return Err(anyhow!(
                        "Transparent huge pages not supported on this system"
                    ));
                }
                Ok(())
            }
            Optimization::NetworkTuning => Ok(()),
            Optimization::KernelParam(key, value) => {
                // Basic validation - check if key exists
                if crate::system_apply::sysctl_get(key).is_err() {
                    return Err(anyhow!("Kernel parameter '{}' not found", key));
                }
                // Validate value is numeric for numeric parameters
                if (key.contains("max") || key.contains("size") || key.contains("timeout"))
                    && value.parse::<i64>().is_err()
                {
                    return Err(anyhow!("Invalid numeric value for {}: {}", key, value));
                }
                Ok(())
            }
            Optimization::IoScheduler(device, scheduler) => {
                let path = format!("/sys/block/{}/queue/scheduler", device);
                if !std::path::Path::new(&path).exists() {
                    return Err(anyhow!("Device '{}' not found", device));
                }
                // Check if scheduler is available
                let content = std::fs::read_to_string(&path)?;
                if !content.contains(scheduler) {
                    return Err(anyhow!(
                        "Scheduler '{}' not available for device {}",
                        scheduler,
                        device
                    ));
                }
                Ok(())
            }
            Optimization::ReadAhead(device, kb) => {
                if *kb == 0 || *kb > 65536 {
                    return Err(anyhow!("Read-ahead must be between 1 and 65536 KB"));
                }
                let path = format!("/sys/block/{}", device);
                if !std::path::Path::new(&path).exists() {
                    return Err(anyhow!("Device '{}' not found", device));
                }
                Ok(())
            }
        }
    }

    /// Apply optimization
    pub fn apply(&self) -> Result<()> {
        match self {
            Optimization::CpuGovernor(gov) => crate::system_apply::apply_cpu_governor(gov),
            Optimization::TransparentHugePages(enable) => {
                crate::system_apply::set_transparent_hugepages(*enable)
            }
            Optimization::NetworkTuning => crate::system_apply::apply_optimized_network_tuning(),
            Optimization::KernelParam(key, value) => crate::system_apply::sysctl_set(key, value),
            Optimization::IoScheduler(device, scheduler) => {
                crate::system_apply::set_io_scheduler(device, scheduler)
            }
            Optimization::ReadAhead(device, kb) => crate::system_apply::set_readahead(device, *kb),
        }
    }
}

impl fmt::Display for Optimization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Optimization profile
#[derive(Debug, Clone)]
pub struct OptimizationProfile {
    pub name: String,
    pub description: String,
    pub optimizations: Vec<Optimization>,
}

impl OptimizationProfile {
    /// Development machine profile
    pub fn development() -> Self {
        Self {
            name: "Development".to_string(),
            description: "Optimized for development workloads".to_string(),
            optimizations: vec![
                Optimization::CpuGovernor("performance".to_string()),
                Optimization::TransparentHugePages(true),
                Optimization::NetworkTuning,
                Optimization::KernelParam("vm.swappiness".to_string(), "10".to_string()),
                Optimization::KernelParam("fs.file-max".to_string(), "2097152".to_string()),
            ],
        }
    }

    /// Server profile
    pub fn server() -> Self {
        Self {
            name: "Server".to_string(),
            description: "Optimized for server workloads".to_string(),
            optimizations: vec![
                Optimization::CpuGovernor("performance".to_string()),
                Optimization::TransparentHugePages(true),
                Optimization::NetworkTuning,
                Optimization::KernelParam("vm.swappiness".to_string(), "1".to_string()),
                Optimization::KernelParam("vm.dirty_ratio".to_string(), "10".to_string()),
                Optimization::KernelParam("net.core.somaxconn".to_string(), "8192".to_string()),
                Optimization::KernelParam("fs.file-max".to_string(), "4194304".to_string()),
            ],
        }
    }

    /// Balanced profile
    pub fn balanced() -> Self {
        Self {
            name: "Balanced".to_string(),
            description: "Balanced performance and power consumption".to_string(),
            optimizations: vec![
                Optimization::CpuGovernor("schedutil".to_string()),
                Optimization::TransparentHugePages(true),
                Optimization::KernelParam("vm.swappiness".to_string(), "30".to_string()),
            ],
        }
    }

    /// Power saving profile
    pub fn powersave() -> Self {
        Self {
            name: "Power Save".to_string(),
            description: "Optimized for power efficiency".to_string(),
            optimizations: vec![
                Optimization::CpuGovernor("powersave".to_string()),
                Optimization::TransparentHugePages(false),
                Optimization::KernelParam("vm.swappiness".to_string(), "60".to_string()),
            ],
        }
    }

    /// Apply all optimizations in profile
    pub fn apply(&self) -> Result<()> {
        println!("Applying {} profile...\n", self.name);

        for opt in &self.optimizations {
            println!("  → {}", opt.description());
            if let Err(e) = opt.apply() {
                eprintln!("    ✗ Failed: {}", e);
            } else {
                println!("    ✓ Applied");
            }
        }

        println!("\n✓ Profile applied successfully!");
        Ok(())
    }
}

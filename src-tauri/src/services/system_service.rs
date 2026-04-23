// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sysinfo::{System, Disks};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: OsInfo,
    pub hostname: String,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub uptime: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsInfo {
    pub name: String,
    pub version: String,
    pub arch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub model: String,
    pub cores: usize,
    pub usage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total: u64,
    pub available: u64,
    pub used: u64,
    pub usage_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub mount: String,
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub usage_percent: f32,
}

pub struct SystemService;

impl SystemService {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Get system information
    pub fn get_system_info(&self) -> Result<SystemInfo> {
        info!("Getting system information");

        // Refresh system info
        let sys = System::new_all();

        // OS info
        let os = OsInfo {
            name: std::env::consts::OS.to_string(),
            version: os_info::get().version().to_string(),
            arch: std::env::consts::ARCH.to_string(),
        };

        // Hostname
        let hostname = gethostname::gethostname()
            .to_string_lossy()
            .to_string();

        // CPU info
        let cpu = CpuInfo {
            model: sys
                .physical_core_count()
                .map_or_else(|| "Unknown".to_string(), |n| format!("{} cores", n)),
            cores: sys.physical_core_count().unwrap_or(1),
            usage: sys.global_cpu_info().cpu_usage(),
        };

        // Memory info
        let total = sys.total_memory();
        let available = sys.available_memory();
        let used = total - available;
        let memory = MemoryInfo {
            total,
            available,
            used,
            usage_percent: if total > 0 {
                (used as f32 / total as f32) * 100.0
            } else {
                0.0
            },
        };

        // Disk info
        let disk_list = Disks::new_with_refreshed_list();
        let disks: Vec<DiskInfo> = disk_list
            .iter()
            .map(|disk| {
                let total = disk.total_space();
                let available = disk.available_space();
                let used = total - available;

                DiskInfo {
                    mount: disk
                        .mount_point()
                        .to_string_lossy()
                        .to_string(),
                    total,
                    used,
                    available,
                    usage_percent: if total > 0 {
                        (used as f32 / total as f32) * 100.0
                    } else {
                        0.0
                    },
                }
            })
            .collect();

        // Uptime - use default since sysinfo 0.30 removed System::uptime()
        let uptime = 0u64;

        Ok(SystemInfo {
            os,
            hostname,
            cpu,
            memory,
            disks,
            uptime,
        })
    }

    /// Launch an application
    #[cfg(target_os = "windows")]
    pub fn launch_app(&self, path: &str, args: &[String], working_dir: Option<&str>) -> Result<()> {
        info!("Launching app: {} with args: {:?}", path, args);

        let mut command = std::process::Command::new(path);

        if !args.is_empty() {
            command.args(args);
        }

        if let Some(dir) = working_dir {
            command.current_dir(dir);
        }

        command.spawn().map_err(|e| AppError::internal(format!("Failed to launch: {}", e)))?;

        Ok(())
    }

    /// Launch an application (Unix)
    #[cfg(unix)]
    pub fn launch_app(&self, path: &str, args: &[String], working_dir: Option<&str>) -> Result<()> {
        info!("Launching app: {} with args: {:?}", path, args);

        use std::os::unix::process::CommandExt;

        let mut command = std::process::Command::new(path);

        if !args.is_empty() {
            command.args(args);
        }

        if let Some(dir) = working_dir {
            command.current_dir(dir);
        }

        // Detach from parent process
        unsafe {
            command.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }

        command
            .spawn()
            .map_err(|e| AppError::internal(format!("Failed to launch: {}", e)))?;

        Ok(())
    }

    /// Take a screenshot (simplified - will need platform-specific implementation)
    #[cfg(target_os = "windows")]
    pub fn screenshot(&self, region: Option<(i32, i32, i32, i32)>) -> Result<Vec<u8>> {
        info!("Taking screenshot");

        // TODO: Implement Windows screenshot using screen-capture crate
        Err(AppError::internal("Screenshot not yet implemented"))
    }

    /// Take a screenshot (macOS)
    #[cfg(target_os = "macos")]
    pub fn screenshot(&self, region: Option<(i32, i32, i32, i32)>) -> Result<Vec<u8>> {
        info!("Taking screenshot");

        // TODO: Implement macOS screenshot
        Err(AppError::internal("Screenshot not yet implemented"))
    }

    /// Take a screenshot (Linux)
    #[cfg(target_os = "linux")]
    pub fn screenshot(&self, _region: Option<(i32, i32, i32, i32)>) -> Result<Vec<u8>> {
        info!("Taking screenshot");

        // TODO: Implement Linux screenshot
        Err(AppError::internal("Screenshot not yet implemented"))
    }

    /// Get list of windows (simplified)
    pub fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        info!("Listing windows");

        // TODO: Implement platform-specific window enumeration
        // This would require platform-specific APIs:
        // - Windows: EnumWindows
        // - macOS: CGWindowListCopyWindowInfo
        // - Linux: X11/Wayland

        Ok(Vec::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    pub class_name: String,
    pub process_id: u32,
    pub is_minimized: bool,
    pub is_maximized: bool,
    pub is_active: bool,
    pub rect: Rect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

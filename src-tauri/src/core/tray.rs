// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! System tray integration
//!
//! Provides system tray icon and menu for cross-platform desktop integration

use crate::error::Result;
use tauri::{AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem};

/// System tray handler
pub struct TrayHandler {
    app_handle: AppHandle,
}

impl TrayHandler {
    /// Create system tray with menu
    pub fn create() -> SystemTray {
        let quit = CustomMenuItem::new("quit".to_string(), "退出");
        let hide = CustomMenuItem::new("hide".to_string(), "隐藏窗口");
        let show = CustomMenuItem::new("show".to_string(), "显示窗口");
        let separator = SystemTrayMenuItem::Separator;
        let skills = CustomMenuItem::new("skills".to_string(), "技能管理");
        let settings = CustomMenuItem::new("settings".to_string(), "设置");
        let dashboard = CustomMenuItem::new("dashboard".to_string(), "仪表板");
        let status = CustomMenuItem::new("status".to_string(), "状态: 运行中");

        let menu = SystemTrayMenu::new()
            .add_item(dashboard)
            .add_item(skills)
            .add_item(settings)
            .add_native_item(separator)
            .add_item(show)
            .add_item(hide)
            .add_native_item(separator)
            .add_item(status)
            .add_native_item(separator)
            .add_item(quit);

        SystemTray::new().with_menu(menu)
    }

    /// Handle tray events
    pub fn on_tray_event(app: &AppHandle, event: SystemTrayEvent) {
        match event {
            SystemTrayEvent::LeftClick { .. } => {
                // Show/hide window on left click
                if let Some(window) = app.get_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
            SystemTrayEvent::RightClick { .. } => {
                // Show menu on right click (default behavior)
            }
            SystemTrayEvent::DoubleClick { .. } => {
                // Show window on double click
                if let Some(window) = app.get_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "quit" => {
                        // Exit application
                        std::process::exit(0);
                    }
                    "hide" => {
                        if let Some(window) = app.get_window("main") {
                            let _ = window.hide();
                        }
                    }
                    "show" => {
                        if let Some(window) = app.get_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "dashboard" => {
                        if let Some(window) = app.get_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            // Emit event to navigate to dashboard
                            let _ = window.emit("navigate", "dashboard");
                        }
                    }
                    "skills" => {
                        if let Some(window) = app.get_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.emit("navigate", "skills");
                        }
                    }
                    "settings" => {
                        if let Some(window) = app.get_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.emit("navigate", "settings");
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Update tray status text
    pub fn update_status(app: &AppHandle, status: &str) -> Result<()> {
        app.tray_handle()
            .get_item("status")
            .set_title(format!("状态: {}", status));
        Ok(())
    }

    /// Show tray notification
    pub fn show_notification(app: &AppHandle, title: &str, body: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            use tauri::api::notification::Notification;
            let _ = Notification::new(app)
                .title(title)
                .body(body)
                .show();
        }

        #[cfg(target_os = "macos")]
        {
            use tauri::api::notification::Notification;
            let _ = Notification::new(app)
                .title(title)
                .body(body)
                .show();
        }

        #[cfg(target_os = "linux")]
        {
            // Linux notifications are handled differently
            tracing::info!("Notification: {} - {}", title, body);
        }

        Ok(())
    }
}

/// Tray icon provider
pub struct TrayIcon;

impl TrayIcon {
    /// Get tray icon bytes
    pub fn get_icon() -> &'static [u8] {
        // Return a simple icon (would be actual icon data in production)
        include_bytes!("../../assets/tray-icon.png")
    }

    /// Get icon path for platform
    pub fn get_icon_path() -> String {
        #[cfg(target_os = "windows")]
        return "icons/tray-icon.ico".to_string();

        #[cfg(target_os = "macos")]
        return "icons/tray-icon.png".to_string();

        #[cfg(target_os = "linux")]
        return "icons/tray-icon.png".to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_menu_creation() {
        let tray = TrayHandler::create();
        // Just verify it doesn't panic
        assert_eq!(tray.id, "desktop-agent-tray");
    }
}

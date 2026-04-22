// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::config::Config;
use crate::core::state::AppState;
use crate::database::Database;
use crate::error::Result;
use crate::services::ServiceContainer;
use crate::skill::SkillEngine;
use std::sync::Arc;
use tauri::Manager;
use tauri::{App, AppHandle, Builder, SystemTray, SystemTrayEvent, SystemTrayMenu};
use tracing::{error, info};

pub struct Application {
    config: Config,
    db: Arc<Database>,
    services: Arc<ServiceContainer>,
    skill_engine: Arc<SkillEngine>,
}

impl Application {
    /// Create a new application instance
    pub async fn new() -> Result<Self> {
        // Load configuration
        let config = Config::load().await?;

        // Get database path
        let db_path = Config::database_path()?;

        // Initialize database
        let db = Arc::new(Database::new(&db_path).await?);

        // Initialize services
        let services = Arc::new(ServiceContainer::new(
            config.clone(),
            db.clone(),
        )?);

        // Get skills directory
        let skill_dir = Config::skill_dir()?;

        // Initialize skill engine
        let skill_engine = Arc::new(SkillEngine::new(
            skill_dir,
            services.clone(),
        )?);

        Ok(Self {
            config,
            db,
            services,
            skill_engine,
        })
    }

    /// Run the Tauri application
    pub async fn run(self) -> Result<()> {
        let services_clone = self.services.clone();
        let skill_engine_clone = self.skill_engine.clone();
        let db_clone = self.db.clone();

        // Build the application
        let result = Builder::default()
            .setup(move |app| {
                // Create app state
                let state = AppState::new(
                    db_clone.clone(),
                    services_clone.clone(),
                    skill_engine_clone.clone(),
                );

                // Register commands
                register_commands(app, state);

                Ok(())
            })
            .system_tray(self.create_system_tray())
            .on_system_tray_event(|app, event| {
                match event {
                    SystemTrayEvent::LeftClick { .. } => {
                        // Show/hide main window
                        let window = app.get_window("main").unwrap();
                        if window.is_visible().unwrap() {
                            window.hide().unwrap();
                        } else {
                            window.show().unwrap();
                            window.set_focus().unwrap();
                        }
                    }
                    SystemTrayEvent::MenuItemClick { id, .. } => {
                        match id.as_str() {
                            "show" => {
                                let window = app.get_window("main").unwrap();
                                window.show().unwrap();
                                window.set_focus().unwrap();
                            }
                            "hide" => {
                                let window = app.get_window("main").unwrap();
                                window.hide().unwrap();
                            }
                            "quit" => {
                                app.exit(0);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            })
            .invoke_handler(tauri::generate_handler![
                crate::core::commands::system::get_system_info,
                crate::core::commands::file::list_directory,
                crate::core::commands::file::search_files,
                crate::core::commands::skill::list_skills,
                crate::core::commands::skill::get_skill,
                crate::core::commands::skill::execute_skill,
                crate::core::commands::config::get_config,
                crate::core::commands::config::set_config,
            ])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");

        Ok(result)
    }

    /// Create system tray
    fn create_system_tray(&self) -> SystemTray {
        let quit = SystemTrayMenu::new()
            .add_item(CustomMenuItem::new("show", "显示"))
            .add_item(CustomMenuItem::new("hide", "隐藏"))
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_item(CustomMenuItem::new("quit", "退出"));

        SystemTray::new().with_menu(quit)
    }
}

use tauri::CustomMenuItem;
use tauri::SystemTrayMenuItem;

/// Register Tauri commands
fn register_commands(app: &mut App, state: AppState) {
    app.manage(state);
}

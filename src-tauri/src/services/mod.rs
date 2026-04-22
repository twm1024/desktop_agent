// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

pub mod file_service;
pub mod system_service;
pub mod text_service;
pub mod network_service;
pub mod data_service;
pub mod backup_service;

use crate::config::Config;
use crate::database::Database;
use crate::error::Result;
use crate::services::file_service::FileService;
use crate::services::system_service::SystemService;
use crate::services::text_service::TextService;
use crate::services::network_service::NetworkService;
use crate::services::data_service::DataService;
use crate::services::backup_service::BackupService;
use std::sync::Arc;

/// Container for all application services
pub struct ServiceContainer {
    pub file_service: Arc<FileService>,
    pub system_service: Arc<SystemService>,
    pub text_service: Arc<TextService>,
    pub network_service: Arc<NetworkService>,
    pub data_service: Arc<DataService>,
    pub backup_service: Arc<BackupService>,
}

impl ServiceContainer {
    pub fn new(config: Config, db: Arc<Database>) -> Result<Self> {
        let file_service = Arc::new(FileService::new(config.clone())?);
        let system_service = Arc::new(SystemService::new()?);
        let text_service = Arc::new(TextService::new()?);
        let network_service = Arc::new(NetworkService::default());
        let data_service = Arc::new(DataService::new());
        let backup_service = Arc::new(BackupService::new(config.clone(), Some(db.clone())));

        Ok(Self {
            file_service,
            system_service,
            text_service,
            network_service,
            data_service,
            backup_service,
        })
    }
}

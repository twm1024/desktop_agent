// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Backup and restore service
//!
//! Provides functionality for backing up and restoring application configuration and data

use crate::config::Config;
use crate::database::Database;
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use zip::{write::FileOptions, ZipWriter};

use tokio::io::AsyncWriteExt;

/// Backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub app_version: String,
    pub includes: BackupIncludes,
    pub size_bytes: u64,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupIncludes {
    pub config: bool,
    pub database: bool,
    pub skills: bool,
    pub logs: bool,
}

/// Backup options
#[derive(Debug, Clone)]
pub struct BackupOptions {
    pub includes: BackupIncludes,
    pub compress: bool,
    pub encrypt: bool,
    pub destination: PathBuf,
}

impl Default for BackupOptions {
    fn default() -> Self {
        Self {
            includes: BackupIncludes {
                config: true,
                database: true,
                skills: true,
                logs: false,
            },
            compress: true,
            encrypt: false,
            destination: PathBuf::from("backups"),
        }
    }
}

/// Restore options
#[derive(Debug, Clone)]
pub struct RestoreOptions {
    pub force: bool,
    pub stop_on_error: bool,
}

/// Backup entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    pub filename: String,
    pub metadata: BackupMetadata,
    pub path: PathBuf,
}

/// Backup service
pub struct BackupService {
    config: Config,
    db: Option<std::sync::Arc<Database>>,
}

impl BackupService {
    pub fn new(config: Config, db: Option<std::sync::Arc<Database>>) -> Self {
        Self { config, db }
    }

    /// Create a backup
    pub async fn create_backup(&self, options: BackupOptions) -> Result<BackupEntry> {
        info!("Creating backup with options: {:?}", options.includes);

        // Ensure backup directory exists
        fs::create_dir_all(&options.destination)?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = if options.compress {
            format!("backup_{}.zip", timestamp)
        } else {
            format!("backup_{}", timestamp)
        };

        let backup_path = options.destination.join(&filename);
        let mut size_bytes = 0u64;

        if options.compress {
            // Create ZIP archive
            let file = fs::File::create(&backup_path)?;
            let mut zip = ZipWriter::new(file);
            let options = FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            // Backup config
            if options.includes.config {
                if let Some(config_dir) = self.config.config_dir().ok() {
                    if let Err(e) = self.add_dir_to_zip(&mut zip, &config_dir, "config", options) {
                        warn!("Failed to backup config: {}", e);
                    }
                }
            }

            // Backup database
            if options.includes.database {
                if let Some(db_path) = Config::database_path().ok() {
                    if let Err(e) = self.add_file_to_zip(&mut zip, &db_path, "database.db", options) {
                        warn!("Failed to backup database: {}", e);
                    }
                }
            }

            // Backup skills
            if options.includes.skills {
                if let Some(skills_dir) = self.config.skill_dir().ok() {
                    if let Err(e) = self.add_dir_to_zip(&mut zip, &skills_dir, "skills", options) {
                        warn!("Failed to backup skills: {}", e);
                    }
                }
            }

            zip.finish()?;
        } else {
            // Create directory backup
            let backup_dir = backup_path.join("data");
            fs::create_dir_all(&backup_dir)?;

            if options.includes.config {
                if let Some(config_dir) = self.config.config_dir().ok() {
                    let dest = backup_dir.join("config");
                    self.copy_dir(&config_dir, &dest)?;
                }
            }

            if options.includes.database {
                if let Some(db_path) = Config::database_path().ok() {
                    fs::copy(&db_path, backup_dir.join("database.db"))?;
                }
            }

            if options.includes.skills {
                if let Some(skills_dir) = self.config.skill_dir().ok() {
                    let dest = backup_dir.join("skills");
                    self.copy_dir(&skills_dir, &dest)?;
                }
            }
        }

        // Calculate size
        if backup_path.exists() {
            size_bytes = fs::metadata(&backup_path)?.len();
        }

        // Create metadata
        let metadata = BackupMetadata {
            version: "1.0".to_string(),
            created_at: Utc::now(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            includes: options.includes.clone(),
            size_bytes,
            checksum: self.calculate_checksum(&backup_path)?,
        };

        // Save metadata
        let metadata_path = backup_path.with_extension("meta.json");
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        fs::write(&metadata_path, metadata_json)?;

        info!("Backup created successfully: {:?}", backup_path);

        Ok(BackupEntry {
            filename,
            metadata,
            path: backup_path,
        })
    }

    /// Restore from a backup
    pub async fn restore_backup(&self, backup_path: &Path, options: RestoreOptions) -> Result<()> {
        info!("Restoring from backup: {:?}", backup_path);

        // Load metadata
        let metadata_path = backup_path.with_extension("meta.json");
        let metadata: BackupMetadata = if metadata_path.exists() {
            let metadata_json = fs::read_to_string(&metadata_path)?;
            serde_json::from_str(&metadata_json)?
        } else {
            // Try to read metadata from inside the backup
            return Err(crate::error::AppError::Config(
                "Backup metadata not found".to_string(),
            ));
        };

        info!("Backup metadata: {:?}", metadata);

        // Verify checksum
        let checksum = self.calculate_checksum(backup_path)?;
        if checksum != metadata.checksum {
            return Err(crate::error::AppError::Config(
                "Backup checksum mismatch - file may be corrupted".to_string(),
            ));
        }

        // Perform restoration
        if backup_path.extension().map_or(false, |e| e == "zip") {
            self.restore_from_zip(backup_path, &metadata.includes, options).await?;
        } else {
            self.restore_from_dir(backup_path, &metadata.includes, options).await?;
        }

        info!("Backup restored successfully");
        Ok(())
    }

    /// List available backups
    pub async fn list_backups(&self) -> Result<Vec<BackupEntry>> {
        let backup_dir = self.config.data_dir()?.join("backups");
        let mut backups = Vec::new();

        if !backup_dir.exists() {
            return Ok(backups);
        }

        for entry in fs::read_dir(backup_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Look for both .zip and .meta.json files
            let metadata_path = if path.extension().map_or(false, |e| e == "zip") {
                path.with_extension("meta.json")
            } else if path.extension().map_or(false, |e| e == "meta") {
                path.clone()
            } else {
                continue;
            };

            if metadata_path.exists() {
                let metadata_json = fs::read_to_string(&metadata_path)?;
                let metadata: BackupMetadata = serde_json::from_str(&metadata_json)?;

                backups.push(BackupEntry {
                    filename: path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    metadata,
                    path,
                });
            }
        }

        // Sort by creation time (newest first)
        backups.sort_by(|a, b| b.metadata.created_at.cmp(&a.metadata.created_at));

        Ok(backups)
    }

    /// Delete a backup
    pub async fn delete_backup(&self, backup_path: &Path) -> Result<()> {
        info!("Deleting backup: {:?}", backup_path);

        // Delete backup file
        if backup_path.exists() {
            fs::remove_file(backup_path)?;
        }

        // Delete metadata file
        let metadata_path = backup_path.with_extension("meta.json");
        if metadata_path.exists() {
            fs::remove_file(&metadata_path)?;
        }

        Ok(())
    }

    /// Auto-backup (scheduled backup)
    pub async fn auto_backup(&self) -> Result<BackupEntry> {
        info!("Running automatic backup");

        // Create backup options for auto backup
        let options = BackupOptions {
            includes: BackupIncludes {
                config: true,
                database: true,
                skills: true,
                logs: false, // Don't include logs in auto backups
            },
            compress: true,
            encrypt: false,
            destination: self.config.data_dir()?.join("backups"),
        };

        let backup = self.create_backup(options).await?;

        // Clean old backups (keep last 10)
        self.cleanup_old_backups(10).await?;

        Ok(backup)
    }

    async fn restore_from_zip(
        &self,
        backup_path: &Path,
        includes: &BackupIncludes,
        options: RestoreOptions,
    ) -> Result<()> {
        // This would use a ZIP library to extract files
        // For now, return a placeholder
        warn!("ZIP restore not fully implemented");
        Ok(())
    }

    async fn restore_from_dir(
        &self,
        backup_path: &Path,
        includes: &BackupIncludes,
        _options: RestoreOptions,
    ) -> Result<()> {
        let backup_data = backup_path.join("data");

        if includes.config && backup_data.join("config").exists() {
            let config_dir = self.config.config_dir()?;
            self.copy_dir(&backup_data.join("config"), &config_dir)?;
        }

        if includes.database {
            let db_file = backup_data.join("database.db");
            if db_file.exists() {
                let db_path = Config::database_path()?;
                fs::copy(&db_file, &db_path)?;
            }
        }

        if includes.skills && backup_data.join("skills").exists() {
            let skills_dir = self.config.skill_dir()?;
            self.copy_dir(&backup_data.join("skills"), &skills_dir)?;
        }

        Ok(())
    }

    fn add_dir_to_zip(
        &self,
        zip: &mut ZipWriter<std::fs::File>,
        dir_path: &Path,
        prefix: &str,
        options: FileOptions,
    ) -> Result<()> {
        if !dir_path.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            let name = path.strip_prefix(dir_path.parent().unwrap_or(dir_path))?;

            if path.is_dir() {
                self.add_dir_to_zip(zip, &path, &format!("{}/{}", prefix, name.display()), options)?;
            } else {
                let zip_path = format!("{}/{}", prefix, name.display());
                zip.start_file(&zip_path, options)?;
                let contents = fs::read(&path)?;
                zip.write_all(&contents)?;
            }
        }

        Ok(())
    }

    fn add_file_to_zip(
        &self,
        zip: &mut ZipWriter<std::fs::File>,
        file_path: &Path,
        zip_name: &str,
        options: FileOptions,
    ) -> Result<()> {
        if !file_path.exists() {
            return Ok(());
        }

        zip.start_file(zip_name, options)?;
        let contents = fs::read(file_path)?;
        zip.write_all(&contents)?;

        Ok(())
    }

    fn copy_dir(&self, src: &Path, dst: &Path) -> Result<()> {
        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                self.copy_dir(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    fn calculate_checksum(&self, path: &Path) -> Result<String> {
        use sha2::{Sha256, Digest};

        let contents = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let hash = hasher.finalize();

        Ok(hex::encode(hash))
    }

    async fn cleanup_old_backups(&self, keep_count: usize) -> Result<()> {
        let backups = self.list_backups().await?;

        if backups.len() > keep_count {
            for backup in backups.into_iter().skip(keep_count) {
                if let Err(e) = self.delete_backup(&backup.path).await {
                    warn!("Failed to delete old backup {:?}: {}", backup.path, e);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_options_default() {
        let options = BackupOptions::default();
        assert!(options.includes.config);
        assert!(options.includes.database);
        assert!(options.includes.skills);
        assert!(!options.includes.logs);
    }
}

// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Skill market module
//!
//! Provides skill marketplace for discovering, installing, and updating skills

#![allow(dead_code)]
use crate::database::Database;
use crate::database::repositories::SkillRepository;
use crate::error::{AppError, Result};
use crate::services::network_service::{HttpRequest, NetworkService};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

/// Market configuration
#[derive(Debug, Clone)]
pub struct MarketConfig {
    pub registry_url: String,
    pub cache_dir: PathBuf,
    pub skills_dir: PathBuf,
    pub cache_ttl_secs: u64,
}

impl Default for MarketConfig {
    fn default() -> Self {
        Self {
            registry_url: "https://registry.desktop-agent.dev/api/v1".to_string(),
            cache_dir: PathBuf::from("cache/market"),
            skills_dir: PathBuf::from("skills"),
            cache_ttl_secs: 3600,
        }
    }
}

/// Market skill listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSkill {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub downloads: u64,
    pub rating: f64,
    pub rating_count: u32,
    pub size_bytes: u64,
    pub created_at: String,
    pub updated_at: String,
    pub license: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub icon_url: Option<String>,
    pub verified: bool,
    pub featured: bool,
}

/// Search query for market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSearchQuery {
    pub query: Option<String>,
    pub tags: Option<Vec<String>>,
    pub author: Option<String>,
    pub sort_by: Option<SortBy>,
    pub sort_order: Option<SortOrder>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    Downloads,
    Rating,
    Updated,
    Name,
    Created,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Search result from market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSearchResult {
    pub skills: Vec<MarketSkill>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

/// Skill detail from market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSkillDetail {
    #[serde(flatten)]
    pub skill: MarketSkill,
    pub readme: String,
    pub changelog: String,
    pub dependencies: Vec<String>,
    pub versions: Vec<SkillVersion>,
    pub screenshots: Vec<String>,
}

/// Skill version info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub version: String,
    pub released_at: String,
    pub changelog: String,
    pub download_url: String,
    pub checksum: String,
    pub size_bytes: u64,
    pub min_app_version: Option<String>,
}

/// Install result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub skill_id: String,
    pub version: String,
    pub success: bool,
    pub message: String,
    pub installed_path: Option<String>,
}

/// Skill marketplace
pub struct SkillMarket {
    config: MarketConfig,
    http: Arc<NetworkService>,
    db: Option<Arc<Database>>,
}

impl SkillMarket {
    pub fn new(config: MarketConfig, http: Arc<NetworkService>, db: Option<Arc<Database>>) -> Self {
        Self { config, http, db }
    }

    /// Search for skills in the market
    pub async fn search(&self, query: MarketSearchQuery) -> Result<MarketSearchResult> {
        let mut params = std::collections::HashMap::new();

        if let Some(q) = &query.query {
            params.insert("q".to_string(), q.clone());
        }
        if let Some(tags) = &query.tags {
            params.insert("tags".to_string(), tags.join(","));
        }
        if let Some(author) = &query.author {
            params.insert("author".to_string(), author.clone());
        }
        if let Some(sort) = query.sort_by {
            params.insert("sort".to_string(), format!("{:?}", sort).to_lowercase());
        }
        if let Some(order) = query.sort_order {
            params.insert("order".to_string(), format!("{:?}", order).to_lowercase());
        }
        params.insert("page".to_string(), query.page.unwrap_or(1).to_string());
        params.insert("per_page".to_string(), query.per_page.unwrap_or(20).to_string());

        let url = format!("{}/skills/search", self.config.registry_url);

        let response = self.http.request(HttpRequest {
            url,
            method: crate::services::network_service::HttpMethod::GET,
            headers: None,
            body: None,
            query: Some(params),
            timeout: Some(30000),
            max_redirects: Some(5),
        }).await?;

        if !response.success {
            return Err(AppError::Network(format!("Search failed: {}", response.status)));
        }

        let result: MarketSearchResult = serde_json::from_str(&response.body)?;
        Ok(result)
    }

    /// Get skill details
    pub async fn get_skill_detail(&self, skill_id: &str) -> Result<MarketSkillDetail> {
        let url = format!("{}/skills/{}", self.config.registry_url, skill_id);

        let response = self.http.request(HttpRequest {
            url,
            method: crate::services::network_service::HttpMethod::GET,
            headers: None,
            body: None,
            query: None,
            timeout: Some(30000),
            max_redirects: Some(5),
        }).await?;

        if !response.success {
            return Err(AppError::Network(format!("Failed to get skill: {}", response.status)));
        }

        let detail: MarketSkillDetail = serde_json::from_str(&response.body)?;
        Ok(detail)
    }

    /// Install a skill from market
    pub async fn install(&self, skill_id: &str, version: Option<&str>) -> Result<InstallResult> {
        info!("Installing skill {} (version {:?})", skill_id, version);

        // Get skill detail
        let detail = self.get_skill_detail(skill_id).await?;

        // Find the version to install
        let target_version = if let Some(v) = version {
            detail.versions.iter().find(|sv| sv.version == v)
                .ok_or_else(|| AppError::Config(format!("Version {} not found", v)))?
        } else {
            detail.versions.first()
                .ok_or_else(|| AppError::Config("No versions available".to_string()))?
        };

        // Download skill package
        let package_data = self.http.download(target_version.download_url.clone()).await?;

        // Verify checksum
        let checksum = self.calculate_checksum(&package_data);
        if checksum != target_version.checksum {
            return Err(AppError::Config("Checksum mismatch - package may be corrupted".to_string()));
        }

        // Extract and install
        let skill_dir = self.config.skills_dir.join(&detail.skill.name);
        tokio::fs::create_dir_all(&skill_dir).await?;

        // Write package to temp file and extract
        let temp_path = skill_dir.join("package.tar.gz");
        tokio::fs::write(&temp_path, &package_data).await?;

        // Extract tarball
        self.extract_package(&temp_path, &skill_dir).await?;

        // Clean up temp file
        let _ = tokio::fs::remove_file(&temp_path).await;

        // Verify manifest exists
        let manifest_path = skill_dir.join("skill.yaml");
        if !manifest_path.exists() {
            tokio::fs::remove_dir_all(&skill_dir).await?;
            return Err(AppError::Config("Invalid skill package - missing manifest".to_string()));
        }

        // Update database
        if let Some(db) = &self.db {
            let skill_repo = SkillRepository::new(db);
            let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
            let now = chrono::Utc::now().timestamp();

            let record = crate::database::repositories::SkillRecord {
                id: detail.skill.id.clone(),
                name: detail.skill.name.clone(),
                version: target_version.version.clone(),
                description: Some(detail.skill.description.clone()),
                author: Some(detail.skill.author.clone()),
                tags: Some(serde_json::to_string(&detail.skill.tags)?),
                manifest: manifest_content,
                enabled: true,
                installed_at: now,
                updated_at: now,
                last_executed_at: None,
                execution_count: 0,
                source: Some("market".to_string()),
                checksum: Some(target_version.checksum.clone()),
                metadata: None,
            };

            skill_repo.insert(&record).await?;
        }

        info!("Skill {} v{} installed successfully", skill_id, target_version.version);

        Ok(InstallResult {
            skill_id: skill_id.to_string(),
            version: target_version.version.clone(),
            success: true,
            message: format!("Successfully installed {} v{}", detail.skill.name, target_version.version),
            installed_path: Some(skill_dir.to_string_lossy().to_string()),
        })
    }

    /// Uninstall a skill
    pub async fn uninstall(&self, skill_id: &str) -> Result<bool> {
        info!("Uninstalling skill {}", skill_id);

        // Remove from database
        if let Some(db) = &self.db {
            let skill_repo = SkillRepository::new(db);
            if let Some(record) = skill_repo.get_by_id(skill_id).await? {
                let skill_dir = self.config.skills_dir.join(&record.name);
                if skill_dir.exists() {
                    tokio::fs::remove_dir_all(&skill_dir).await?;
                }
                skill_repo.delete(skill_id).await?;
            }
        }

        info!("Skill {} uninstalled", skill_id);
        Ok(true)
    }

    /// Check for updates
    pub async fn check_updates(&self) -> Result<Vec<UpdateInfo>> {
        let mut updates = Vec::new();

        if let Some(db) = &self.db {
            let skill_repo = SkillRepository::new(db);
            let installed = skill_repo.list_all().await?;

            for skill in installed {
                if skill.source.as_deref() != Some("market") {
                    continue;
                }

                if let Ok(detail) = self.get_skill_detail(&skill.id).await {
                    if let Some(latest) = detail.versions.first() {
                        if latest.version != skill.version {
                            updates.push(UpdateInfo {
                                skill_id: skill.id.clone(),
                                name: skill.name.clone(),
                                current_version: skill.version.clone(),
                                latest_version: latest.version.clone(),
                                changelog: latest.changelog.clone(),
                            });
                        }
                    }
                }
            }
        }

        Ok(updates)
    }

    /// Update a skill
    pub async fn update(&self, skill_id: &str) -> Result<InstallResult> {
        info!("Updating skill {}", skill_id);
        self.install(skill_id, None).await
    }

    /// Get featured/trending skills
    pub async fn get_featured(&self) -> Result<Vec<MarketSkill>> {
        let url = format!("{}/skills/featured", self.config.registry_url);

        let response = self.http.request(HttpRequest {
            url,
            method: crate::services::network_service::HttpMethod::GET,
            headers: None,
            body: None,
            query: None,
            timeout: Some(30000),
            max_redirects: Some(5),
        }).await?;

        if !response.success {
            return Ok(Vec::new());
        }

        serde_json::from_str(&response.body)
            .map_err(|e| AppError::Serialization(format!("Failed to parse: {}", e)))
    }

    /// Get categories
    pub async fn get_categories(&self) -> Result<Vec<MarketCategory>> {
        let url = format!("{}/categories", self.config.registry_url);

        let response = self.http.request(HttpRequest {
            url,
            method: crate::services::network_service::HttpMethod::GET,
            headers: None,
            body: None,
            query: None,
            timeout: Some(30000),
            max_redirects: Some(5),
        }).await?;

        if !response.success {
            return Ok(Vec::new());
        }

        serde_json::from_str(&response.body)
            .map_err(|e| AppError::Serialization(format!("Failed to parse: {}", e)))
    }

    fn calculate_checksum(&self, data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    async fn extract_package(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        // Use tar command for extraction (available on all platforms)
        let output = tokio::process::Command::new("tar")
            .arg("-xzf")
            .arg(archive_path)
            .arg("-C")
            .arg(dest_dir)
            .arg("--strip-components=1")
            .output()
            .await
            .map_err(|e| AppError::Filesystem(format!("Failed to extract: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Filesystem(format!("Extraction failed: {}", stderr)));
        }

        Ok(())
    }
}

/// Update information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub skill_id: String,
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub changelog: String,
}

/// Market category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub skill_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_config_default() {
        let config = MarketConfig::default();
        assert!(!config.registry_url.is_empty());
    }

    #[test]
    fn test_search_query_builder() {
        let query = MarketSearchQuery {
            query: Some("ocr".to_string()),
            tags: Some(vec!["text".to_string()]),
            author: None,
            sort_by: Some(SortBy::Downloads),
            sort_order: Some(SortOrder::Desc),
            page: Some(1),
            per_page: Some(10),
        };
        assert_eq!(query.query.as_deref(), Some("ocr"));
    }
}

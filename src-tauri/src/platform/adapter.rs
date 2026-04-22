// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Unified platform adapter for chat platforms
//!
//! Provides a unified interface for different chat platforms (Feishu, WeCom, DingTalk)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Platform type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlatformType {
    Feishu,
    WeCom,
    DingTalk,
}

impl PlatformType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlatformType::Feishu => "feishu",
            PlatformType::WeCom => "wecom",
            PlatformType::DingTalk => "dingtalk",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "feishu" | "lark" => Some(PlatformType::Feishu),
            "wecom" | "wechat" | "weixin" => Some(PlatformType::WeCom),
            "dingtalk" | "ding" => Some(PlatformType::DingTalk),
            _ => None,
        }
    }
}

/// User information from platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformUser {
    pub user_id: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub department: Option<String>,
    pub extra: HashMap<String, String>,
}

/// Message type from platform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PlatformMessage {
    Text { content: String },
    Image { url: String, alt: Option<String> },
    File { url: String, name: String, size: Option<u64> },
    Audio { url: String, duration: Option<u32> },
    Video { url: String, duration: Option<u32> },
    Post { title: String, content: String },
    Card { content: String },
    Unknown { raw_type: String, data: serde_json::Value },
}

/// Incoming event from platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformEvent {
    pub platform: PlatformType,
    pub event_type: String,
    pub user: PlatformUser,
    pub chat_id: String,
    pub message: Option<PlatformMessage>,
    pub timestamp: i64,
    pub extra: HashMap<String, serde_json::Value>,
}

/// Response message to platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformResponse {
    pub chat_id: String,
    pub message: ResponseMessage,
    pub options: ResponseOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ResponseMessage {
    Text { content: String },
    Markdown { content: String },
    Image { url: String },
    Card { elements: Vec<CardElement> },
    File { url: String, name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardElement {
    pub tag: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseOptions {
    pub at_user: Option<String>,
    pub at_all: bool,
    pub reply_in_thread: bool,
    pub persist: bool,
}

impl Default for ResponseOptions {
    fn default() -> Self {
        Self {
            at_user: None,
            at_all: false,
            reply_in_thread: false,
            persist: true,
        }
    }
}

/// Unified platform adapter trait
#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    /// Get platform type
    fn platform_type(&self) -> PlatformType;

    /// Parse incoming webhook event
    async fn parse_event(
        &self,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> Result<PlatformEvent, PlatformError>;

    /// Send response to platform
    async fn send_response(
        &self,
        response: &PlatformResponse,
    ) -> Result<(), PlatformError>;

    /// Verify webhook signature
    async fn verify_webhook(
        &self,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> Result<bool, PlatformError>;

    /// Get user information
    async fn get_user(&self, user_id: &str) -> Result<PlatformUser, PlatformError>;

    /// Get chat information
    async fn get_chat(&self, chat_id: &str) -> Result<ChatInfo, PlatformError>;

    /// Upload media file
    async fn upload_media(
        &self,
        file_path: &str,
        file_type: MediaType,
    ) -> Result<String, PlatformError>;

    /// Download media file
    async fn download_media(&self, url: &str) -> Result<Vec<u8>, PlatformError>;

    /// Check if adapter is healthy
    async fn health_check(&self) -> Result<(), PlatformError>;
}

/// Chat information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatInfo {
    pub chat_id: String,
    pub name: Option<String>,
    pub chat_type: ChatType,
    pub owner_id: Option<String>,
    pub member_count: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatType {
    Private,
    Group,
    Bot,
}

/// Media type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Image,
    Audio,
    Video,
    File,
}

/// Platform error
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Platform adapter registry
pub struct PlatformAdapterRegistry {
    adapters: HashMap<PlatformType, Box<dyn PlatformAdapter>>,
}

impl PlatformAdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    /// Register a platform adapter
    pub fn register(&mut self, adapter: Box<dyn PlatformAdapter>) {
        let platform_type = adapter.platform_type();
        self.adapters.insert(platform_type, adapter);
    }

    /// Get adapter by platform type
    pub fn get(&self, platform_type: PlatformType) -> Option<&dyn PlatformAdapter> {
        self.adapters.get(&platform_type).map(|a| a.as_ref())
    }

    /// Get all registered platforms
    pub fn platforms(&self) -> Vec<PlatformType> {
        self.adapters.keys().copied().collect()
    }

    /// Check if platform is registered
    pub fn has_platform(&self, platform_type: PlatformType) -> bool {
        self.adapters.contains_key(&platform_type)
    }
}

impl Default for PlatformAdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to convert platform event to unified format
pub fn standardize_event(
    platform: PlatformType,
    mut raw_data: HashMap<String, serde_json::Value>,
) -> Result<PlatformEvent, PlatformError> {
    // Extract common fields
    let user_id = raw_data
        .remove("user_id")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .ok_or_else(|| PlatformError::ParseError("Missing user_id".to_string()))?;

    let chat_id = raw_data
        .remove("chat_id")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .ok_or_else(|| PlatformError::ParseError("Missing chat_id".to_string()))?;

    let name = raw_data
        .remove("user_name")
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    let avatar = raw_data
        .remove("user_avatar")
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    let timestamp = raw_data
        .remove("timestamp")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp());

    let event_type = raw_data
        .remove("event_type")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "message".to_string());

    let message = raw_data
        .remove("message")
        .and_then(|v| serde_json::from_value(v).ok());

    let user = PlatformUser {
        user_id,
        name,
        avatar,
        email: None,
        mobile: None,
        department: None,
        extra: HashMap::new(),
    };

    Ok(PlatformEvent {
        platform,
        event_type,
        user,
        chat_id,
        message,
        timestamp,
        extra: raw_data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_type_from_str() {
        assert_eq!(PlatformType::from_str("feishu"), Some(PlatformType::Feishu));
        assert_eq!(PlatformType::from_str("wecom"), Some(PlatformType::WeCom));
        assert_eq!(PlatformType::from_str("dingtalk"), Some(PlatformType::DingTalk));
        assert_eq!(PlatformType::from_str("unknown"), None);
    }

    #[test]
    fn test_platform_type_as_str() {
        assert_eq!(PlatformType::Feishu.as_str(), "feishu");
        assert_eq!(PlatformType::WeCom.as_str(), "wecom");
        assert_eq!(PlatformType::DingTalk.as_str(), "dingtalk");
    }
}

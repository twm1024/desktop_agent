// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Skill error: {0}")]
    Skill(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Webhook error: {0}")]
    Webhook(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn skill(msg: impl Into<String>) -> Self {
        Self::Skill(msg.into())
    }

    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::PermissionDenied(msg.into())
    }

    pub fn security(msg: impl Into<String>) -> Self {
        Self::Security(msg.into())
    }

    pub fn webhook(msg: impl Into<String>) -> Self {
        Self::Webhook(msg.into())
    }

    pub fn platform(msg: impl Into<String>) -> Self {
        Self::Platform(msg.into())
    }

    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

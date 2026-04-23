#![allow(dead_code)]
// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use thiserror::Error;
use std::path;

pub type Result<T> = std::result::Result<T, AppError>;

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Filesystem error: {0}")]
    Filesystem(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Skill error: {0}")]
    Skill(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Network error: {0}")]
    Network(String),

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

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),
}

// From implementations for error conversion
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}

impl From<serde_yaml::Error> for AppError {
    fn from(e: serde_yaml::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        Self::Network(e.to_string())
    }
}

impl From<String> for AppError {
    fn from(e: String) -> Self {
        Self::Internal(e)
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<path::StripPrefixError> for AppError {
    fn from(e: path::StripPrefixError) -> Self {
        Self::Internal(e.to_string())
    }
}

impl AppError {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn io(msg: impl std::fmt::Display) -> Self {
        Self::Io(msg.to_string())
    }

    pub fn filesystem(msg: impl Into<String>) -> Self {
        Self::Filesystem(msg.into())
    }

    pub fn database(msg: impl Into<String>) -> Self {
        Self::Database(msg.into())
    }

    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::Serialization(msg.into())
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

    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(msg.into())
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

    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }

    pub fn rate_limit(msg: impl Into<String>) -> Self {
        Self::RateLimit(msg.into())
    }
}

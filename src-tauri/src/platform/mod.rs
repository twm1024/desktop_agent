// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Platform abstraction layer
//!
//! This module provides unified interfaces for different platforms:
//! - Chat platforms (Feishu, WeCom, DingTalk)
//! - File system operations
//! - Platform-specific features

pub mod adapter;
pub mod feishu;
pub mod wecom;
pub mod filesystem;

pub use adapter::PlatformType;

// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Security module
//!
//! This module contains all security-related components:
//! - Webhook signature verification
//! - Configuration encryption
//! - Rate limiting
//! - Replay attack protection
//! - Log sanitization
//! - RBAC (Role-Based Access Control)
//! - Skill permissions

pub mod webhook;
pub mod encryption;
pub mod rate_limiter;
pub mod sanitizer;
pub mod rbac;
pub mod audit;
pub mod input_validator;


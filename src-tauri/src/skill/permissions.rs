// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::error::{AppError, Result};
use crate::skill::manifest::{
    FilePermissionDecl, NetworkPermissionDecl, SkillPermissionDecl, SystemPermissionDecl,
};
use regex::Regex;
use std::path::Path;

/// Skill permissions validator
pub struct SkillPermissions {
    file_perms: Vec<FilePermission>,
    network_perms: Vec<NetworkPermission>,
    system_perms: Vec<SystemPermission>,
}

#[derive(Debug, Clone)]
pub struct FilePermission {
    pub path_pattern: String,
    pub access: FileAccess,
    pub recursive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAccess {
    Read,
    Write,
    ReadWrite,
}

impl FileAccess {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "readwrite" | "read_write" => Ok(Self::ReadWrite),
            _ => Err(AppError::invalid_input(format!("Invalid access type: {}", s))),
        }
    }

    pub fn can_read(&self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite)
    }

    pub fn can_write(&self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite)
    }
}

#[derive(Debug, Clone)]
pub struct NetworkPermission {
    pub domain_pattern: String,
    pub allowed: bool,
    pub ports: Vec<u16>,
}

#[derive(Debug, Clone)]
pub struct SystemPermission {
    pub action: SystemAction,
    pub allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemAction {
    ExecuteCommand,
    Screenshot,
    Clipboard,
    SystemInfo,
}

impl SystemAction {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "execute_command" => Ok(Self::ExecuteCommand),
            "screenshot" => Ok(Self::Screenshot),
            "clipboard" => Ok(Self::Clipboard),
            "system_info" => Ok(Self::SystemInfo),
            _ => Err(AppError::invalid_input(format!("Invalid system action: {}", s))),
        }
    }
}

impl SkillPermissions {
    /// Create permissions from declaration
    pub fn from_decl(decl: &SkillPermissionDecl) -> Result<Self> {
        let file_perms = decl
            .file
            .iter()
            .map(|f| {
                Ok(FilePermission {
                    path_pattern: f.path.clone(),
                    access: FileAccess::from_str(&f.access)?,
                    recursive: f.recursive,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let network_perms = decl
            .network
            .iter()
            .map(|n| NetworkPermission {
                domain_pattern: n.domain.clone(),
                allowed: n.allowed,
                ports: n.ports.clone(),
            })
            .collect();

        let system_perms = decl
            .system
            .iter()
            .map(|s| {
                Ok(SystemPermission {
                    action: SystemAction::from_str(&s.action)?,
                    allowed: s.allowed,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            file_perms,
            network_perms,
            system_perms,
        })
    }

    /// Check if file access is allowed
    pub fn check_file_access(&self, path: &Path, access: FileAccess) -> Result<()> {
        let path_str = path.to_string_lossy().to_string();

        for perm in &self.file_perms {
            if self.path_matches(&perm.path_pattern, &path_str) {
                match (&perm.access, access) {
                    (FileAccess::ReadWrite, _) => return Ok(()),
                    (FileAccess::Read, FileAccess::Read) => return Ok(()),
                    (FileAccess::Write, FileAccess::Write) => return Ok(()),
                    _ => continue,
                }
            }
        }

        // Default deny
        Err(AppError::permission_denied(format!(
            "File access denied for {:?} ({:?})",
            path, access
        )))
    }

    /// Check if network access is allowed
    pub fn check_network_access(&self, domain: &str, port: u16) -> Result<()> {
        for perm in &self.network_perms {
            if self.domain_matches(&perm.domain_pattern, domain) {
                if !perm.allowed {
                    return Err(AppError::permission_denied(format!(
                        "Network access denied for {}",
                        domain
                    )));
                }

                // Check port if specified
                if !perm.ports.is_empty() && !perm.ports.contains(&port) {
                    return Err(AppError::permission_denied(format!(
                        "Port {} not allowed for {}",
                        port, domain
                    )));
                }

                return Ok(());
            }
        }

        // Default deny
        Err(AppError::permission_denied(format!(
            "Network access denied for {}",
            domain
        )))
    }

    /// Check if system action is allowed
    pub fn check_system_action(&self, action: SystemAction) -> Result<()> {
        for perm in &self.system_perms {
            if perm.action == action {
                if !perm.allowed {
                    return Err(AppError::permission_denied(format!(
                        "System action denied: {:?}",
                        action
                    )));
                }
                return Ok(());
            }
        }

        // Default deny
        Err(AppError::permission_denied(format!(
            "System action denied: {:?}",
            action
        )))
    }

    /// Match path pattern
    fn path_matches(&self, pattern: &str, path: &str) -> bool {
        // Expand ~ to home directory
        let pattern = if pattern.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                format!("{}/{}", home.display(), &pattern[2..])
            } else {
                pattern.to_string()
            }
        } else {
            pattern.to_string()
        };

        // Convert glob to regex
        let regex_pattern = glob_to_regex(&pattern);
        if let Ok(regex) = Regex::new(&regex_pattern) {
            regex.is_match(path)
        } else {
            false
        }
    }

    /// Match domain pattern
    fn domain_matches(&self, pattern: &str, domain: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 1];
            return domain.starts_with(prefix);
        }

        pattern == domain
    }
}

/// Convert glob pattern to regex
fn glob_to_regex(pattern: &str) -> String {
    let mut regex = String::new();
    let mut chars = pattern.chars().peekable();

    regex.push('^');

    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    regex.push_str(".*");
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '?' => regex.push_str("[^/]"),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' => {
                regex.push('\\');
                regex.push(c);
            }
            _ => regex.push(c),
        }
    }

    regex.push('$');
    regex
}

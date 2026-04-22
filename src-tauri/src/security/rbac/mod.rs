// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Role-Based Access Control (RBAC) module
//!
//! Provides comprehensive permission management with role-based access control

use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Built-in roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemRole {
    /// Super administrator with all permissions
    Admin,
    /// Regular user with basic permissions
    User,
    /// Guest with read-only access
    Guest,
    /// Service account for system operations
    Service,
}

impl SystemRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SystemRole::Admin => "admin",
            SystemRole::User => "user",
            SystemRole::Guest => "guest",
            SystemRole::Service => "service",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "admin" | "administrator" | "root" => Some(SystemRole::Admin),
            "user" | "member" => Some(SystemRole::User),
            "guest" | "visitor" => Some(SystemRole::Guest),
            "service" | "system" | "bot" => Some(SystemRole::Service),
            _ => None,
        }
    }

    /// Get default permissions for this role
    pub fn default_permissions(&self) -> HashSet<Permission> {
        match self {
            SystemRole::Admin => {
                // Admin has all permissions
                Permission::all()
            }
            SystemRole::User => {
                // Regular user permissions
                [
                    // Skill execution
                    Permission::SkillExecute,
                    Permission::SkillList,
                    Permission::SkillView,
                    // File operations
                    Permission::FileRead,
                    Permission::FileWrite,
                    Permission::FileList,
                    // System operations
                    Permission::SystemInfo,
                    // Session management
                    Permission::SessionCreate,
                    Permission::SessionView,
                    Permission::SessionOwn,
                ]
                .into_iter()
                .collect()
            }
            SystemRole::Guest => {
                // Guest has limited read-only permissions
                [
                    Permission::SkillList,
                    Permission::SkillView,
                    Permission::SystemInfo,
                    Permission::SessionView,
                ]
                .into_iter()
                .collect()
            }
            SystemRole::Service => {
                // Service account permissions
                [
                    Permission::SkillExecute,
                    Permission::SkillList,
                    Permission::FileRead,
                    Permission::FileList,
                    Permission::SystemInfo,
                    Permission::SessionCreate,
                ]
                .into_iter()
                .collect()
            }
        }
    }
}

/// Individual permission
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    /// Resource category (e.g., "skill", "file", "system")
    pub resource: Resource,
    /// Action on the resource (e.g., "read", "write", "execute")
    pub action: Action,
}

impl Permission {
    /// Create a new permission
    pub fn new(resource: Resource, action: Action) -> Self {
        Self { resource, action }
    }

    /// Get permission as string (resource:action)
    pub fn as_str(&self) -> String {
        format!("{}:{}", self.resource.as_str(), self.action.as_str())
    }

    /// Parse permission from string
    pub fn from_str(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        let resource = Resource::from_str(parts[0])?;
        let action = Action::from_str(parts[1])?;
        Some(Self { resource, action })
    }

    /// All permissions (for admin)
    pub fn all() -> HashSet<Permission> {
        let mut perms = HashSet::new();
        for resource in Resource::all() {
            for action in Action::all() {
                perms.insert(Permission::new(resource, action));
            }
        }
        perms
    }

    // Skill permissions
    pub const SKILL_EXECUTE: Permission = Permission {
        resource: Resource::Skill,
        action: Action::Execute,
    };
    pub const SKILL_LIST: Permission = Permission {
        resource: Resource::Skill,
        action: Action::List,
    };
    pub const SKILL_VIEW: Permission = Permission {
        resource: Resource::Skill,
        action: Action::View,
    };
    pub const SKILL_MANAGE: Permission = Permission {
        resource: Resource::Skill,
        action: Action::Manage,
    };

    // File permissions
    pub const FILE_READ: Permission = Permission {
        resource: Resource::File,
        action: Action::Read,
    };
    pub const FILE_WRITE: Permission = Permission {
        resource: Resource::File,
        action: Action::Write,
    };
    pub const FILE_DELETE: Permission = Permission {
        resource: Resource::File,
        action: Action::Delete,
    };
    pub const FILE_LIST: Permission = Permission {
        resource: Resource::File,
        action: Action::List,
    };

    // System permissions
    pub const SYSTEM_INFO: Permission = Permission {
        resource: Resource::System,
        action: Action::View,
    };
    pub const SYSTEM_MANAGE: Permission = Permission {
        resource: Resource::System,
        action: Action::Manage,
    };

    // User permissions
    pub const USER_VIEW: Permission = Permission {
        resource: Resource::User,
        action: Action::View,
    };
    pub const USER_MANAGE: Permission = Permission {
        resource: Resource::User,
        action: Action::Manage,
    };

    // Session permissions
    pub const SESSION_CREATE: Permission = Permission {
        resource: Resource::Session,
        action: Action::Create,
    };
    pub const SESSION_VIEW: Permission = Permission {
        resource: Resource::Session,
        action: Action::View,
    };
    pub const SESSION_MANAGE: Permission = Permission {
        resource: Resource::Session,
        action: Action::Manage,
    };
    pub const SESSION_OWN: Permission = Permission {
        resource: Resource::Session,
        action: Action::Own,
    };

    // Log permissions
    pub const LOG_VIEW: Permission = Permission {
        resource: Resource::Log,
        action: Action::View,
    };
    pub const LOG_DELETE: Permission = Permission {
        resource: Resource::Log,
        action: Action::Delete,
    };

    // Config permissions
    pub const CONFIG_VIEW: Permission = Permission {
        resource: Resource::Config,
        action: Action::View,
    };
    pub const CONFIG_MANAGE: Permission = Permission {
        resource: Resource::Config,
        action: Action::Manage,
    };
}

/// Resource category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Resource {
    Skill,
    File,
    System,
    User,
    Session,
    Log,
    Config,
}

impl Resource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Resource::Skill => "skill",
            Resource::File => "file",
            Resource::System => "system",
            Resource::User => "user",
            Resource::Session => "session",
            Resource::Log => "log",
            Resource::Config => "config",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "skill" => Some(Resource::Skill),
            "file" => Some(Resource::File),
            "system" => Some(Resource::System),
            "user" => Some(Resource::User),
            "session" => Some(Resource::Session),
            "log" => Some(Resource::Log),
            "config" => Some(Resource::Config),
            _ => None,
        }
    }

    pub fn all() -> Vec<Resource> {
        vec![
            Resource::Skill,
            Resource::File,
            Resource::System,
            Resource::User,
            Resource::Session,
            Resource::Log,
            Resource::Config,
        ]
    }
}

/// Action type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Create,
    Read,
    Write,
    Delete,
    Execute,
    List,
    View,
    Manage,
    Own,
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Create => "create",
            Action::Read => "read",
            Action::Write => "write",
            Action::Delete => "delete",
            Action::Execute => "execute",
            Action::List => "list",
            Action::View => "view",
            Action::Manage => "manage",
            Action::Own => "own",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "create" => Some(Action::Create),
            "read" => Some(Action::Read),
            "write" => Some(Action::Write),
            "delete" => Some(Action::Delete),
            "execute" => Some(Action::Execute),
            "list" => Some(Action::List),
            "view" => Some(Action::View),
            "manage" => Some(Action::Manage),
            "own" => Some(Action::Own),
            _ => None,
        }
    }

    pub fn all() -> Vec<Action> {
        vec![
            Action::Create,
            Action::Read,
            Action::Write,
            Action::Delete,
            Action::Execute,
            Action::List,
            Action::View,
            Action::Manage,
            Action::Own,
        ]
    }
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: HashSet<Permission>,
    pub is_system: bool,
}

impl Role {
    pub fn new(id: String, name: String, permissions: HashSet<Permission>) -> Self {
        Self {
            id,
            name,
            description: None,
            permissions,
            is_system: false,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn system(mut self) -> Self {
        self.is_system = true;
        self
    }

    /// Create from system role
    pub fn from_system_role(system_role: SystemRole) -> Self {
        let (id, name) = match system_role {
            SystemRole::Admin => ("admin", "Administrator"),
            SystemRole::User => ("user", "User"),
            SystemRole::Guest => ("guest", "Guest"),
            SystemRole::Service => ("service", "Service Account"),
        };
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: Some(format!("System {} role", name)),
            permissions: system_role.default_permissions(),
            is_system: true,
        }
    }

    /// Check if role has permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    /// Add permission to role
    pub fn add_permission(&mut self, permission: Permission) {
        self.permissions.insert(permission);
    }

    /// Remove permission from role
    pub fn remove_permission(&mut self, permission: &Permission) {
        self.permissions.remove(permission);
    }
}

/// RBAC manager
pub struct RbacManager {
    roles: Arc<RwLock<HashMap<String, Role>>>,
    user_roles: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    user_permissions: Arc<RwLock<HashMap<String, HashSet<Permission>>>>,
}

impl RbacManager {
    pub fn new() -> Self {
        let mut roles = HashMap::new();

        // Initialize system roles
        for system_role in [SystemRole::Admin, SystemRole::User, SystemRole::Guest, SystemRole::Service] {
            let role = Role::from_system_role(system_role);
            roles.insert(role.id.clone(), role);
        }

        Self {
            roles: Arc::new(RwLock::new(roles)),
            user_roles: Arc::new(RwLock::new(HashMap::new())),
            user_permissions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a custom role
    pub async fn add_role(&self, role: Role) -> Result<()> {
        let mut roles = self.roles.write().await;
        if roles.contains_key(&role.id) {
            return Err(AppError::Config(format!("Role {} already exists", role.id)));
        }
        roles.insert(role.id.clone(), role);
        Ok(())
    }

    /// Get a role by ID
    pub async fn get_role(&self, role_id: &str) -> Option<Role> {
        let roles = self.roles.read().await;
        roles.get(role_id).cloned()
    }

    /// List all roles
    pub async fn list_roles(&self) -> Vec<Role> {
        let roles = self.roles.read().await;
        roles.values().cloned().collect()
    }

    /// Delete a custom role
    pub async fn delete_role(&self, role_id: &str) -> Result<()> {
        let mut roles = self.roles.write().await;
        let role = roles.get(role_id).ok_or_else(|| {
            AppError::Config(format!("Role {} not found", role_id))
        })?;

        if role.is_system {
            return Err(AppError::Config(
                "Cannot delete system role".to_string(),
            ));
        }

        roles.remove(role_id);
        Ok(())
    }

    /// Assign role to user
    pub async fn assign_role(&self, user_id: &str, role_id: &str) -> Result<()> {
        // Verify role exists
        {
            let roles = self.roles.read().await;
            if !roles.contains_key(role_id) {
                return Err(AppError::Config(format!("Role {} not found", role_id)));
            }
        }

        let mut user_roles = self.user_roles.write().await;
        user_roles
            .entry(user_id.to_string())
            .or_insert_with(HashSet::new)
            .insert(role_id.to_string());

        // Clear permission cache
        let mut user_permissions = self.user_permissions.write().await;
        user_permissions.remove(user_id);

        Ok(())
    }

    /// Revoke role from user
    pub async fn revoke_role(&self, user_id: &str, role_id: &str) -> Result<()> {
        let mut user_roles = self.user_roles.write().await;
        if let Some(roles) = user_roles.get_mut(user_id) {
            roles.remove(role_id);
        }

        // Clear permission cache
        let mut user_permissions = self.user_permissions.write().await;
        user_permissions.remove(user_id);

        Ok(())
    }

    /// Get user's roles
    pub async fn get_user_roles(&self, user_id: &str) -> Vec<String> {
        let user_roles = self.user_roles.read().await;
        user_roles
            .get(user_id)
            .map(|roles| roles.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Check if user has permission
    pub async fn has_permission(&self, user_id: &str, permission: &Permission) -> bool {
        // Check cache first
        {
            let user_permissions = self.user_permissions.read().await;
            if let Some(permissions) = user_permissions.get(user_id) {
                return permissions.contains(permission);
            }
        }

        // Build permission set
        let permissions = self.build_user_permissions(user_id).await;

        // Cache it
        {
            let mut user_permissions = self.user_permissions.write().await;
            user_permissions.insert(user_id.to_string(), permissions);
        }

        // Check again
        let user_permissions = self.user_permissions.read().await;
        user_permissions
            .get(user_id)
            .map(|perms| perms.contains(permission))
            .unwrap_or(false)
    }

    /// Check if user has any of the specified permissions
    pub async fn has_any_permission(&self, user_id: &str, permissions: &[Permission]) -> bool {
        for permission in permissions {
            if self.has_permission(user_id, permission).await {
                return true;
            }
        }
        false
    }

    /// Check if user has all of the specified permissions
    pub async fn has_all_permissions(&self, user_id: &str, permissions: &[Permission]) -> bool {
        for permission in permissions {
            if !self.has_permission(user_id, permission).await {
                return false;
            }
        }
        true
    }

    /// Get user's permissions
    pub async fn get_user_permissions(&self, user_id: &str) -> HashSet<Permission> {
        // Check cache first
        {
            let user_permissions = self.user_permissions.read().await;
            if let Some(permissions) = user_permissions.get(user_id) {
                return permissions.clone();
            }
        }

        // Build and cache
        let permissions = self.build_user_permissions(user_id).await;
        let mut user_permissions = self.user_permissions.write().await;
        user_permissions.insert(user_id.to_string(), permissions.clone());
        permissions
    }

    /// Build user's permission set from roles
    async fn build_user_permissions(&self, user_id: &str) -> HashSet<Permission> {
        let user_roles = self.user_roles.read().await;
        let role_ids = user_roles.get(user_id);

        if role_ids.is_none() || role_ids.unwrap().is_empty() {
            // Default to guest role if no roles assigned
            return SystemRole::Guest.default_permissions();
        }

        let roles = self.roles.read().await;
        let mut permissions = HashSet::new();

        for role_id in role_ids.unwrap() {
            if let Some(role) = roles.get(role_id) {
                permissions.extend(role.permissions.iter().cloned());
            }
        }

        permissions
    }

    /// Clear permission cache for a user
    pub async fn clear_cache(&self, user_id: &str) {
        let mut user_permissions = self.user_permissions.write().await;
        user_permissions.remove(user_id);
    }

    /// Clear all permission cache
    pub async fn clear_all_cache(&self) {
        let mut user_permissions = self.user_permissions.write().await;
        user_permissions.clear();
    }
}

impl Default for RbacManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_role_permissions() {
        let admin_perms = SystemRole::Admin.default_permissions();
        assert!(admin_perms.contains(&Permission::SKILL_MANAGE));
        assert!(admin_perms.contains(&Permission::CONFIG_MANAGE));
    }

    #[tokio::test]
    async fn test_rbac_manager() {
        let rbac = RbacManager::new();

        // Assign admin role
        rbac.assign_role("user1", "admin").await.unwrap();

        // Check permissions
        assert!(rbac
            .has_permission("user1", &Permission::SYSTEM_MANAGE)
            .await);
        assert!(rbac
            .has_permission("user1", &Permission::SKILL_EXECUTE)
            .await);
    }

    #[tokio::test]
    async fn test_custom_role() {
        let rbac = RbacManager::new();

        let mut permissions = HashSet::new();
        permissions.insert(Permission::FILE_READ);
        permissions.insert(Permission::FILE_LIST);

        let role = Role::new("reader".to_string(), "Reader".to_string(), permissions);
        rbac.add_role(role).await.unwrap();

        rbac.assign_role("user2", "reader").await.unwrap();

        assert!(rbac.has_permission("user2", &Permission::FILE_READ).await);
        assert!(!rbac.has_permission("user2", &Permission::FILE_WRITE).await);
    }
}

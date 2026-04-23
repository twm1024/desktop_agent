//! Cross-platform file system abstraction layer
//!
//! Provides a unified interface for file operations across Windows, macOS, and Linux
//! with proper permission checking and security measures.

#![allow(dead_code)]
use std::path::{Path, PathBuf};
use crate::error::Result;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// Platform-specific file system information
#[derive(Debug, Clone)]
pub struct FileSystemInfo {
    pub file_system: String,
    pub mount_point: PathBuf,
    pub total_space: u64,
    pub available_space: u64,
    pub is_removable: bool,
}

/// File metadata with cross-platform support
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub is_readonly: bool,
    pub created: Option<std::time::SystemTime>,
    pub modified: Option<std::time::SystemTime>,
    pub accessed: Option<std::time::SystemTime>,
    pub permissions: FilePermissions,
}

/// Cross-platform file permissions
#[derive(Debug, Clone)]
pub struct FilePermissions {
    pub owner_read: bool,
    pub owner_write: bool,
    pub owner_execute: bool,
    pub group_read: bool,
    pub group_write: bool,
    pub group_execute: bool,
    pub other_read: bool,
    pub other_write: bool,
    pub other_execute: bool,
}

/// File system access level for security
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAccess {
    Read,
    Write,
    Execute,
    Delete,
    Metadata,
    All,
}

/// Abstract file system interface
pub trait AbstractFileSystem {
    /// Get information about a file
    fn get_metadata(&self, path: &Path) -> Result<FileMetadata>;

    /// Check if a path exists
    fn exists(&self, path: &Path) -> Result<bool>;

    /// Check if a path is a directory
    fn is_directory(&self, path: &Path) -> Result<bool>;

    /// Check if a path is a file
    fn is_file(&self, path: &Path) -> Result<bool>;

    /// List contents of a directory
    fn list_directory(&self, path: &Path, include_hidden: bool) -> Result<Vec<PathBuf>>;

    /// Create a directory
    fn create_directory(&self, path: &Path) -> Result<()>;

    /// Create a directory recursively
    fn create_directory_all(&self, path: &Path) -> Result<()>;

    /// Delete a file
    fn delete_file(&self, path: &Path) -> Result<()>;

    /// Delete an empty directory
    fn delete_directory(&self, path: &Path) -> Result<()>;

    /// Delete a directory and its contents recursively
    fn delete_directory_all(&self, path: &Path) -> Result<()>;

    /// Copy a file
    fn copy_file(&self, src: &Path, dst: &Path) -> Result<()>;

    /// Move or rename a file
    fn move_file(&self, src: &Path, dst: &Path) -> Result<()>;

    /// Check if access is permitted
    fn check_access(&self, path: &Path, access: FileAccess) -> Result<bool>;

    /// Get file system info for the volume containing the path
    fn get_volume_info(&self, path: &Path) -> Result<FileSystemInfo>;

    /// Resolve symlinks
    fn canonicalize(&self, path: &Path) -> Result<PathBuf>;

    /// Get home directory
    fn home_dir(&self) -> Result<PathBuf>;

    /// Get temp directory
    fn temp_dir(&self) -> Result<PathBuf>;

    /// Get data directory (platform-specific app data location)
    fn data_dir(&self) -> Result<PathBuf>;

    /// Get config directory
    fn config_dir(&self) -> Result<PathBuf>;

    /// Get cache directory
    fn cache_dir(&self) -> Result<PathBuf>;
}

/// Default implementation of abstract file system
pub struct DefaultFileSystem;

impl DefaultFileSystem {
    pub fn new() -> Self {
        Self
    }
}

impl AbstractFileSystem for DefaultFileSystem {
    fn get_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let metadata = std::fs::metadata(path)
            .map_err(|e| crate::error::AppError::Filesystem(format!("Failed to get metadata: {}", e)))?;

        let file_type = metadata.file_type();
        let is_symlink = file_type.is_symlink();

        // For symlinks, we might want to follow them or not
        let (is_file, is_dir) = if is_symlink {
            if let Ok(target_meta) = std::fs::metadata(path) {
                let target_type = target_meta.file_type();
                (target_type.is_file(), target_type.is_dir())
            } else {
                (false, false)
            }
        } else {
            (file_type.is_file(), file_type.is_dir())
        };

        let is_hidden = is_hidden_path(path);
        let is_readonly = metadata.permissions().readonly();

        let permissions = FilePermissions::from_std(&metadata.permissions());

        Ok(FileMetadata {
            path: path.to_path_buf(),
            size: metadata.len(),
            is_file,
            is_dir,
            is_symlink,
            is_hidden,
            is_readonly,
            created: metadata.created().ok(),
            modified: metadata.modified().ok(),
            accessed: metadata.accessed().ok(),
            permissions,
        })
    }

    fn exists(&self, path: &Path) -> Result<bool> {
        Ok(path.exists())
    }

    fn is_directory(&self, path: &Path) -> Result<bool> {
        Ok(path.is_dir())
    }

    fn is_file(&self, path: &Path) -> Result<bool> {
        Ok(path.is_file())
    }

    fn list_directory(&self, path: &Path, include_hidden: bool) -> Result<Vec<PathBuf>> {
        let entries = std::fs::read_dir(path)
            .map_err(|e| crate::error::AppError::Filesystem(format!("Failed to read directory: {}", e)))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                if include_hidden {
                    true
                } else {
                    !is_hidden_path(&entry.path())
                }
            })
            .map(|entry| entry.path())
            .collect();

        Ok(entries)
    }

    fn create_directory(&self, path: &Path) -> Result<()> {
        std::fs::create_dir(path)
            .map_err(|e| crate::error::AppError::Filesystem(format!("Failed to create directory: {}", e)))
    }

    fn create_directory_all(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(path)
            .map_err(|e| crate::error::AppError::Filesystem(format!("Failed to create directory: {}", e)))
    }

    fn delete_file(&self, path: &Path) -> Result<()> {
        std::fs::remove_file(path)
            .map_err(|e| crate::error::AppError::Filesystem(format!("Failed to delete file: {}", e)))
    }

    fn delete_directory(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir(path)
            .map_err(|e| crate::error::AppError::Filesystem(format!("Failed to delete directory: {}", e)))
    }

    fn delete_directory_all(&self, path: &Path) -> Result<()> {
        std::fs::remove_dir_all(path)
            .map_err(|e| crate::error::AppError::Filesystem(format!("Failed to delete directory: {}", e)))
    }

    fn copy_file(&self, src: &Path, dst: &Path) -> Result<()> {
        std::fs::copy(src, dst)
            .map_err(|e| crate::error::AppError::Filesystem(format!("Failed to copy file: {}", e)))?;
        Ok(())
    }

    fn move_file(&self, src: &Path, dst: &Path) -> Result<()> {
        std::fs::rename(src, dst)
            .map_err(|e| crate::error::AppError::Filesystem(format!("Failed to move file: {}", e)))
    }

    fn check_access(&self, path: &Path, access: FileAccess) -> Result<bool> {
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return Ok(false),
        };

        match access {
            FileAccess::Read | FileAccess::Metadata => {
                // On Unix, we can check read permission
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = metadata.permissions().mode();
                    Ok(mode & 0o444 != 0)
                }
                #[cfg(windows)]
                {
                    Ok(!metadata.permissions().readonly())
                }
            }
            FileAccess::Write => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = metadata.permissions().mode();
                    Ok(mode & 0o222 != 0)
                }
                #[cfg(windows)]
                {
                    Ok(!metadata.permissions().readonly())
                }
            }
            FileAccess::Execute => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = metadata.permissions().mode();
                    Ok(mode & 0o111 != 0)
                }
                #[cfg(windows)]
                {
                    // On Windows, execute is always true for files that can be executed
                    Ok(true)
                }
            }
            FileAccess::Delete => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = metadata.permissions().mode();
                    // Parent must be writable
                    if let Some(parent) = path.parent() {
                        if let Ok(parent_meta) = std::fs::metadata(parent) {
                            let parent_mode = parent_meta.permissions().mode();
                            return Ok(parent_mode & 0o222 != 0);
                        }
                    }
                    Ok(mode & 0o222 != 0)
                }
                #[cfg(windows)]
                {
                    Ok(!metadata.permissions().readonly())
                }
            }
            FileAccess::All => Ok(true),
        }
    }

    fn get_volume_info(&self, path: &Path) -> Result<FileSystemInfo> {
        // Get the absolute path
        let abs_path = path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf());

        // Find the root/mount point
        let mount_point = find_mount_point(&abs_path);

        #[cfg(unix)]
        {
            let statvfs = nix::sys::statvfs::statvfs(&mount_point)
                .map_err(|e| crate::error::AppError::Filesystem(format!("Failed to get volume info: {}", e)))?;

            Ok(FileSystemInfo {
                file_system: mount_point.display().to_string(),
                mount_point,
                total_space: statvfs.blocks() as u64 * statvfs.fragment_size() as u64,
                available_space: statvfs.blocks_available() as u64 * statvfs.fragment_size() as u64,
                is_removable: false, // Would need udev or similar to detect this
            })
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            let metadata = std::fs::metadata(&mount_point)
                .map_err(|e| crate::error::AppError::Filesystem(format!("Failed to get volume info: {}", e)))?;

            Ok(FileSystemInfo {
                file_system: mount_point.display().to_string(),
                mount_point,
                total_space: metadata.volume_capacity(),
                available_space: metadata.volume_available_capacity(),
                is_removable: false, // Would need Windows API to detect this
            })
        }

        #[cfg(not(any(unix, windows)))]
        {
            Ok(FileSystemInfo {
                file_system: mount_point.display().to_string(),
                mount_point,
                total_space: 0,
                available_space: 0,
                is_removable: false,
            })
        }
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        path.canonicalize()
            .map_err(|e| crate::error::AppError::Filesystem(format!("Failed to canonicalize path: {}", e)))
    }

    fn home_dir(&self) -> Result<PathBuf> {
        dirs::home_dir()
            .ok_or_else(|| crate::error::AppError::Filesystem("Could not determine home directory".to_string()))
    }

    fn temp_dir(&self) -> Result<PathBuf> {
        std::env::temp_dir()
            .into_os_string()
            .into_string()
            .map(PathBuf::from)
            .map_err(|_| crate::error::AppError::Filesystem("Invalid temp directory path".to_string()))
    }

    fn data_dir(&self) -> Result<PathBuf> {
        dirs::data_local_dir()
            .ok_or_else(|| crate::error::AppError::Filesystem("Could not determine data directory".to_string()))
    }

    fn config_dir(&self) -> Result<PathBuf> {
        dirs::config_dir()
            .ok_or_else(|| crate::error::AppError::Filesystem("Could not determine config directory".to_string()))
    }

    fn cache_dir(&self) -> Result<PathBuf> {
        dirs::cache_dir()
            .ok_or_else(|| crate::error::AppError::Filesystem("Could not determine cache directory".to_string()))
    }
}

impl FilePermissions {
    #[cfg(unix)]
    fn from_std(perm: &std::fs::Permissions) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let mode = perm.mode();

        FilePermissions {
            owner_read: mode & 0o400 != 0,
            owner_write: mode & 0o200 != 0,
            owner_execute: mode & 0o100 != 0,
            group_read: mode & 0o040 != 0,
            group_write: mode & 0o020 != 0,
            group_execute: mode & 0o010 != 0,
            other_read: mode & 0o004 != 0,
            other_write: mode & 0o002 != 0,
            other_execute: mode & 0o001 != 0,
        }
    }

    #[cfg(windows)]
    fn from_std(_perm: &std::fs::Permissions) -> Self {
        // Windows doesn't have the same permission model, so we use defaults
        FilePermissions {
            owner_read: true,
            owner_write: !_perm.readonly(),
            owner_execute: true,
            group_read: true,
            group_write: !_perm.readonly(),
            group_execute: true,
            other_read: true,
            other_write: false,
            other_execute: true,
        }
    }
}

/// Check if a path is hidden (platform-specific)
fn is_hidden_path(path: &Path) -> bool {
    if let Some(file_name) = path.file_name() {
        let name = file_name.to_string_lossy();

        #[cfg(unix)]
        {
            name.starts_with('.')
        }

        #[cfg(windows)]
        {
            // On Windows, check the hidden attribute
            if let Ok(metadata) = std::fs::metadata(path) {
                use std::os::windows::fs::MetadataExt;
                let attrs = metadata.file_attributes();
                attrs & 0x2 != 0 // FILE_ATTRIBUTE_HIDDEN
            } else {
                false
            }
        }

        #[cfg(not(any(unix, windows)))]
        {
            name.starts_with('.')
        }
    } else {
        false
    }
}

/// Find the mount point for a given path
fn find_mount_point(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();

    while let Some(parent) = current.parent() {
        if parent.as_os_str().is_empty() {
            break;
        }

        #[cfg(unix)]
        {
            if let Ok(stat) = std::fs::metadata(parent) {
                let _device = stat.dev();
                // Found the root
                if parent == &parent.canonicalize().unwrap_or(parent.to_path_buf()) {
                    return current;
                }
            }
        }

        current = parent.to_path_buf();
    }

    // If we couldn't find a mount point, return the root
    #[cfg(unix)]
    return PathBuf::from("/");

    #[cfg(windows)]
    {
        if let Some(drive) = path.ancestors().nth(1) {
            return drive.to_path_buf();
        }
        PathBuf::from("C:\\")
    }

    #[cfg(not(any(unix, windows)))]
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_exists() {
        let fs = DefaultFileSystem::new();
        assert!(fs.exists(Path::new(".")).unwrap());
    }

    #[test]
    fn test_home_dir() {
        let fs = DefaultFileSystem::new();
        let home = fs.home_dir().unwrap();
        assert!(home.is_absolute());
    }
}

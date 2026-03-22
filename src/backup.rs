//! Backup utilities for the bookmarks file.
//! 
//! Always creates a timestamped backup before modifying the original file.

use crate::error::{BookmarkError, Result};
use chrono::Local;
use std::path::{Path, PathBuf};

/// Create a backup of a file with a timestamp in the filename.
/// 
/// Returns the path to the backup file.
pub fn create_backup(original_path: &Path) -> Result<PathBuf> {
    if !original_path.exists() {
        return Err(BookmarkError::BackupFailed(format!(
            "Original file does not exist: {}",
            original_path.display()
        )));
    }

    // Generate backup filename with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let file_name = original_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Bookmarks");
    
    let backup_name = format!("{}.backup_{}", file_name, timestamp);
    
    let backup_path = original_path
        .parent()
        .map(|p| p.join(&backup_name))
        .unwrap_or_else(|| PathBuf::from(&backup_name));

    // Copy the file
    std::fs::copy(original_path, &backup_path).map_err(|e| {
        BookmarkError::BackupFailed(format!(
            "Failed to copy {} to {}: {}",
            original_path.display(),
            backup_path.display(),
            e
        ))
    })?;

    Ok(backup_path)
}

/// List all backup files for a given bookmarks file.
pub fn list_backups(bookmarks_path: &Path) -> Result<Vec<PathBuf>> {
    let parent = bookmarks_path.parent().ok_or_else(|| {
        BookmarkError::BackupFailed("Cannot determine backup directory".to_string())
    })?;

    let file_name = bookmarks_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Bookmarks");

    let backup_prefix = format!("{}.backup_", file_name);

    let mut backups: Vec<PathBuf> = std::fs::read_dir(parent)
        .map_err(|e| BookmarkError::BackupFailed(format!("Cannot read directory: {}", e)))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|name| name.starts_with(&backup_prefix))
                .unwrap_or(false)
        })
        .collect();

    // Sort by name (which includes timestamp, so newest last)
    backups.sort();
    
    Ok(backups)
}

/// Restore from a backup file.
pub fn restore_backup(backup_path: &Path, original_path: &Path) -> Result<()> {
    if !backup_path.exists() {
        return Err(BookmarkError::BackupFailed(format!(
            "Backup file does not exist: {}",
            backup_path.display()
        )));
    }

    std::fs::copy(backup_path, original_path).map_err(|e| {
        BookmarkError::BackupFailed(format!(
            "Failed to restore from {} to {}: {}",
            backup_path.display(),
            original_path.display(),
            e
        ))
    })?;

    Ok(())
}

/// Delete old backups, keeping only the most recent N.
pub fn prune_backups(bookmarks_path: &Path, keep: usize) -> Result<usize> {
    let mut backups = list_backups(bookmarks_path)?;
    
    if backups.len() <= keep {
        return Ok(0);
    }

    // Remove newest ones from the list (they're at the end after sorting)
    backups.truncate(backups.len() - keep);

    let mut deleted = 0;
    for backup in backups {
        if std::fs::remove_file(&backup).is_ok() {
            deleted += 1;
        }
    }

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_create_backup() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("Bookmarks");
        
        // Create a test file
        let mut file = std::fs::File::create(&original).unwrap();
        writeln!(file, "test content").unwrap();
        
        // Create backup
        let backup_path = create_backup(&original).unwrap();
        
        assert!(backup_path.exists());
        assert!(backup_path.file_name().unwrap().to_str().unwrap().contains("backup_"));
    }
}

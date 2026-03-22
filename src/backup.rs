//! Backup utilities for the bookmarks file.
//! 
//! Always creates a timestamped backup before modifying the original file.

use crate::error::{BookmarkError, Result};
use chrono::Local;
use std::path::{Path, PathBuf};

fn backup_dir_for(original_path: &Path) -> Result<PathBuf> {
    let parent = original_path.parent().ok_or_else(|| {
        BookmarkError::BackupFailed("Cannot determine backup directory".to_string())
    })?;
    Ok(parent.join("backups"))
}

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
    
    let backup_dir = backup_dir_for(original_path)?;
    std::fs::create_dir_all(&backup_dir).map_err(|e| {
        BookmarkError::BackupFailed(format!(
            "Failed to create backup directory {}: {}",
            backup_dir.display(),
            e
        ))
    })?;
    let backup_path = backup_dir.join(&backup_name);

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
    let backup_dir = backup_dir_for(bookmarks_path)?;

    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let file_name = bookmarks_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Bookmarks");

    let backup_prefix = format!("{}.backup_", file_name);

    let mut backups: Vec<PathBuf> = std::fs::read_dir(&backup_dir)
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

    let expected_backup_parent = backup_dir_for(original_path)?;

    let backup_parent = backup_path.parent().ok_or_else(|| {
        BookmarkError::BackupFailed("Cannot determine backup file directory".to_string())
    })?;

    if backup_parent != expected_backup_parent {
        return Err(BookmarkError::BackupFailed(format!(
            "Refusing to restore from outside backup directory: {}",
            backup_path.display()
        )));
    }

    let original_file_name = original_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| BookmarkError::BackupFailed("Invalid original file name".to_string()))?;
    let backup_file_name = backup_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| BookmarkError::BackupFailed("Invalid backup file name".to_string()))?;
    let expected_prefix = format!("{}.backup_", original_file_name);

    if !backup_file_name.starts_with(&expected_prefix) {
        return Err(BookmarkError::BackupFailed(format!(
            "Refusing to restore non-backup file: {}",
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
        std::fs::remove_file(&backup).map_err(|e| {
            BookmarkError::BackupFailed(format!(
                "Failed to remove old backup {}: {}",
                backup.display(),
                e
            ))
        })?;
        deleted += 1;
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

    #[test]
    fn test_restore_backup_success_for_valid_backup() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("Bookmarks");
        std::fs::write(&original, "current").unwrap();

        let backup_dir = dir.path().join("backups");
        std::fs::create_dir_all(&backup_dir).unwrap();
        let backup = backup_dir.join("Bookmarks.backup_20260101_000000");
        std::fs::write(&backup, "backup-data").unwrap();

        restore_backup(&backup, &original).unwrap();

        let restored = std::fs::read_to_string(&original).unwrap();
        assert_eq!(restored, "backup-data");
    }

    #[test]
    fn test_restore_backup_rejects_non_backup_file_name() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("Bookmarks");
        std::fs::write(&original, "current").unwrap();

        let backup_dir = dir.path().join("backups");
        std::fs::create_dir_all(&backup_dir).unwrap();
        let invalid = backup_dir.join("Bookmarks-not-a-backup");
        std::fs::write(&invalid, "bad-source").unwrap();

        let result = restore_backup(&invalid, &original);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Refusing to restore non-backup file"));
    }

    #[test]
    fn test_restore_backup_rejects_backup_outside_directory() {
        let original_dir = tempdir().unwrap();
        let backup_dir = tempdir().unwrap();

        let original = original_dir.path().join("Bookmarks");
        std::fs::write(&original, "current").unwrap();

        let outside_backup = backup_dir
            .path()
            .join("Bookmarks.backup_20260101_000000");
        std::fs::write(&outside_backup, "backup-data").unwrap();

        let result = restore_backup(&outside_backup, &original);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Refusing to restore from outside backup directory"));
    }
}

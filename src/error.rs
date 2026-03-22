//! Custom error types for the bookmark organizer.
//! 
//! Uses `thiserror` for ergonomic error definitions with automatic
//! `Display` and `Error` trait implementations.

use std::path::PathBuf;
use thiserror::Error;

/// All possible errors that can occur in the bookmark organizer.
#[derive(Error, Debug)]
pub enum BookmarkError {
    #[error("Failed to read bookmarks file at {path}: {source}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write bookmarks file at {path}: {source}")]
    FileWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse bookmarks JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Invalid bookmark structure: {0}")]
    InvalidStructure(String),

    #[error("Failed to parse URL '{url}': {source}")]
    UrlParse {
        url: String,
        #[source]
        source: url::ParseError,
    },

    #[error("HTTP request failed for '{url}': {message}")]
    HttpRequest { url: String, message: String },

    #[error("Backup failed: {0}")]
    BackupFailed(String),

    #[error("Could not determine Edge bookmarks path")]
    BookmarksPathNotFound,

    #[error("Operation cancelled by user")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

/// Result type alias using our custom error.
pub type Result<T> = std::result::Result<T, BookmarkError>;

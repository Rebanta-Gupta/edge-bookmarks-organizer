//! Edge Bookmarks Organizer Library
//! 
//! This library provides functionality for loading, analyzing, organizing,
//! and saving Microsoft Edge bookmarks.
//!
//! # Example
//!
//! ```no_run
//! use edge_bookmarks_organizer::{parser, duplicates, organizer};
//! use std::path::PathBuf;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let path = PathBuf::from("C:/path/to/Bookmarks");
//! let bookmarks_file = parser::load_bookmarks_file(&path)?;
//! let bookmarks = parser::parse_bookmarks(&bookmarks_file);
//! 
//! let domain_stats = organizer::get_domain_stats(&bookmarks);
//! let duplicate_stats = duplicates::get_duplicate_stats(&bookmarks);
//! # Ok(())
//! # }
//! ```

pub mod backup;
pub mod bookmark;
pub mod deadlinks;
pub mod duplicates;
pub mod embeddings;
pub mod error;
pub mod organizer;
pub mod parser;
pub mod rebuilder;

// Re-export commonly used types
pub use bookmark::{Bookmark, BookmarksFile, CheckedBookmark, LinkStatus};
pub use error::{BookmarkError, Result};

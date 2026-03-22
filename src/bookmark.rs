//! Core bookmark data structures.
//! 
//! Defines the internal representation of bookmarks and the raw JSON
//! structure used by Microsoft Edge.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A flattened bookmark entry extracted from Edge's nested structure.
/// This is our internal working representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    /// Unique identifier from Edge
    pub id: String,
    /// Display name of the bookmark
    pub name: String,
    /// The bookmark URL
    pub url: String,
    /// Normalized URL for comparison (lowercase host, no trailing slash)
    pub normalized_url: String,
    /// Extracted domain from the URL
    pub domain: String,
    /// Full folder path (e.g., "Bookmarks Bar/Tech/Rust")
    pub folder_path: String,
    /// Creation timestamp (microseconds since Windows epoch)
    pub date_added: Option<String>,
    /// Last modification timestamp
    pub date_last_used: Option<String>,
    /// Optional topic category (for topic-based grouping)
    pub topic: Option<String>,
}

/// Raw bookmark node as stored in Edge's JSON file.
/// Can be either a folder or a URL bookmark.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawBookmarkNode {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub children: Option<Vec<RawBookmarkNode>>,
    #[serde(default)]
    pub date_added: Option<String>,
    #[serde(default)]
    pub date_last_used: Option<String>,
    /// Preserve any additional fields Edge might add
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Root structure of Edge's Bookmarks JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarksFile {
    pub checksum: String,
    pub roots: BookmarkRoots,
    pub version: i32,
    /// Preserve additional root-level fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// The main bookmark roots (bookmark bar, other bookmarks, synced).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkRoots {
    pub bookmark_bar: RawBookmarkNode,
    pub other: RawBookmarkNode,
    #[serde(default)]
    pub synced: Option<RawBookmarkNode>,
    /// Preserve additional root folders
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Status of a bookmark after dead link checking.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LinkStatus {
    /// Link is reachable (2xx or 3xx status)
    Alive,
    /// Link returned an error status (4xx or 5xx)
    Dead { status_code: u16 },
    /// Connection failed (timeout, DNS error, etc.)
    Unreachable { reason: String },
    /// Not yet checked
    #[default]
    Unknown,
}

/// A bookmark with its link status after checking.
#[derive(Debug, Clone)]
pub struct CheckedBookmark {
    pub bookmark: Bookmark,
    pub status: LinkStatus,
}

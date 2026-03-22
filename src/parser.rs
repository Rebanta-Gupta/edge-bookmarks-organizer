//! Bookmark file parsing and flattening.
//! 
//! Handles reading Edge's JSON bookmark file and converting the nested
//! folder structure into a flat list of bookmark entries.

use crate::bookmark::{Bookmark, BookmarksFile, RawBookmarkNode};
use crate::error::{BookmarkError, Result};
use std::path::{Path, PathBuf};
use url::Url;

/// Get the default path to Edge's Bookmarks file.
/// 
/// On Windows, this is typically:
/// `%LocalAppData%\Microsoft\Edge\User Data\Default\Bookmarks`
pub fn get_default_bookmarks_path() -> Result<PathBuf> {
    // Try to get LocalAppData directory
    if let Some(local_app_data) = dirs::data_local_dir() {
        let path = local_app_data
            .join("Microsoft")
            .join("Edge")
            .join("User Data")
            .join("Default")
            .join("Bookmarks");
        
        if path.exists() {
            return Ok(path);
        }
    }

    // Fallback: try environment variable directly (Windows)
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let path = PathBuf::from(local_app_data)
            .join("Microsoft")
            .join("Edge")
            .join("User Data")
            .join("Default")
            .join("Bookmarks");
        
        if path.exists() {
            return Ok(path);
        }
    }

    Err(BookmarkError::BookmarksPathNotFound)
}

/// Load and parse the bookmarks file from disk.
pub fn load_bookmarks_file(path: &Path) -> Result<BookmarksFile> {
    let content = std::fs::read_to_string(path).map_err(|e| BookmarkError::FileRead {
        path: path.to_path_buf(),
        source: e,
    })?;

    let bookmarks: BookmarksFile = serde_json::from_str(&content)?;
    Ok(bookmarks)
}

/// Normalize a URL for duplicate detection.
/// 
/// - Converts host to lowercase
/// - Removes trailing slashes from path
/// - Removes default ports (80 for http, 443 for https)
/// - Sorts query parameters for consistency
pub fn normalize_url(url_str: &str) -> String {
    match Url::parse(url_str) {
        Ok(mut url) => {
            // Lowercase the host
            if let Some(host) = url.host_str() {
                let lower_host = host.to_lowercase();
                let _ = url.set_host(Some(&lower_host));
            }

            // Remove default ports
            if url.port() == Some(80) && url.scheme() == "http" {
                let _ = url.set_port(None);
            }
            if url.port() == Some(443) && url.scheme() == "https" {
                let _ = url.set_port(None);
            }

            // Get path and remove trailing slash (unless it's just "/")
            let path = url.path().to_string();
            let normalized_path = if path.len() > 1 && path.ends_with('/') {
                &path[..path.len() - 1]
            } else {
                &path
            };

            // Rebuild URL with normalized components
            format!(
                "{}://{}{}{}",
                url.scheme(),
                url.host_str().unwrap_or(""),
                normalized_path,
                url.query().map(|q| format!("?{}", q)).unwrap_or_default()
            )
        }
        Err(_) => url_str.to_lowercase(), // Fallback for invalid URLs
    }
}

/// Extract the domain from a URL.
pub fn extract_domain(url_str: &str) -> String {
    match Url::parse(url_str) {
        Ok(url) => url.host_str().unwrap_or("unknown").to_lowercase(),
        Err(_) => "unknown".to_string(),
    }
}

/// Recursively flatten a bookmark node tree into a list of bookmarks.
/// 
/// This walks through the nested folder structure and extracts all
/// URL bookmarks, recording their full folder path for later reconstruction.
fn flatten_node(node: &RawBookmarkNode, current_path: &str, bookmarks: &mut Vec<Bookmark>) {
    match node.node_type.as_str() {
        "url" => {
            // This is an actual bookmark
            if let Some(url) = &node.url {
                let bookmark = Bookmark {
                    id: node.id.clone(),
                    name: node.name.clone(),
                    url: url.clone(),
                    normalized_url: normalize_url(url),
                    domain: extract_domain(url),
                    folder_path: current_path.to_string(),
                    date_added: node.date_added.clone(),
                    date_last_used: node.date_last_used.clone(),
                    topic: None,
                };
                bookmarks.push(bookmark);
            }
        }
        "folder" => {
            // This is a folder - recurse into children
            if let Some(children) = &node.children {
                let new_path = if current_path.is_empty() {
                    node.name.clone()
                } else {
                    format!("{}/{}", current_path, node.name)
                };

                for child in children {
                    flatten_node(child, &new_path, bookmarks);
                }
            }
        }
        _ => {
            // Unknown type - skip
        }
    }
}

/// Flatten a root node while skipping the root wrapper folder name.
///
/// Edge stores bookmarks under wrapper roots like `bookmark_bar` / `other`.
/// We only want user-facing folder paths, so we flatten from root children.
fn flatten_root(node: &RawBookmarkNode, bookmarks: &mut Vec<Bookmark>) {
    match node.node_type.as_str() {
        "folder" => {
            if let Some(children) = &node.children {
                for child in children {
                    flatten_node(child, "", bookmarks);
                }
            }
        }
        "url" => flatten_node(node, "", bookmarks),
        _ => {}
    }
}

/// Parse a bookmarks file and return a flat list of all bookmarks.
pub fn parse_bookmarks(bookmarks_file: &BookmarksFile) -> Vec<Bookmark> {
    let mut bookmarks = Vec::new();

    // Process bookmark bar contents (skip root wrapper name)
    flatten_root(&bookmarks_file.roots.bookmark_bar, &mut bookmarks);

    // Process other bookmarks contents (skip root wrapper name)
    flatten_root(&bookmarks_file.roots.other, &mut bookmarks);

    // Process synced bookmarks if present
    if let Some(synced) = &bookmarks_file.roots.synced {
        flatten_root(synced, &mut bookmarks);
    }

    bookmarks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmark::{BookmarkRoots, BookmarksFile, RawBookmarkNode};
    use std::collections::HashMap;

    #[test]
    fn test_normalize_url() {
        assert_eq!(
            normalize_url("https://Example.COM/path/"),
            "https://example.com/path"
        );
        assert_eq!(
            normalize_url("http://test.com:80/page"),
            "http://test.com/page"
        );
        assert_eq!(
            normalize_url("https://test.com:443/"),
            "https://test.com/"
        );
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://www.example.com/path"), "www.example.com");
        assert_eq!(extract_domain("http://TEST.COM"), "test.com");
        assert_eq!(extract_domain("invalid-url"), "unknown");
    }

    #[test]
    fn test_parse_bookmarks_skips_root_wrapper_name() {
        let url_node = RawBookmarkNode {
            id: "10".to_string(),
            name: "Example".to_string(),
            node_type: "url".to_string(),
            url: Some("https://example.com".to_string()),
            children: None,
            date_added: None,
            date_last_used: None,
            extra: HashMap::new(),
        };

        let fav_folder = RawBookmarkNode {
            id: "5".to_string(),
            name: "Favorites bar".to_string(),
            node_type: "folder".to_string(),
            url: None,
            children: Some(vec![url_node]),
            date_added: None,
            date_last_used: None,
            extra: HashMap::new(),
        };

        let bookmark_bar = RawBookmarkNode {
            id: "1".to_string(),
            name: "Bookmarks Bar".to_string(),
            node_type: "folder".to_string(),
            url: None,
            children: Some(vec![fav_folder]),
            date_added: None,
            date_last_used: None,
            extra: HashMap::new(),
        };

        let other = RawBookmarkNode {
            id: "2".to_string(),
            name: "Other bookmarks".to_string(),
            node_type: "folder".to_string(),
            url: None,
            children: Some(vec![]),
            date_added: None,
            date_last_used: None,
            extra: HashMap::new(),
        };

        let file = BookmarksFile {
            checksum: "x".to_string(),
            roots: BookmarkRoots {
                bookmark_bar,
                other,
                synced: None,
                extra: HashMap::new(),
            },
            version: 1,
            extra: HashMap::new(),
        };

        let bookmarks = parse_bookmarks(&file);
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].folder_path, "Favorites bar");
    }
}

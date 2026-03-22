//! Bookmark structure rebuilding.
//! 
//! Reconstructs Edge's nested JSON bookmark structure from our flat
//! bookmark list, optionally reorganizing by domain or topic.

use crate::bookmark::{Bookmark, BookmarkRoots, BookmarksFile, RawBookmarkNode};
use crate::error::Result;
use std::collections::HashMap;

/// Strategy for organizing bookmarks when rebuilding.
#[derive(Debug, Clone, Copy)]
pub enum OrganizeStrategy {
    /// Keep original folder structure
    PreserveOriginal,
    /// Group by domain (creates domain-named folders)
    ByDomain,
    /// Group by root domain (simplifies subdomains)
    ByRootDomain,
    /// Group by topic (requires topic field to be populated)
    ByTopic,
}

/// Build a raw bookmark node for a single URL bookmark.
fn build_url_node(bookmark: &Bookmark, id: &str) -> RawBookmarkNode {
    RawBookmarkNode {
        id: id.to_string(),
        name: bookmark.name.clone(),
        node_type: "url".to_string(),
        url: Some(bookmark.url.clone()),
        children: None,
        date_added: bookmark.date_added.clone(),
        date_last_used: bookmark.date_last_used.clone(),
        extra: HashMap::new(),
    }
}

/// Build a folder node with children.
fn build_folder_node(name: &str, id: &str, children: Vec<RawBookmarkNode>) -> RawBookmarkNode {
    RawBookmarkNode {
        id: id.to_string(),
        name: name.to_string(),
        node_type: "folder".to_string(),
        url: None,
        children: Some(children),
        date_added: None,
        date_last_used: None,
        extra: HashMap::new(),
    }
}

/// ID generator for new nodes.
struct IdGenerator {
    next_id: u64,
}

impl IdGenerator {
    fn new(start: u64) -> Self {
        Self { next_id: start }
    }

    fn next(&mut self) -> String {
        let id = self.next_id;
        self.next_id += 1;
        id.to_string()
    }
}

/// Rebuild bookmarks organized by domain.
pub fn rebuild_by_domain(bookmarks: &[Bookmark]) -> RawBookmarkNode {
    let mut id_gen = IdGenerator::new(1);
    
    // Group by domain
    let mut domain_map: HashMap<String, Vec<&Bookmark>> = HashMap::new();
    for bookmark in bookmarks {
        domain_map
            .entry(bookmark.domain.clone())
            .or_default()
            .push(bookmark);
    }

    // Sort domains alphabetically
    let mut domains: Vec<_> = domain_map.keys().cloned().collect();
    domains.sort();

    // Build folder for each domain
    let children: Vec<RawBookmarkNode> = domains
        .into_iter()
        .map(|domain| {
            let domain_bookmarks = &domain_map[&domain];
            let bookmark_nodes: Vec<RawBookmarkNode> = domain_bookmarks
                .iter()
                .map(|b| build_url_node(b, &id_gen.next()))
                .collect();
            
            build_folder_node(&domain, &id_gen.next(), bookmark_nodes)
        })
        .collect();

    build_folder_node("Organized Bookmarks", &id_gen.next(), children)
}

/// Rebuild bookmarks preserving original folder structure.
pub fn rebuild_preserve_structure(bookmarks: &[Bookmark]) -> RawBookmarkNode {
    let mut id_gen = IdGenerator::new(1);
    
    // Group by folder path
    let mut folder_map: HashMap<String, Vec<&Bookmark>> = HashMap::new();
    for bookmark in bookmarks {
        folder_map
            .entry(bookmark.folder_path.clone())
            .or_default()
            .push(bookmark);
    }

    // Build nested structure
    fn build_nested(
        path: &str,
        folder_map: &HashMap<String, Vec<&Bookmark>>,
        all_paths: &[String],
        id_gen: &mut IdGenerator,
    ) -> Vec<RawBookmarkNode> {
        let mut nodes = Vec::new();

        // Add bookmarks directly in this folder
        if let Some(bookmarks) = folder_map.get(path) {
            for bookmark in bookmarks {
                nodes.push(build_url_node(bookmark, &id_gen.next()));
            }
        }

        // Find immediate child folders
        let prefix = if path.is_empty() {
            "".to_string()
        } else {
            format!("{}/", path)
        };

        let mut child_folders: Vec<String> = all_paths
            .iter()
            .filter(|p| {
                if path.is_empty() {
                    !p.contains('/')
                } else {
                    p.starts_with(&prefix) && !p[prefix.len()..].contains('/')
                }
            })
            .filter(|p| *p != path)
            .cloned()
            .collect();
        
        child_folders.sort();
        child_folders.dedup();

        for child_path in child_folders {
            let folder_name = if path.is_empty() {
                child_path.clone()
            } else {
                child_path[prefix.len()..].to_string()
            };

            let children = build_nested(&child_path, folder_map, all_paths, id_gen);
            if !children.is_empty() || folder_map.contains_key(&child_path) {
                nodes.push(build_folder_node(&folder_name, &id_gen.next(), children));
            }
        }

        nodes
    }

    let all_paths: Vec<String> = folder_map.keys().cloned().collect();
    let children = build_nested("", &folder_map, &all_paths, &mut id_gen);
    
    build_folder_node("Bookmarks Bar", &id_gen.next(), children)
}

/// Rebuild the complete bookmarks file with a new structure.
pub fn rebuild_bookmarks_file(
    original: &BookmarksFile,
    bookmarks: &[Bookmark],
    strategy: OrganizeStrategy,
) -> BookmarksFile {
    let organized = match strategy {
        OrganizeStrategy::ByDomain | OrganizeStrategy::ByRootDomain => {
            rebuild_by_domain(bookmarks)
        }
        OrganizeStrategy::PreserveOriginal | OrganizeStrategy::ByTopic => {
            rebuild_preserve_structure(bookmarks)
        }
    };

    // Create empty "other" folder
    let other = RawBookmarkNode {
        id: "2".to_string(),
        name: "Other bookmarks".to_string(),
        node_type: "folder".to_string(),
        url: None,
        children: Some(Vec::new()),
        date_added: None,
        date_last_used: None,
        extra: HashMap::new(),
    };

    BookmarksFile {
        checksum: original.checksum.clone(), // Edge will recalculate this
        roots: BookmarkRoots {
            bookmark_bar: organized,
            other,
            synced: None,
            extra: HashMap::new(),
        },
        version: original.version,
        extra: original.extra.clone(),
    }
}

/// Write bookmarks to a JSON file.
pub fn write_bookmarks_file(bookmarks: &BookmarksFile, path: &std::path::Path) -> Result<()> {
    use crate::error::BookmarkError;
    
    let json = serde_json::to_string_pretty(bookmarks)?;
    std::fs::write(path, json).map_err(|e| BookmarkError::FileWrite {
        path: path.to_path_buf(),
        source: e,
    })?;
    
    Ok(())
}

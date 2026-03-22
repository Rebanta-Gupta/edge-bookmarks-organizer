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

fn update_max_id_from_node(node: &RawBookmarkNode, current_max: &mut u64) {
    if let Ok(id) = node.id.parse::<u64>() {
        *current_max = (*current_max).max(id);
    }

    if let Some(children) = &node.children {
        for child in children {
            update_max_id_from_node(child, current_max);
        }
    }
}

fn next_safe_id_start(original: &BookmarksFile) -> u64 {
    let mut max_id = 0_u64;
    update_max_id_from_node(&original.roots.bookmark_bar, &mut max_id);
    update_max_id_from_node(&original.roots.other, &mut max_id);
    if let Some(synced) = &original.roots.synced {
        update_max_id_from_node(synced, &mut max_id);
    }
    max_id.saturating_add(1)
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
    rebuild_by_domain_with_id_start(bookmarks, 1)
}

fn rebuild_by_domain_with_id_start(bookmarks: &[Bookmark], id_start: u64) -> RawBookmarkNode {
    let mut id_gen = IdGenerator::new(id_start);
    
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

/// Rebuild bookmarks organized by topic.
fn rebuild_by_topic_with_id_start(bookmarks: &[Bookmark], id_start: u64) -> RawBookmarkNode {
    let mut id_gen = IdGenerator::new(id_start);

    // Group by topic first, then by domain to create topic subfolders.
    let mut topic_map: HashMap<String, HashMap<String, Vec<&Bookmark>>> = HashMap::new();
    for bookmark in bookmarks {
        let topic = bookmark
            .topic
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .unwrap_or("Uncategorized")
            .to_string();

        let domain = if bookmark.domain.trim().is_empty() {
            "Unknown Domain".to_string()
        } else {
            bookmark.domain.clone()
        };

        topic_map
            .entry(topic)
            .or_default()
            .entry(domain)
            .or_default()
            .push(bookmark);
    }

    // Sort topics alphabetically for stable output.
    let mut topics: Vec<_> = topic_map.keys().cloned().collect();
    topics.sort();

    let children: Vec<RawBookmarkNode> = topics
        .into_iter()
        .map(|topic| {
            let mut domains: Vec<_> = topic_map[&topic].keys().cloned().collect();
            domains.sort();

            let domain_children: Vec<RawBookmarkNode> = domains
                .into_iter()
                .map(|domain| {
                    let topic_domain_bookmarks = &topic_map[&topic][&domain];
                    let bookmark_nodes: Vec<RawBookmarkNode> = topic_domain_bookmarks
                        .iter()
                        .map(|b| build_url_node(b, &id_gen.next()))
                        .collect();

                    build_folder_node(&domain, &id_gen.next(), bookmark_nodes)
                })
                .collect();

            build_folder_node(&topic, &id_gen.next(), domain_children)
        })
        .collect();

    build_folder_node("Organized Bookmarks", &id_gen.next(), children)
}

/// Rebuild bookmarks preserving original folder structure.
pub fn rebuild_preserve_structure(bookmarks: &[Bookmark]) -> RawBookmarkNode {
    rebuild_preserve_structure_with_id_start(bookmarks, 1)
}

fn rebuild_preserve_structure_with_id_start(bookmarks: &[Bookmark], id_start: u64) -> RawBookmarkNode {
    let mut id_gen = IdGenerator::new(id_start);
    
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
    let id_start = next_safe_id_start(original);

    let organized = match strategy {
        OrganizeStrategy::ByDomain | OrganizeStrategy::ByRootDomain => {
            rebuild_by_domain_with_id_start(bookmarks, id_start)
        }
        OrganizeStrategy::ByTopic => {
            rebuild_by_topic_with_id_start(bookmarks, id_start)
        }
        OrganizeStrategy::PreserveOriginal => {
            rebuild_preserve_structure_with_id_start(bookmarks, id_start)
        }
    };

    // Preserve original root metadata to maximize compatibility with Edge.
    let mut bookmark_bar = original.roots.bookmark_bar.clone();
    bookmark_bar.children = organized.children;

    let mut other = original.roots.other.clone();
    other.children = Some(Vec::new());

    BookmarksFile {
        checksum: original.checksum.clone(), // Edge will recalculate this
        roots: BookmarkRoots {
            bookmark_bar,
            other,
            synced: original.roots.synced.clone(),
            extra: original.roots.extra.clone(),
        },
        version: original.version,
        extra: original.extra.clone(),
    }
}

/// Write bookmarks to a JSON file.
pub fn write_bookmarks_file(bookmarks: &BookmarksFile, path: &std::path::Path) -> Result<()> {
    use crate::error::BookmarkError;
    use std::io::Write;
    
    let json = serde_json::to_string_pretty(bookmarks)?;

    let parent = path.parent().ok_or_else(|| {
        BookmarkError::Other(format!(
            "Cannot determine output directory for {}",
            path.display()
        ))
    })?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| BookmarkError::Other("Invalid output file name".to_string()))?;

    let tmp_name = format!("{}.tmp", file_name);
    let tmp_path = parent.join(tmp_name);

    {
        let mut tmp_file = std::fs::File::create(&tmp_path).map_err(|e| BookmarkError::FileWrite {
            path: tmp_path.clone(),
            source: e,
        })?;
        tmp_file.write_all(json.as_bytes()).map_err(|e| BookmarkError::FileWrite {
            path: tmp_path.clone(),
            source: e,
        })?;
        tmp_file.sync_all().map_err(|e| BookmarkError::FileWrite {
            path: tmp_path.clone(),
            source: e,
        })?;
    }

    // Best-effort replacement across platforms: remove existing target before rename.
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| BookmarkError::FileWrite {
            path: path.to_path_buf(),
            source: e,
        })?;
    }

    std::fs::rename(&tmp_path, path).map_err(|e| BookmarkError::FileWrite {
        path: path.to_path_buf(),
        source: e,
    })?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn bookmark_with_topic(id: &str, name: &str, url: &str, domain: &str, topic: Option<&str>) -> Bookmark {
        Bookmark {
            id: id.to_string(),
            name: name.to_string(),
            url: url.to_string(),
            normalized_url: url.to_string(),
            domain: domain.to_string(),
            folder_path: "Bookmarks Bar".to_string(),
            date_added: None,
            date_last_used: None,
            topic: topic.map(|t| t.to_string()),
        }
    }

    fn sample_bookmarks_file() -> BookmarksFile {
        let bookmark_bar = RawBookmarkNode {
            id: "1".to_string(),
            name: "Bookmarks Bar".to_string(),
            node_type: "folder".to_string(),
            url: None,
            children: Some(Vec::new()),
            date_added: None,
            date_last_used: None,
            extra: HashMap::new(),
        };

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
            checksum: "abc123".to_string(),
            roots: BookmarkRoots {
                bookmark_bar,
                other,
                synced: None,
                extra: HashMap::new(),
            },
            version: 1,
            extra: HashMap::new(),
        }
    }

    #[test]
    fn test_write_bookmarks_file_replaces_content_and_cleans_tmp() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("Bookmarks");
        let tmp = dir.path().join("Bookmarks.tmp");

        std::fs::write(&output, "old-content").unwrap();

        let file = sample_bookmarks_file();
        write_bookmarks_file(&file, &output).unwrap();

        let written = std::fs::read_to_string(&output).unwrap();
        let parsed: BookmarksFile = serde_json::from_str(&written).unwrap();
        assert_eq!(parsed.checksum, "abc123");
        assert_eq!(parsed.version, 1);
        assert!(!tmp.exists());
    }

    #[test]
    fn test_rebuild_by_topic_creates_topic_domain_subfolders() {
        let bookmarks = vec![
            bookmark_with_topic("1", "Movie A", "https://watch.example.com/a", "watch.example.com", Some("Entertainment")),
            bookmark_with_topic("2", "Movie B", "https://watch.example.com/b", "watch.example.com", Some("Entertainment")),
            bookmark_with_topic("3", "Rust", "https://doc.rust-lang.org", "doc.rust-lang.org", Some("Technology")),
            bookmark_with_topic("4", "No Topic", "https://example.com", "example.com", Some("   ")),
        ];

        let organized = rebuild_by_topic_with_id_start(&bookmarks, 100);
        let topic_children = organized.children.expect("topic folders should exist");

        let entertainment = topic_children
            .iter()
            .find(|n| n.name == "Entertainment")
            .expect("Entertainment topic folder should exist");
        let entertainment_children = entertainment.children.clone().expect("topic should contain domain folders");
        assert!(entertainment_children.iter().any(|n| n.name == "watch.example.com"));

        let uncategorized = topic_children
            .iter()
            .find(|n| n.name == "Uncategorized")
            .expect("blank topics should map to Uncategorized");
        let uncategorized_children = uncategorized.children.clone().expect("topic should contain domain folders");
        assert!(uncategorized_children.iter().any(|n| n.name == "example.com"));
    }
}

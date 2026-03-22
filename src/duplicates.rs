//! Duplicate bookmark detection and removal.
//! 
//! Identifies bookmarks pointing to the same URL (after normalization)
//! and provides utilities to deduplicate them.

use crate::bookmark::Bookmark;
use std::collections::{HashMap, HashSet};

/// A group of duplicate bookmarks sharing the same normalized URL.
#[derive(Debug)]
pub struct DuplicateGroup {
    pub normalized_url: String,
    pub bookmarks: Vec<Bookmark>,
}

/// Find all duplicate bookmarks based on normalized URL.
/// 
/// Returns groups of duplicates (each group has 2+ bookmarks with same URL).
pub fn find_duplicates(bookmarks: &[Bookmark]) -> Vec<DuplicateGroup> {
    // Group by normalized URL
    let mut url_map: HashMap<String, Vec<Bookmark>> = HashMap::new();

    for bookmark in bookmarks {
        url_map
            .entry(bookmark.normalized_url.clone())
            .or_default()
            .push(bookmark.clone());
    }

    // Filter to only groups with duplicates (2+)
    url_map
        .into_iter()
        .filter(|(_, bms)| bms.len() > 1)
        .map(|(url, bookmarks)| DuplicateGroup {
            normalized_url: url,
            bookmarks,
        })
        .collect()
}

/// Remove duplicates from the bookmark list, keeping the first occurrence.
/// 
/// "First" is determined by the order in the input list, which typically
/// reflects the order in the bookmarks file.
pub fn remove_duplicates(bookmarks: Vec<Bookmark>) -> Vec<Bookmark> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result = Vec::new();

    for bookmark in bookmarks {
        if seen.insert(bookmark.normalized_url.clone()) {
            // First time seeing this URL - keep it
            result.push(bookmark);
        }
        // Otherwise skip (duplicate)
    }

    result
}

/// Remove duplicates, preferring bookmarks that were used more recently.
pub fn remove_duplicates_keep_recent(bookmarks: Vec<Bookmark>) -> Vec<Bookmark> {
    // Group by normalized URL
    let mut url_map: HashMap<String, Vec<Bookmark>> = HashMap::new();

    for bookmark in bookmarks {
        url_map
            .entry(bookmark.normalized_url.clone())
            .or_default()
            .push(bookmark);
    }

    // For each group, keep the most recently used (or first if no usage data)
    url_map
        .into_values()
        .map(|mut group| {
            group.sort_by(|a, b| {
                // Sort by date_last_used descending (most recent first)
                b.date_last_used.cmp(&a.date_last_used)
            });
            group.remove(0) // Take the first (most recent)
        })
        .collect()
}

/// Statistics about duplicates.
#[derive(Debug)]
pub struct DuplicateStats {
    pub total_duplicates: usize,      // Total extra copies (not counting originals)
    pub unique_urls_with_dupes: usize, // Number of URLs that have duplicates
    pub groups: Vec<DuplicateGroup>,
}

/// Get comprehensive duplicate statistics.
pub fn get_duplicate_stats(bookmarks: &[Bookmark]) -> DuplicateStats {
    let groups = find_duplicates(bookmarks);
    
    let total_duplicates: usize = groups
        .iter()
        .map(|g| g.bookmarks.len() - 1) // -1 because one is the "original"
        .sum();

    DuplicateStats {
        total_duplicates,
        unique_urls_with_dupes: groups.len(),
        groups,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bookmark(id: &str, url: &str, normalized: &str) -> Bookmark {
        Bookmark {
            id: id.to_string(),
            name: format!("Bookmark {}", id),
            url: url.to_string(),
            normalized_url: normalized.to_string(),
            domain: "example.com".to_string(),
            folder_path: "".to_string(),
            date_added: None,
            date_last_used: None,
            topic: None,
        }
    }

    #[test]
    fn test_find_duplicates() {
        let bookmarks = vec![
            make_bookmark("1", "[example.com](https://example.com/)", "[example.com](https://example.com)"),
            make_bookmark("2", "[example.com](https://example.com)", "[example.com](https://example.com)"),
            make_bookmark("3", "[other.com](https://other.com/)", "[other.com](https://other.com)"),
        ];

        let dupes = find_duplicates(&bookmarks);
        assert_eq!(dupes.len(), 1);
        assert_eq!(dupes[0].bookmarks.len(), 2);
    }

    #[test]
    fn test_remove_duplicates() {
        let bookmarks = vec![
            make_bookmark("1", "[example.com](https://example.com/)", "[example.com](https://example.com)"),
            make_bookmark("2", "[example.com](https://example.com)", "[example.com](https://example.com)"),
            make_bookmark("3", "[other.com](https://other.com/)", "[other.com](https://other.com)"),
        ];

        let deduped = remove_duplicates(bookmarks);
        assert_eq!(deduped.len(), 2);
    }
}

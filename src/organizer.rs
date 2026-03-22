//! Bookmark organization by domain and topic.
//! 
//! Groups bookmarks into logical categories for easier management.

use crate::bookmark::Bookmark;
use std::collections::HashMap;

/// Group bookmarks by their domain.
/// 
/// Returns a map where keys are domains and values are lists of
/// bookmarks belonging to that domain.
pub fn group_by_domain(bookmarks: &[Bookmark]) -> HashMap<String, Vec<&Bookmark>> {
    let mut groups: HashMap<String, Vec<&Bookmark>> = HashMap::new();

    for bookmark in bookmarks {
        groups
            .entry(bookmark.domain.clone())
            .or_default()
            .push(bookmark);
    }

    groups
}

/// Statistics about domain grouping.
#[derive(Debug)]
pub struct DomainStats {
    pub domain: String,
    pub count: usize,
    pub bookmarks: Vec<String>, // Names of bookmarks
}

/// Get statistics about bookmarks grouped by domain.
pub fn get_domain_stats(bookmarks: &[Bookmark]) -> Vec<DomainStats> {
    let groups = group_by_domain(bookmarks);
    
    let mut stats: Vec<DomainStats> = groups
        .into_iter()
        .map(|(domain, bms)| DomainStats {
            domain,
            count: bms.len(),
            bookmarks: bms.iter().map(|b| b.name.clone()).collect(),
        })
        .collect();

    // Sort by count descending
    stats.sort_by(|a, b| b.count.cmp(&a.count));
    
    stats
}

/// Group bookmarks by a top-level domain category.
/// 
/// Simplifies domains like "www.reddit.com" and "old.reddit.com" into "reddit.com"
pub fn group_by_root_domain(bookmarks: &[Bookmark]) -> HashMap<String, Vec<&Bookmark>> {
    let mut groups: HashMap<String, Vec<&Bookmark>> = HashMap::new();

    for bookmark in bookmarks {
        let root_domain = extract_root_domain(&bookmark.domain);
        groups
            .entry(root_domain)
            .or_default()
            .push(bookmark);
    }

    groups
}

/// Extract the root domain (e.g., "reddit.com" from "www.reddit.com").
fn extract_root_domain(domain: &str) -> String {
    let parts: Vec<&str> = domain.split('.').collect();
    
    if parts.len() <= 2 {
        return domain.to_string();
    }

    // Handle common TLDs like .co.uk, .com.au, etc.
    let common_second_level = ["co", "com", "org", "net", "edu", "gov"];
    
    if parts.len() >= 3 {
        let second_last = parts[parts.len() - 2];
        if common_second_level.contains(&second_last) && parts[parts.len() - 1].len() == 2 {
            // This is likely a country-code TLD with second level.
            // For domains like example.co.uk (len == 3), the full domain is already the root.
            return if parts.len() >= 4 {
                parts[parts.len() - 3..].join(".")
            } else {
                domain.to_string()
            };
        }
    }

    // Default: take last two parts
    parts[parts.len() - 2..].join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_root_domain() {
        assert_eq!(extract_root_domain("www.reddit.com"), "reddit.com");
        assert_eq!(extract_root_domain("old.reddit.com"), "reddit.com");
        assert_eq!(extract_root_domain("reddit.com"), "reddit.com");
        assert_eq!(extract_root_domain("example.co.uk"), "example.co.uk");
        assert_eq!(extract_root_domain("sub.example.co.uk"), "example.co.uk");
    }
}

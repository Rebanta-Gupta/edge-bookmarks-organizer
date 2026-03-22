//! Dead link detection using HTTP requests.
//! 
//! Checks bookmark URLs for accessibility and categorizes them as
//! alive, dead (4xx/5xx), or unreachable (network errors).

use crate::bookmark::{Bookmark, CheckedBookmark, LinkStatus};
use crate::error::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Configuration for dead link checking.
#[derive(Debug, Clone)]
pub struct CheckConfig {
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Number of concurrent requests
    pub concurrency: usize,
    /// Whether to follow redirects
    pub follow_redirects: bool,
    /// User agent string
    pub user_agent: String,
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 5,
            concurrency: 10,
            follow_redirects: true,
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) EdgeBookmarkChecker/1.0".to_string(),
        }
    }
}

/// Check a single URL and return its status.
async fn check_url(url: &str, config: &CheckConfig) -> LinkStatus {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .redirect(if config.follow_redirects {
            reqwest::redirect::Policy::limited(10)
        } else {
            reqwest::redirect::Policy::none()
        })
        .user_agent(&config.user_agent)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return LinkStatus::Unreachable {
                reason: format!("Failed to create client: {}", e),
            }
        }
    };

    // Try HEAD first (lighter), fall back to GET if HEAD fails
    let response = match client.head(url).send().await {
        Ok(resp) => resp,
        Err(_) => {
            // HEAD might not be supported, try GET
            match client.get(url).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    return LinkStatus::Unreachable {
                        reason: e.to_string(),
                    }
                }
            }
        }
    };

    let status = response.status().as_u16();
    
    if status < 400 {
        LinkStatus::Alive
    } else {
        LinkStatus::Dead { status_code: status }
    }
}

/// Check multiple bookmarks for dead links concurrently.
pub async fn check_bookmarks(
    bookmarks: Vec<Bookmark>,
    config: &CheckConfig,
    show_progress: bool,
) -> Vec<CheckedBookmark> {
    use futures::stream::{self, StreamExt};

    let total = bookmarks.len();
    let pb = if show_progress {
        let pb = ProgressBar::new(total as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    let config = config.clone();
    
    let results: Vec<CheckedBookmark> = stream::iter(bookmarks)
        .map(|bookmark| {
            let config = config.clone();
            async move {
                let status = check_url(&bookmark.url, &config).await;
                CheckedBookmark { bookmark, status }
            }
        })
        .buffer_unordered(config.concurrency)
        .inspect(|_| {
            if let Some(ref pb) = pb {
                pb.inc(1);
            }
        })
        .collect()
        .await;

    if let Some(pb) = pb {
        pb.finish_with_message("Done checking links");
    }

    results
}

/// Filter to only dead links.
pub fn filter_dead_links(checked: &[CheckedBookmark]) -> Vec<&CheckedBookmark> {
    checked
        .iter()
        .filter(|cb| matches!(cb.status, LinkStatus::Dead { .. } | LinkStatus::Unreachable { .. }))
        .collect()
}

/// Filter to only alive links.
pub fn filter_alive_links(checked: &[CheckedBookmark]) -> Vec<&CheckedBookmark> {
    checked
        .iter()
        .filter(|cb| matches!(cb.status, LinkStatus::Alive))
        .collect()
}

/// Remove dead links from a bookmark list.
pub fn remove_dead_bookmarks(checked: Vec<CheckedBookmark>) -> Vec<Bookmark> {
    checked
        .into_iter()
        .filter(|cb| matches!(cb.status, LinkStatus::Alive | LinkStatus::Unknown))
        .map(|cb| cb.bookmark)
        .collect()
}

/// Statistics about link checking results.
#[derive(Debug)]
pub struct LinkCheckStats {
    pub total: usize,
    pub alive: usize,
    pub dead: usize,
    pub unreachable: usize,
}

impl LinkCheckStats {
    pub fn from_checked(checked: &[CheckedBookmark]) -> Self {
        let mut stats = LinkCheckStats {
            total: checked.len(),
            alive: 0,
            dead: 0,
            unreachable: 0,
        };

        for cb in checked {
            match &cb.status {
                LinkStatus::Alive => stats.alive += 1,
                LinkStatus::Dead { .. } => stats.dead += 1,
                LinkStatus::Unreachable { .. } => stats.unreachable += 1,
                LinkStatus::Unknown => {}
            }
        }

        stats
    }
}

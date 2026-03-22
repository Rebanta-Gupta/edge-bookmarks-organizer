//! Topic-based grouping using lightweight text similarity.
//! 
//! This module provides a stubbed interface for topic extraction.
//! For production use, you could integrate rust-bert or another ML library.
//! Currently uses simple keyword-based categorization as a lightweight alternative.

use crate::bookmark::Bookmark;
use rust_stemmers::{Algorithm, Stemmer};
use std::collections::HashMap;

/// Predefined topic categories with associated keywords.
/// Extend this list to improve categorization accuracy.
const TOPIC_KEYWORDS: &[(&str, &[&str])] = &[
    ("Technology", &["tech", "software", "programming", "code", "developer", "api", "github", "stack"]),
    ("News", &["news", "cnn", "bbc", "reuters", "times", "post", "journal"]),
    ("Social Media", &["twitter", "facebook", "instagram", "linkedin", "reddit", "tiktok"]),
    ("Shopping", &["amazon", "ebay", "shop", "store", "buy", "cart", "checkout"]),
    ("Entertainment", &["youtube", "netflix", "spotify", "music", "movie", "video", "game"]),
    ("Reference", &["wikipedia", "wiki", "docs", "documentation", "reference", "manual"]),
    ("Finance", &["bank", "finance", "stock", "invest", "crypto", "money", "trading"]),
    ("Education", &["learn", "course", "tutorial", "education", "university", "school"]),
    ("Development", &["rust", "python", "javascript", "java", "golang", "cpp", "typescript"]),
    ("Cloud", &["aws", "azure", "gcp", "cloud", "kubernetes", "docker", "devops"]),
];

/// Simple topic extractor using keyword matching.
pub struct TopicExtractor {
    stemmer: Stemmer,
    topic_stems: HashMap<String, Vec<String>>, // topic -> stemmed keywords
}

impl TopicExtractor {
    pub fn new() -> Self {
        let stemmer = Stemmer::create(Algorithm::English);
        let mut topic_stems = HashMap::new();

        for (topic, keywords) in TOPIC_KEYWORDS {
            let stems: Vec<String> = keywords
                .iter()
                .map(|kw| stemmer.stem(kw).to_string())
                .collect();
            topic_stems.insert(topic.to_string(), stems);
        }

        Self { stemmer, topic_stems }
    }

    /// Extract a topic from a bookmark based on its name, URL, and domain.
    pub fn extract_topic(&self, bookmark: &Bookmark) -> Option<String> {
        // Combine text sources for analysis
        let text = format!(
            "{} {} {}",
            bookmark.name.to_lowercase(),
            bookmark.domain.to_lowercase(),
            bookmark.url.to_lowercase()
        );

        // Tokenize and stem
        let tokens: Vec<String> = text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 2)
            .map(|s| self.stemmer.stem(s).to_string())
            .collect();

        // Score each topic
        let mut scores: Vec<(String, usize)> = self
            .topic_stems
            .iter()
            .map(|(topic, stems)| {
                let score = tokens
                    .iter()
                    .filter(|t| stems.contains(t))
                    .count();
                (topic.clone(), score)
            })
            .filter(|(_, score)| *score > 0)
            .collect();

        // Return highest scoring topic
        scores.sort_by(|a, b| b.1.cmp(&a.1));
        scores.first().map(|(topic, _)| topic.clone())
    }

    /// Assign topics to all bookmarks.
    pub fn assign_topics(&self, bookmarks: &mut [Bookmark]) {
        for bookmark in bookmarks {
            bookmark.topic = self.extract_topic(bookmark);
        }
    }
}

impl Default for TopicExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Group bookmarks by their assigned topic.
pub fn group_by_topic(bookmarks: &[Bookmark]) -> HashMap<String, Vec<&Bookmark>> {
    let mut groups: HashMap<String, Vec<&Bookmark>> = HashMap::new();

    for bookmark in bookmarks {
        let topic = bookmark
            .topic
            .clone()
            .unwrap_or_else(|| "Uncategorized".to_string());
        groups.entry(topic).or_default().push(bookmark);
    }

    groups
}

// ============================================================================
// STUB: Advanced embedding-based topic extraction
// ============================================================================
// 
// For more sophisticated topic grouping, you could integrate rust-bert:
//
// ```rust
// use rust_bert::pipelines::sentence_embeddings::{
//     SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
// };
//
// pub struct EmbeddingTopicExtractor {
//     model: SentenceEmbeddingsModel,
//     topic_embeddings: HashMap<String, Vec<f32>>,
// }
//
// impl EmbeddingTopicExtractor {
//     pub fn new() -> Self {
//         let model = SentenceEmbeddingsBuilder::remote(
//             SentenceEmbeddingsModelType::AllMiniLmL12V2
//         ).create_model().unwrap();
//         
//         // Pre-compute embeddings for topic names
//         let topics = ["Technology", "News", "Shopping", ...];
//         let topic_embeddings = model.encode(&topics).unwrap();
//         
//         Self { model, topic_embeddings }
//     }
//
//     pub fn extract_topic(&self, bookmark: &Bookmark) -> String {
//         let embedding = self.model.encode(&[&bookmark.name]).unwrap();
//         // Find nearest topic by cosine similarity
//         self.find_nearest_topic(&embedding[0])
//     }
// }
// ```
//
// This requires downloading ~100MB model files and significantly increases
// build time and binary size. The keyword-based approach above is a
// lightweight alternative suitable for most use cases.

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bookmark(name: &str, url: &str, domain: &str) -> Bookmark {
        Bookmark {
            id: "1".to_string(),
            name: name.to_string(),
            url: url.to_string(),
            normalized_url: url.to_string(),
            domain: domain.to_string(),
            folder_path: "".to_string(),
            date_added: None,
            date_last_used: None,
            topic: None,
        }
    }

    #[test]
    fn test_topic_extraction() {
        let extractor = TopicExtractor::new();
        
        let github = make_bookmark("Rust GitHub", "[github.com](https://github.com/rust-lang)", "github.com");
        assert_eq!(extractor.extract_topic(&github), Some("Technology".to_string()));

        let youtube = make_bookmark("YouTube", "[youtube.com](https://youtube.com)", "youtube.com");
        assert_eq!(extractor.extract_topic(&youtube), Some("Entertainment".to_string()));
    }
}

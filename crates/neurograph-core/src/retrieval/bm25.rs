// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Full BM25 (Best Matching 25) index for keyword-based retrieval.
//!
//! Replaces the simple keyword searcher with a proper inverted index
//! supporting TF-IDF scoring with BM25 normalization. Supports
//! incremental document addition/removal.
//!
//! Parameters:
//! - `k1` (1.2): Controls term frequency saturation
//! - `b` (0.75): Controls document length normalization

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// A full BM25 inverted index for keyword-based retrieval.
pub struct BM25Index {
    /// Document frequency: term -> number of documents containing term
    df: HashMap<String, usize>,
    /// Term frequency per document: doc_id -> term -> raw count
    tf: HashMap<Uuid, HashMap<String, usize>>,
    /// Document lengths (in tokens)
    doc_lengths: HashMap<Uuid, usize>,
    /// Average document length (maintained incrementally)
    avg_dl: f64,
    /// Total number of indexed documents
    num_docs: usize,
    /// BM25 k1 parameter: term frequency saturation (typical: 1.2)
    k1: f64,
    /// BM25 b parameter: document length normalization (typical: 0.75)
    b: f64,
}

impl BM25Index {
    /// Create a new BM25 index with the given parameters.
    pub fn new(k1: f64, b: f64) -> Self {
        Self {
            df: HashMap::new(),
            tf: HashMap::new(),
            doc_lengths: HashMap::new(),
            avg_dl: 0.0,
            num_docs: 0,
            k1,
            b,
        }
    }

    /// Create with standard BM25 parameters (k1=1.2, b=0.75).
    pub fn standard() -> Self {
        Self::new(1.2, 0.75)
    }

    /// Number of indexed documents.
    pub fn len(&self) -> usize {
        self.num_docs
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.num_docs == 0
    }

    /// Index a document. If the document ID already exists, it is re-indexed.
    pub fn add_document(&mut self, doc_id: Uuid, text: &str) {
        // Remove existing entry if present (re-index)
        if self.tf.contains_key(&doc_id) {
            self.remove_document(&doc_id);
        }

        let tokens = tokenize(text);
        let doc_len = tokens.len();

        // Compute term frequencies for this document
        let mut term_freqs: HashMap<String, usize> = HashMap::new();
        let mut seen_terms: HashSet<String> = HashSet::new();

        for token in &tokens {
            *term_freqs.entry(token.clone()).or_insert(0) += 1;
            if seen_terms.insert(token.clone()) {
                // First occurrence of this term in this doc — update DF
                *self.df.entry(token.clone()).or_insert(0) += 1;
            }
        }

        self.tf.insert(doc_id, term_freqs);
        self.doc_lengths.insert(doc_id, doc_len);
        self.num_docs += 1;

        // Recalculate average document length
        self.recalc_avg_dl();
    }

    /// Remove a document from the index.
    pub fn remove_document(&mut self, doc_id: &Uuid) {
        if let Some(term_freqs) = self.tf.remove(doc_id) {
            for term in term_freqs.keys() {
                if let Some(count) = self.df.get_mut(term) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        self.df.remove(term);
                    }
                }
            }
        }
        self.doc_lengths.remove(doc_id);
        self.num_docs = self.num_docs.saturating_sub(1);
        self.recalc_avg_dl();
    }

    /// Query the index and return ranked results.
    ///
    /// Returns `Vec<(doc_id, bm25_score)>` sorted by score descending.
    pub fn query(&self, query_text: &str, top_k: usize) -> Vec<(Uuid, f64)> {
        if self.num_docs == 0 {
            return Vec::new();
        }

        let query_tokens = tokenize(query_text);
        let mut scores: HashMap<Uuid, f64> = HashMap::new();

        for token in &query_tokens {
            let df = match self.df.get(token) {
                Some(&df) if df > 0 => df as f64,
                _ => continue,
            };

            // IDF: log((N - df + 0.5) / (df + 0.5) + 1.0)
            // The +1.0 prevents negative IDF for very common terms
            let idf = ((self.num_docs as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();

            for (doc_id, term_freqs) in &self.tf {
                let tf = match term_freqs.get(token) {
                    Some(&tf) if tf > 0 => tf as f64,
                    _ => continue,
                };

                let doc_len = *self.doc_lengths.get(doc_id).unwrap_or(&0) as f64;

                // BM25 TF normalization:
                // (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * dl / avgdl))
                let norm_tf = (tf * (self.k1 + 1.0))
                    / (tf + self.k1 * (1.0 - self.b + self.b * doc_len / self.avg_dl.max(1.0)));

                *scores.entry(*doc_id).or_default() += idf * norm_tf;
            }
        }

        let mut ranked: Vec<(Uuid, f64)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(top_k);
        ranked
    }

    fn recalc_avg_dl(&mut self) {
        if self.num_docs > 0 {
            let total_len: usize = self.doc_lengths.values().sum();
            self.avg_dl = total_len as f64 / self.num_docs as f64;
        } else {
            self.avg_dl = 0.0;
        }
    }
}

impl Default for BM25Index {
    fn default() -> Self {
        Self::standard()
    }
}

/// Tokenizer: lowercase, split on non-alphanumeric, filter stopwords and short tokens.
fn tokenize(text: &str) -> Vec<String> {
    let stopwords: HashSet<&str> = [
        "a", "an", "the", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "shall", "can", "to", "of", "in", "for",
        "on", "with", "at", "by", "from", "as", "into", "about", "like",
        "through", "after", "over", "between", "out", "up", "and", "but",
        "or", "not", "no", "so", "if", "than", "too", "very", "just",
        "it", "its", "this", "that", "these", "those", "he", "she", "they",
        "we", "you", "i", "me", "my", "your", "his", "her", "their", "our",
    ]
    .into_iter()
    .collect();

    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|s| !s.is_empty() && s.len() > 1 && !stopwords.contains(s))
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bm25_basic_ranking() {
        let mut index = BM25Index::standard();

        let doc1 = Uuid::new_v4();
        let doc2 = Uuid::new_v4();
        let doc3 = Uuid::new_v4();

        index.add_document(doc1, "Alice joined Anthropic as a research scientist");
        index.add_document(doc2, "Bob moved from Google to OpenAI in January");
        index.add_document(doc3, "Alice published a paper on temporal knowledge graphs");

        let results = index.query("Alice Anthropic", 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, doc1, "doc1 mentions both Alice and Anthropic");
    }

    #[test]
    fn test_bm25_empty_index() {
        let index = BM25Index::standard();
        let results = index.query("anything", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_bm25_remove_document() {
        let mut index = BM25Index::standard();
        let doc1 = Uuid::new_v4();
        let doc2 = Uuid::new_v4();

        index.add_document(doc1, "Alice works at Anthropic");
        index.add_document(doc2, "Bob works at Google");
        assert_eq!(index.len(), 2);

        index.remove_document(&doc1);
        assert_eq!(index.len(), 1);

        let results = index.query("Alice", 10);
        assert!(results.is_empty(), "Alice doc was removed");
    }

    #[test]
    fn test_bm25_reindex_document() {
        let mut index = BM25Index::standard();
        let doc1 = Uuid::new_v4();

        index.add_document(doc1, "Alice works at Google");
        let results1 = index.query("Google", 10);
        assert!(!results1.is_empty());

        // Re-index same doc with different content
        index.add_document(doc1, "Alice works at Anthropic");
        let results2 = index.query("Google", 10);
        assert!(results2.is_empty(), "Google should not match after re-index");

        let results3 = index.query("Anthropic", 10);
        assert!(!results3.is_empty());
        assert_eq!(index.len(), 1, "Document count should remain 1");
    }

    #[test]
    fn test_bm25_multi_term_boosting() {
        let mut index = BM25Index::standard();
        let doc1 = Uuid::new_v4();
        let doc2 = Uuid::new_v4();

        index.add_document(doc1, "Alice research scientist Anthropic safety");
        index.add_document(doc2, "Bob engineer Google cloud");

        let results = index.query("Alice scientist safety", 10);
        assert_eq!(results[0].0, doc1);
        // doc1 should score much higher because it matches 3 query terms
        if results.len() > 1 {
            assert!(results[0].1 > results[1].1 * 2.0);
        }
    }

    #[test]
    fn test_tokenizer_filters_stopwords() {
        let tokens = tokenize("The quick brown fox jumps over the lazy dog");
        assert!(!tokens.contains(&"the".to_string()));
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"brown".to_string()));
        assert!(tokens.contains(&"fox".to_string()));
    }
}

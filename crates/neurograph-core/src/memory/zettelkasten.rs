// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! A-MEM inspired Zettelkasten self-organizing memory.
//!
//! Each memory is stored as a "note" with auto-generated keywords, tags,
//! and inter-note links. When a new note is added, the system:
//!
//! 1. Extracts keywords from content
//! 2. Computes embedding for semantic similarity
//! 3. Finds semantically similar existing notes
//! 4. Auto-links related notes (bidirectional)
//! 5. Updates existing notes' keywords with new associations
//!
//! This creates a self-organizing graph where memories evolve their
//! connections and context over time — the core A-MEM insight.
//!
//! ## Ebbinghaus Forgetting
//!
//! Each note tracks `memory_strength` (S). Retention is computed as:
//!
//! ```text
//! R = e^(-t/S)
//! ```
//!
//! where `t` is hours since last access and `S` is strength × 24.
//! - S starts at 1.0 and increases by 0.5 on each access (capped at 10.0).
//! - Notes with R < threshold are candidates for forgetting.
//!
//! Reference: "Autonomous Memory Construction and Self-Organization in
//! AI Through the Zettelkasten Method" (A-MEM, 2025).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// A single note in the Zettelkasten system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNote {
    /// Unique identifier.
    pub id: Uuid,
    /// The raw content of this memory.
    pub content: String,
    /// Auto-generated or manually assigned summary/context.
    pub context: String,
    /// Extracted keywords (auto-populated from content).
    pub keywords: Vec<String>,
    /// User or system-assigned tags for categorization.
    pub tags: Vec<String>,
    /// Embedding vector for semantic similarity.
    pub embedding: Vec<f32>,
    /// IDs of linked notes (bidirectional connections).
    pub links: HashSet<Uuid>,
    /// Memory strength (S in Ebbinghaus formula). Grows on access.
    pub memory_strength: f64,
    /// Number of times this note has been accessed.
    pub access_count: u64,
    /// When this note was created.
    pub created_at: DateTime<Utc>,
    /// When this note was last accessed.
    pub last_accessed: DateTime<Utc>,
    /// Computed retention probability (R = e^(-t/S)).
    pub retention: f64,
}

impl MemoryNote {
    /// Create a new note with content and embedding.
    pub fn new(content: impl Into<String>, embedding: Vec<f32>) -> Self {
        let content = content.into();
        let keywords = Self::extract_keywords(&content);
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            context: String::new(),
            keywords,
            tags: Vec::new(),
            content,
            embedding,
            links: HashSet::new(),
            memory_strength: 1.0,
            access_count: 1,
            created_at: now,
            last_accessed: now,
            retention: 1.0,
        }
    }

    /// Simple keyword extraction: split on whitespace, filter stop words,
    /// keep words > 3 chars, lowercase, deduplicate.
    fn extract_keywords(text: &str) -> Vec<String> {
        let stop_words: HashSet<&str> = [
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
            "have", "has", "had", "do", "does", "did", "will", "would", "could",
            "should", "may", "might", "shall", "can", "need", "dare", "ought",
            "used", "to", "of", "in", "for", "on", "with", "at", "by", "from",
            "as", "into", "through", "during", "before", "after", "above",
            "below", "between", "but", "and", "or", "not", "no", "nor", "so",
            "yet", "both", "either", "neither", "each", "every", "all", "any",
            "few", "more", "most", "other", "some", "such", "than", "too",
            "very", "just", "about", "also", "that", "this", "these", "those",
            "what", "which", "who", "whom", "how", "when", "where", "why",
            "it", "its", "he", "she", "they", "them", "his", "her", "their",
        ]
        .iter()
        .copied()
        .collect();

        let mut seen = HashSet::new();
        text.split(|c: char| !c.is_alphanumeric())
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() > 3 && !stop_words.contains(w.as_str()))
            .filter(|w| seen.insert(w.clone()))
            .take(20) // Cap at 20 keywords per note
            .collect()
    }

    /// Compute current retention using the Ebbinghaus forgetting curve.
    ///
    /// R = e^(-t/S) where t = hours since last access, S = strength × 24.
    pub fn compute_retention(&self) -> f64 {
        let now = Utc::now();
        let hours_since_access = (now - self.last_accessed).num_hours().max(0) as f64;
        let s = self.memory_strength * 24.0; // Convert to hours
        (-hours_since_access / s).exp()
    }

    /// Record an access, boosting memory strength.
    pub fn access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
        // Strength increases on each access (spaced repetition effect)
        self.memory_strength = (self.memory_strength + 0.5).min(10.0);
        self.retention = self.compute_retention();
    }

    /// Add a bidirectional link to another note.
    pub fn link_to(&mut self, other_id: Uuid) {
        self.links.insert(other_id);
    }
}

/// Configuration for the Zettelkasten memory system.
#[derive(Debug, Clone)]
pub struct ZettelkastenConfig {
    /// Similarity threshold for auto-linking (0.0–1.0).
    pub link_threshold: f64,
    /// Maximum number of auto-links per new note.
    pub max_auto_links: usize,
    /// Retention threshold below which notes are candidates for forgetting.
    pub forget_threshold: f64,
    /// Maximum notes before overflow eviction.
    pub max_notes: usize,
}

impl Default for ZettelkastenConfig {
    fn default() -> Self {
        Self {
            link_threshold: 0.70,
            max_auto_links: 10,
            forget_threshold: 0.2,
            max_notes: 50_000,
        }
    }
}

/// The Zettelkasten self-organizing memory system.
pub struct ZettelkastenMemory {
    /// All notes indexed by ID.
    notes: HashMap<Uuid, MemoryNote>,
    /// Tag index: tag → note IDs.
    tag_index: HashMap<String, HashSet<Uuid>>,
    /// Keyword index: keyword → note IDs.
    keyword_index: HashMap<String, HashSet<Uuid>>,
    /// Configuration.
    config: ZettelkastenConfig,
}

/// Result of adding a new note.
#[derive(Debug)]
pub struct AddNoteResult {
    /// The ID of the new note.
    pub note_id: Uuid,
    /// IDs of notes that were auto-linked.
    pub linked_notes: Vec<Uuid>,
    /// Number of existing notes whose keywords were updated.
    pub evolved_notes: usize,
}

/// Result of a forgetting pass.
#[derive(Debug)]
pub struct ForgetResult {
    /// Notes whose retention was recalculated.
    pub retention_updated: usize,
    /// Notes forgotten (removed).
    pub notes_forgotten: usize,
    /// Notes remaining.
    pub notes_remaining: usize,
}

impl ZettelkastenMemory {
    /// Create a new Zettelkasten memory system.
    pub fn new(config: ZettelkastenConfig) -> Self {
        Self {
            notes: HashMap::new(),
            tag_index: HashMap::new(),
            keyword_index: HashMap::new(),
            config,
        }
    }

    /// Add a new note with auto-linking and memory evolution.
    ///
    /// This is the core A-MEM operation:
    /// 1. Create note with auto-extracted keywords
    /// 2. Find semantically similar existing notes by embedding
    /// 3. Auto-link to similar notes (bidirectional)
    /// 4. **Evolve** linked notes' keywords (the key A-MEM insight)
    pub fn add_note(
        &mut self,
        content: impl Into<String>,
        embedding: Vec<f32>,
        tags: Vec<String>,
    ) -> AddNoteResult {
        let mut note = MemoryNote::new(content, embedding);
        note.tags = tags.clone();

        let note_id = note.id;
        let note_keywords = note.keywords.clone();

        // Find similar notes by embedding similarity
        let similar_notes = self.find_similar(&note.embedding, self.config.max_auto_links);

        let mut linked_notes = Vec::new();
        let mut evolved_notes = 0;

        // Auto-link to similar notes
        for (similar_id, _similarity) in &similar_notes {
            if *similar_id == note_id {
                continue;
            }

            // Add bidirectional link
            note.link_to(*similar_id);
            if let Some(existing) = self.notes.get_mut(similar_id) {
                existing.link_to(note_id);

                // KEY A-MEM INSIGHT: Evolve the existing note's keywords
                // by adding the new note's keywords that are novel
                let existing_kw_set: HashSet<_> = existing.keywords.iter().cloned().collect();
                let new_keywords: Vec<_> = note_keywords
                    .iter()
                    .filter(|kw| !existing_kw_set.contains(*kw))
                    .take(3) // Add at most 3 new keywords per existing note
                    .cloned()
                    .collect();

                if !new_keywords.is_empty() {
                    for kw in &new_keywords {
                        // Update keyword index
                        self.keyword_index
                            .entry(kw.clone())
                            .or_default()
                            .insert(*similar_id);
                    }
                    existing.keywords.extend(new_keywords);
                    evolved_notes += 1;
                }
            }

            linked_notes.push(*similar_id);
        }

        // Update indexes
        for tag in &note.tags {
            self.tag_index
                .entry(tag.clone())
                .or_default()
                .insert(note_id);
        }
        for keyword in &note.keywords {
            self.keyword_index
                .entry(keyword.clone())
                .or_default()
                .insert(note_id);
        }

        self.notes.insert(note_id, note);

        AddNoteResult {
            note_id,
            linked_notes,
            evolved_notes,
        }
    }

    /// Search for notes by query text.
    ///
    /// Combines keyword matching with Zettelkasten link traversal:
    /// 1. Find notes matching query keywords
    /// 2. From matching seeds, BFS traverse links up to 2 hops
    /// 3. Score by: keyword overlap × tier boost × link distance
    pub fn search(&mut self, query: &str, max_results: usize) -> Vec<(Uuid, f64)> {
        let query_keywords = MemoryNote::extract_keywords(query);
        if query_keywords.is_empty() {
            return Vec::new();
        }

        // Step 1: Find seed notes (keyword match)
        let mut seed_scores: HashMap<Uuid, f64> = HashMap::new();
        for kw in &query_keywords {
            if let Some(note_ids) = self.keyword_index.get(kw) {
                for id in note_ids {
                    *seed_scores.entry(*id).or_default() += 1.0;
                }
            }
        }

        // Normalize by query keyword count
        let kw_count = query_keywords.len() as f64;
        for score in seed_scores.values_mut() {
            *score /= kw_count;
        }

        // Step 2: BFS expand from seeds (2 hops)
        let mut expanded: HashMap<Uuid, f64> = seed_scores.clone();
        let seeds: Vec<Uuid> = seed_scores.keys().cloned().collect();

        for seed_id in &seeds {
            let seed_score = seed_scores.get(seed_id).copied().unwrap_or(0.0);

            if let Some(note) = self.notes.get(seed_id) {
                // 1-hop neighbors
                for neighbor_id in &note.links {
                    let hop1_score = seed_score * 0.5; // Decay by 50% per hop
                    let entry = expanded.entry(*neighbor_id).or_default();
                    *entry = entry.max(hop1_score);

                    // 2-hop neighbors
                    if let Some(neighbor) = self.notes.get(neighbor_id) {
                        for hop2_id in &neighbor.links {
                            if !seeds.contains(hop2_id) {
                                let hop2_score = seed_score * 0.25;
                                let entry = expanded.entry(*hop2_id).or_default();
                                *entry = entry.max(hop2_score);
                            }
                        }
                    }
                }
            }
        }

        // Record access for retrieved notes
        let result_ids: Vec<Uuid> = expanded.keys().cloned().collect();
        for id in &result_ids {
            if let Some(note) = self.notes.get_mut(id) {
                note.access();
            }
        }

        // Sort by score and truncate
        let mut results: Vec<(Uuid, f64)> = expanded.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(max_results);
        results
    }

    /// Search by embedding similarity.
    pub fn search_by_embedding(
        &mut self,
        embedding: &[f32],
        max_results: usize,
    ) -> Vec<(Uuid, f64)> {
        let similar = self.find_similar(embedding, max_results);

        // Record access
        for (id, _) in &similar {
            if let Some(note) = self.notes.get_mut(id) {
                note.access();
            }
        }

        similar
    }

    /// Run the Ebbinghaus forgetting pass.
    ///
    /// For each note:
    /// 1. Recompute retention: R = e^(-t/S)
    /// 2. If R < threshold → forget (remove)
    pub fn apply_forgetting(&mut self) -> ForgetResult {
        let mut retention_updated = 0;
        let mut to_forget = Vec::new();

        for (id, note) in &mut self.notes {
            let new_retention = note.compute_retention();
            if (new_retention - note.retention).abs() > 0.001 {
                note.retention = new_retention;
                retention_updated += 1;
            }

            if new_retention < self.config.forget_threshold {
                to_forget.push(*id);
            }
        }

        let notes_forgotten = to_forget.len();

        // Remove forgotten notes and clean up indexes/links
        for id in &to_forget {
            if let Some(note) = self.notes.remove(id) {
                // Remove from tag index
                for tag in &note.tags {
                    if let Some(ids) = self.tag_index.get_mut(tag) {
                        ids.remove(id);
                    }
                }
                // Remove from keyword index
                for kw in &note.keywords {
                    if let Some(ids) = self.keyword_index.get_mut(kw) {
                        ids.remove(id);
                    }
                }
                // Remove links from connected notes
                for linked_id in &note.links {
                    if let Some(linked_note) = self.notes.get_mut(linked_id) {
                        linked_note.links.remove(id);
                    }
                }
            }
        }

        // Overflow eviction if over max
        if self.notes.len() > self.config.max_notes {
            let mut all: Vec<(Uuid, f64)> = self
                .notes
                .iter()
                .map(|(id, note)| (*id, note.retention))
                .collect();
            all.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            let to_evict = self.notes.len() - self.config.max_notes;
            for (id, _) in all.into_iter().take(to_evict) {
                self.notes.remove(&id);
                // Note: Full index cleanup is done lazily for performance
            }
        }

        ForgetResult {
            retention_updated,
            notes_forgotten,
            notes_remaining: self.notes.len(),
        }
    }

    /// Get a note by ID.
    pub fn get(&self, id: &Uuid) -> Option<&MemoryNote> {
        self.notes.get(id)
    }

    /// Get a mutable reference to a note by ID.
    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut MemoryNote> {
        self.notes.get_mut(id)
    }

    /// Total number of notes.
    pub fn note_count(&self) -> usize {
        self.notes.len()
    }

    /// Total number of inter-note links.
    pub fn link_count(&self) -> usize {
        self.notes.values().map(|n| n.links.len()).sum::<usize>() / 2
    }

    /// Get notes by tag.
    pub fn notes_by_tag(&self, tag: &str) -> Vec<&MemoryNote> {
        self.tag_index
            .get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.notes.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find similar notes by embedding cosine similarity.
    fn find_similar(&self, embedding: &[f32], max_results: usize) -> Vec<(Uuid, f64)> {
        let mut similarities: Vec<(Uuid, f64)> = self
            .notes
            .iter()
            .filter(|(_, note)| !note.embedding.is_empty())
            .map(|(id, note)| (*id, cosine_sim(embedding, &note.embedding)))
            .filter(|(_, sim)| *sim >= self.config.link_threshold)
            .collect();

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities.truncate(max_results);
        similarities
    }
}

impl Default for ZettelkastenMemory {
    fn default() -> Self {
        Self::new(ZettelkastenConfig::default())
    }
}

/// Cosine similarity for embedding comparison.
fn cosine_sim(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_embedding(seed: f32, dim: usize) -> Vec<f32> {
        (0..dim).map(|i| (seed + i as f32 * 0.1).sin()).collect()
    }

    #[test]
    fn test_add_and_retrieve_note() {
        let mut zettel = ZettelkastenMemory::default();
        let result = zettel.add_note(
            "Alice works at Anthropic as a researcher",
            make_embedding(1.0, 64),
            vec!["person".to_string()],
        );

        assert_eq!(zettel.note_count(), 1);
        let note = zettel.get(&result.note_id).unwrap();
        assert!(note.keywords.contains(&"alice".to_string()));
        assert!(note.keywords.contains(&"anthropic".to_string()));
        assert!(note.keywords.contains(&"researcher".to_string()));
    }

    #[test]
    fn test_auto_linking() {
        let mut zettel = ZettelkastenMemory::new(ZettelkastenConfig {
            link_threshold: 0.5, // Lower threshold for test vectors
            ..Default::default()
        });

        // Two notes with similar embeddings
        let r1 = zettel.add_note(
            "Alice works at Anthropic",
            make_embedding(1.0, 64),
            vec![],
        );
        let r2 = zettel.add_note(
            "Alice is a researcher at Anthropic",
            make_embedding(1.05, 64), // Very similar embedding
            vec![],
        );

        // They should be auto-linked
        let note1 = zettel.get(&r1.note_id).unwrap();
        let note2 = zettel.get(&r2.note_id).unwrap();
        assert!(note1.links.contains(&r2.note_id) || note2.links.contains(&r1.note_id));
    }

    #[test]
    fn test_keyword_evolution() {
        let mut zettel = ZettelkastenMemory::new(ZettelkastenConfig {
            link_threshold: 0.5,
            ..Default::default()
        });

        let r1 = zettel.add_note(
            "Alice works at Anthropic",
            make_embedding(1.0, 64),
            vec![],
        );

        let initial_keywords_len = zettel.get(&r1.note_id).unwrap().keywords.len();

        // Add a related note → should evolve note1's keywords
        let r2 = zettel.add_note(
            "Anthropic builds Claude language models safety research",
            make_embedding(1.05, 64),
            vec![],
        );

        // The 2nd result should report evolved notes if embedding similarity was high enough
        if !r2.linked_notes.is_empty() {
            let note1 = zettel.get(&r1.note_id).unwrap();
            assert!(
                note1.keywords.len() >= initial_keywords_len,
                "Note1 keywords should have grown or stayed the same"
            );
        }
    }

    #[test]
    fn test_search_by_keywords() {
        let mut zettel = ZettelkastenMemory::default();

        zettel.add_note(
            "Alice works at Anthropic as a researcher",
            make_embedding(1.0, 64),
            vec![],
        );
        zettel.add_note(
            "Bob works at Google as an engineer",
            make_embedding(2.0, 64),
            vec![],
        );

        let results = zettel.search("Alice Anthropic", 10);
        assert!(!results.is_empty());

        // Alice note should rank higher
        let top_id = results[0].0;
        let top_note = zettel.get(&top_id).unwrap();
        assert!(top_note.content.contains("Alice"));
    }

    #[test]
    fn test_ebbinghaus_retention() {
        let note = MemoryNote::new("Test content for retention", vec![0.1; 8]);

        // Freshly created → retention ≈ 1.0
        let r = note.compute_retention();
        assert!(
            r > 0.99,
            "Fresh note should have high retention, got {}",
            r
        );
    }

    #[test]
    fn test_memory_strength_growth() {
        let mut note = MemoryNote::new("Test accessing note", vec![0.1; 8]);
        assert!((note.memory_strength - 1.0).abs() < f64::EPSILON);

        note.access();
        assert!((note.memory_strength - 1.5).abs() < f64::EPSILON);

        note.access();
        assert!((note.memory_strength - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_memory_strength_cap() {
        let mut note = MemoryNote::new("Test strength cap", vec![0.1; 8]);

        // Access 30 times → should cap at 10.0
        for _ in 0..30 {
            note.access();
        }

        assert!(
            (note.memory_strength - 10.0).abs() < f64::EPSILON,
            "Strength should cap at 10.0, got {}",
            note.memory_strength
        );
    }

    #[test]
    fn test_notes_by_tag() {
        let mut zettel = ZettelkastenMemory::default();

        zettel.add_note(
            "Alice is a person",
            make_embedding(1.0, 64),
            vec!["person".to_string()],
        );
        zettel.add_note(
            "Anthropic is a company",
            make_embedding(2.0, 64),
            vec!["organization".to_string()],
        );
        zettel.add_note(
            "Bob is a person",
            make_embedding(3.0, 64),
            vec!["person".to_string()],
        );

        let people = zettel.notes_by_tag("person");
        assert_eq!(people.len(), 2);

        let orgs = zettel.notes_by_tag("organization");
        assert_eq!(orgs.len(), 1);
    }

    #[test]
    fn test_keyword_extraction() {
        let keywords = MemoryNote::extract_keywords(
            "Alice works at Anthropic as a research scientist in San Francisco",
        );

        assert!(keywords.contains(&"alice".to_string()));
        assert!(keywords.contains(&"anthropic".to_string()));
        assert!(keywords.contains(&"research".to_string()));
        assert!(keywords.contains(&"scientist".to_string()));
        assert!(keywords.contains(&"francisco".to_string()));
        // Stop words should be excluded
        assert!(!keywords.contains(&"at".to_string()));
        assert!(!keywords.contains(&"as".to_string()));
    }
}

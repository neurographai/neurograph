// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Extractor trait and result types.
//!
//! The `Extractor` trait defines the interface for extracting entities and
//! relationships from input data. Implementations include:
//! - `TextExtractor`: LLM-based extraction from natural language text
//! - `JsonExtractor`: Direct mapping from structured JSON data
//!
//! Influenced by:
//! - Graphiti's `add_episode()` extraction flow (graphiti.py L340-520)
//! - GraphRAG's `extract_entities.py` structured output pattern

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// An entity extracted from input data (before deduplication).
///
/// These are "raw" entities that haven't been reconciled with existing
/// entities in the graph yet. After deduplication, they become `Entity` nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// Entity name as found in the source text.
    pub name: String,

    /// Inferred entity type (e.g., "Person", "Organization", "Location").
    pub entity_type: String,

    /// Brief description/summary extracted from context.
    pub summary: String,
}

/// A relationship extracted from input data (before deduplication).
///
/// Raw relationships link extracted entity names (not IDs) because
/// entities haven't been resolved yet at this stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    /// Name of the source entity.
    pub source_entity: String,

    /// Name of the target entity.
    pub target_entity: String,

    /// Relationship type (e.g., "WORKS_AT", "LIVES_IN").
    pub relationship_type: String,

    /// Natural language fact describing the relationship.
    /// e.g., "Alice works at Anthropic as a research scientist"
    pub fact: String,

    /// Extraction confidence score (0.0 - 1.0).
    pub confidence: f64,
}

/// The result of an extraction operation.
///
/// Contains all entities and relationships extracted from a single input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// Extracted entities.
    pub entities: Vec<ExtractedEntity>,

    /// Extracted relationships.
    pub relationships: Vec<ExtractedRelationship>,

    /// Cost of the extraction operation (in USD).
    pub cost_usd: f64,

    /// Latency of the extraction (in milliseconds).
    pub latency_ms: u64,
}

impl ExtractionResult {
    /// Create an empty extraction result.
    pub fn empty() -> Self {
        Self {
            entities: Vec::new(),
            relationships: Vec::new(),
            cost_usd: 0.0,
            latency_ms: 0,
        }
    }

    /// Check if extraction found anything.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty() && self.relationships.is_empty()
    }

    /// Merge another extraction result into this one.
    pub fn merge(&mut self, other: ExtractionResult) {
        self.entities.extend(other.entities);
        self.relationships.extend(other.relationships);
        self.cost_usd += other.cost_usd;
        self.latency_ms += other.latency_ms;
    }
}

/// The core extractor trait.
///
/// Implementations extract entities and relationships from different
/// input formats (text, JSON, images, etc.).
#[async_trait]
pub trait Extractor: Send + Sync {
    /// Get the extractor name for logging.
    fn name(&self) -> &str;

    /// Extract entities and relationships from text input.
    async fn extract(&self, input: &str) -> Result<ExtractionResult, ExtractionError>;
}

/// Errors during extraction.
#[derive(Debug, thiserror::Error)]
pub enum ExtractionError {
    #[error("LLM extraction failed: {0}")]
    LlmError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Input too large: {size} bytes (max: {max})")]
    InputTooLarge { size: usize, max: usize },

    #[error("Unsupported input format: {0}")]
    UnsupportedFormat(String),
}

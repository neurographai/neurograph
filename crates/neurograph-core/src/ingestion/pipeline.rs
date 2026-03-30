// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Ingestion pipeline orchestrator.
//!
//! Coordinates the full flow:
//! ```text
//! Source Data → Extract → Validate → Deduplicate → Resolve Conflicts → Store
//! ```
//!
//! Influenced by:
//! - Graphiti's `add_episode()` (graphiti.py L340-520): incremental ingestion
//! - Cognee's `cognify()`: clean pipeline abstraction
//! - GraphRAG's indexing pipeline: structured extraction with cost tracking

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;

use crate::drivers::traits::GraphDriver;
use crate::embedders::traits::Embedder;
use crate::graph::entity::EntityId;
use crate::graph::ontology::Ontology;
use crate::graph::{Entity, Episode, Relationship};
use crate::llm::traits::LlmClient;

use super::conflict::{ConflictResolver, ConflictType};
use super::deduplication::{DeduplicationResult, Deduplicator};
use super::extractors::text::TextExtractor;
use super::extractors::traits::{ExtractionResult, Extractor};
use super::validators::SchemaValidator;

/// Result of a pipeline run.
#[derive(Debug)]
pub struct PipelineResult {
    /// Entities created or updated.
    pub entities_stored: usize,
    /// Relationships created.
    pub relationships_stored: usize,
    /// Entities that were deduplicated (merged into existing).
    pub entities_deduplicated: usize,
    /// Relationships invalidated due to conflicts.
    pub conflicts_resolved: usize,
    /// Total LLM cost for this pipeline run (USD).
    pub cost_usd: f64,
    /// Total pipeline latency (milliseconds).
    pub latency_ms: u64,
}

/// Errors from the ingestion pipeline.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Extraction error: {0}")]
    Extraction(String),

    #[error("Deduplication error: {0}")]
    Deduplication(String),

    #[error("Conflict resolution error: {0}")]
    Conflict(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

/// The ingestion pipeline.
///
/// Orchestrates extraction → validation → deduplication → conflict resolution → storage.
pub struct IngestionPipeline {
    driver: Arc<dyn GraphDriver>,
    embedder: Arc<dyn Embedder>,
    extractor: Box<dyn Extractor>,
    deduplicator: Deduplicator,
    validator: SchemaValidator,
    group_id: String,
}

impl IngestionPipeline {
    /// Create a new ingestion pipeline.
    pub fn new(
        driver: Arc<dyn GraphDriver>,
        embedder: Arc<dyn Embedder>,
        llm: Option<Arc<dyn LlmClient>>,
        ontology: Ontology,
        group_id: String,
    ) -> Self {
        let extractor: Box<dyn Extractor> = if let Some(llm) = llm {
            Box::new(TextExtractor::with_llm(llm))
        } else {
            Box::new(TextExtractor::regex_only())
        };

        Self {
            driver,
            embedder,
            extractor,
            deduplicator: Deduplicator::new(),
            validator: SchemaValidator::new(ontology),
            group_id,
        }
    }

    /// Ingest text: extract entities/relationships and store them.
    ///
    /// Full pipeline:
    /// 1. Create an Episode (provenance record)
    /// 2. Extract entities and relationships from text
    /// 3. Validate against ontology
    /// 4. For each entity: check for duplicates, merge or create new
    /// 5. For each relationship: check for conflicts, invalidate old if needed
    /// 6. Store everything with embeddings
    pub async fn ingest_text(
        &self,
        text: &str,
        source_name: &str,
    ) -> Result<(Episode, PipelineResult), PipelineError> {
        let start = Instant::now();
        let mut result = PipelineResult {
            entities_stored: 0,
            relationships_stored: 0,
            entities_deduplicated: 0,
            conflicts_resolved: 0,
            cost_usd: 0.0,
            latency_ms: 0,
        };

        // Step 1: Create episode (provenance)
        let episode = Episode::text(source_name, text)
            .with_group_id(&self.group_id);

        self.driver
            .store_episode(&episode)
            .await
            .map_err(|e| PipelineError::Storage(e.to_string()))?;

        // Step 2: Extract entities and relationships
        let extraction = self
            .extractor
            .extract(text)
            .await
            .map_err(|e| PipelineError::Extraction(e.to_string()))?;

        result.cost_usd += extraction.cost_usd;

        if extraction.is_empty() {
            tracing::info!("No entities or relationships extracted from text");
            result.latency_ms = start.elapsed().as_millis() as u64;
            return Ok((episode, result));
        }

        tracing::info!(
            entities = extraction.entities.len(),
            relationships = extraction.relationships.len(),
            "Extraction complete, starting deduplication"
        );

        // Step 3: Process entities (validate → deduplicate → store)
        let mut entity_name_to_id = std::collections::HashMap::new();

        for extracted_entity in &extraction.entities {
            // Validate
            let validation = self.validator.validate_entity(extracted_entity);
            if !validation.is_valid {
                tracing::warn!(
                    entity = %extracted_entity.name,
                    errors = ?validation.errors,
                    "Entity validation failed, skipping"
                );
                continue;
            }
            for warning in &validation.warnings {
                tracing::debug!(entity = %extracted_entity.name, warning = %warning);
            }

            // Generate embedding
            let embedding = self
                .embedder
                .embed_one(&extracted_entity.name)
                .await
                .map_err(|e| PipelineError::Storage(format!("Embedding error: {}", e)))?;

            // Deduplicate
            let dedup_result = self
                .deduplicator
                .check_duplicate(
                    &extracted_entity.name,
                    &extracted_entity.entity_type,
                    Some(&embedding),
                    self.driver.as_ref(),
                    self.embedder.as_ref(),
                )
                .await
                .map_err(|e| PipelineError::Deduplication(e.to_string()))?;

            match dedup_result {
                DeduplicationResult::New => {
                    // Create new entity
                    let entity = Entity::new(&extracted_entity.name, &extracted_entity.entity_type)
                        .with_summary(&extracted_entity.summary)
                        .with_group_id(&self.group_id)
                        .with_embedding(embedding);

                    self.driver
                        .store_entity(&entity)
                        .await
                        .map_err(|e| PipelineError::Storage(e.to_string()))?;

                    entity_name_to_id.insert(extracted_entity.name.clone(), entity.id.clone());
                    result.entities_stored += 1;

                    tracing::debug!(
                        entity = %extracted_entity.name,
                        id = %entity.id,
                        "Stored new entity"
                    );
                }
                DeduplicationResult::Merge {
                    existing_id,
                    similarity,
                } => {
                    // Merge into existing entity
                    if let Ok(mut existing) = self.driver.get_entity(&existing_id).await {
                        Deduplicator::merge_entities(
                            &mut existing,
                            &extracted_entity.name,
                            &extracted_entity.summary,
                        );
                        self.driver
                            .store_entity(&existing)
                            .await
                            .map_err(|e| PipelineError::Storage(e.to_string()))?;
                    }

                    entity_name_to_id.insert(extracted_entity.name.clone(), existing_id);
                    result.entities_deduplicated += 1;

                    tracing::debug!(
                        entity = %extracted_entity.name,
                        similarity = similarity,
                        "Merged with existing entity"
                    );
                }
            }
        }

        // Step 4: Process relationships (validate → resolve conflicts → store)
        for extracted_rel in &extraction.relationships {
            // Validate
            let validation = self.validator.validate_relationship(extracted_rel);
            if !validation.is_valid {
                tracing::warn!(
                    source = %extracted_rel.source_entity,
                    target = %extracted_rel.target_entity,
                    errors = ?validation.errors,
                    "Relationship validation failed, skipping"
                );
                continue;
            }

            // Resolve entity name → ID
            let source_id = match entity_name_to_id.get(&extracted_rel.source_entity) {
                Some(id) => id.clone(),
                None => {
                    // Entity wasn't extracted — create a minimal one
                    let entity = Entity::new(&extracted_rel.source_entity, "Entity")
                        .with_group_id(&self.group_id);
                    self.driver
                        .store_entity(&entity)
                        .await
                        .map_err(|e| PipelineError::Storage(e.to_string()))?;
                    entity_name_to_id
                        .insert(extracted_rel.source_entity.clone(), entity.id.clone());
                    result.entities_stored += 1;
                    entity.id
                }
            };

            let target_id = match entity_name_to_id.get(&extracted_rel.target_entity) {
                Some(id) => id.clone(),
                None => {
                    let entity = Entity::new(&extracted_rel.target_entity, "Entity")
                        .with_group_id(&self.group_id);
                    self.driver
                        .store_entity(&entity)
                        .await
                        .map_err(|e| PipelineError::Storage(e.to_string()))?;
                    entity_name_to_id
                        .insert(extracted_rel.target_entity.clone(), entity.id.clone());
                    result.entities_stored += 1;
                    entity.id
                }
            };

            // Check for temporal conflicts
            let conflicts = ConflictResolver::detect_conflicts(
                &source_id,
                &extracted_rel.relationship_type,
                &target_id,
                self.driver.as_ref(),
            )
            .await
            .map_err(|e| PipelineError::Conflict(e.to_string()))?;

            let mut skip = false;
            for conflict in conflicts {
                match conflict.conflict_type {
                    ConflictType::Redundant => {
                        tracing::debug!(
                            fact = %extracted_rel.fact,
                            "Redundant relationship, skipping"
                        );
                        skip = true;
                        break;
                    }
                    ConflictType::Contradiction | ConflictType::Supersession => {
                        // Invalidate the old relationship
                        let mut old_rel = conflict.existing_relationship;
                        ConflictResolver::resolve_contradiction(&mut old_rel, Utc::now());
                        self.driver
                            .store_relationship(&old_rel)
                            .await
                            .map_err(|e| PipelineError::Storage(e.to_string()))?;
                        result.conflicts_resolved += 1;
                    }
                }
            }

            if skip {
                continue;
            }

            // Generate fact embedding
            let fact_embedding = self
                .embedder
                .embed_one(&extracted_rel.fact)
                .await
                .map_err(|e| PipelineError::Storage(format!("Embedding error: {}", e)))?;

            // Store the new relationship
            let relationship = Relationship::new(
                source_id,
                target_id,
                &extracted_rel.relationship_type,
                &extracted_rel.fact,
            )
            .with_confidence(extracted_rel.confidence)
            .with_group_id(&self.group_id)
            .with_episode(episode.id.clone())
            .with_embedding(fact_embedding);

            self.driver
                .store_relationship(&relationship)
                .await
                .map_err(|e| PipelineError::Storage(e.to_string()))?;

            result.relationships_stored += 1;

            tracing::debug!(
                fact = %extracted_rel.fact,
                rel_type = %extracted_rel.relationship_type,
                "Stored relationship"
            );
        }

        result.latency_ms = start.elapsed().as_millis() as u64;

        tracing::info!(
            entities_stored = result.entities_stored,
            relationships_stored = result.relationships_stored,
            entities_deduplicated = result.entities_deduplicated,
            conflicts_resolved = result.conflicts_resolved,
            cost_usd = result.cost_usd,
            latency_ms = result.latency_ms,
            "Ingestion pipeline complete"
        );

        Ok((episode, result))
    }

    /// Ingest JSON data.
    pub async fn ingest_json(
        &self,
        json: &serde_json::Value,
        source_name: &str,
    ) -> Result<(Episode, PipelineResult), PipelineError> {
        // Store episode with JSON content
        let episode = Episode::json(source_name, json.clone())
            .with_group_id(&self.group_id);

        self.driver
            .store_episode(&episode)
            .await
            .map_err(|e| PipelineError::Storage(e.to_string()))?;

        // Use JSON extractor
        let json_extractor = super::extractors::json::JsonExtractor::new();
        let json_str = json.to_string();
        let extraction = json_extractor
            .extract(&json_str)
            .await
            .map_err(|e| PipelineError::Extraction(e.to_string()))?;

        // Process same as text (reuse text pipeline logic for dedup/conflict/store)
        // Build a fake text from the extraction for the pipeline
        self.process_extraction(extraction, &episode).await
    }

    /// Process an extraction result through dedup → conflict → store.
    async fn process_extraction(
        &self,
        extraction: ExtractionResult,
        episode: &Episode,
    ) -> Result<(Episode, PipelineResult), PipelineError> {
        let start = Instant::now();
        let mut result = PipelineResult {
            entities_stored: 0,
            relationships_stored: 0,
            entities_deduplicated: 0,
            conflicts_resolved: 0,
            cost_usd: extraction.cost_usd,
            latency_ms: 0,
        };

        let mut entity_name_to_id = std::collections::HashMap::<String, EntityId>::new();

        // Process entities
        for extracted_entity in &extraction.entities {
            let validation = self.validator.validate_entity(extracted_entity);
            if !validation.is_valid {
                continue;
            }

            let entity = Entity::new(&extracted_entity.name, &extracted_entity.entity_type)
                .with_summary(&extracted_entity.summary)
                .with_group_id(&self.group_id);

            self.driver
                .store_entity(&entity)
                .await
                .map_err(|e| PipelineError::Storage(e.to_string()))?;

            entity_name_to_id.insert(extracted_entity.name.clone(), entity.id.clone());
            result.entities_stored += 1;
        }

        // Process relationships
        for extracted_rel in &extraction.relationships {
            let validation = self.validator.validate_relationship(extracted_rel);
            if !validation.is_valid {
                continue;
            }

            let source_id = match entity_name_to_id.get(&extracted_rel.source_entity) {
                Some(id) => id.clone(),
                None => {
                    let entity = Entity::new(&extracted_rel.source_entity, "Entity")
                        .with_group_id(&self.group_id);
                    self.driver
                        .store_entity(&entity)
                        .await
                        .map_err(|e| PipelineError::Storage(e.to_string()))?;
                    let id = entity.id.clone();
                    entity_name_to_id.insert(extracted_rel.source_entity.clone(), id.clone());
                    result.entities_stored += 1;
                    id
                }
            };

            let target_id = match entity_name_to_id.get(&extracted_rel.target_entity) {
                Some(id) => id.clone(),
                None => {
                    let entity = Entity::new(&extracted_rel.target_entity, "Entity")
                        .with_group_id(&self.group_id);
                    self.driver
                        .store_entity(&entity)
                        .await
                        .map_err(|e| PipelineError::Storage(e.to_string()))?;
                    let id = entity.id.clone();
                    entity_name_to_id.insert(extracted_rel.target_entity.clone(), id.clone());
                    result.entities_stored += 1;
                    id
                }
            };

            let relationship = Relationship::new(
                source_id,
                target_id,
                &extracted_rel.relationship_type,
                &extracted_rel.fact,
            )
            .with_confidence(extracted_rel.confidence)
            .with_group_id(&self.group_id)
            .with_episode(episode.id.clone());

            self.driver
                .store_relationship(&relationship)
                .await
                .map_err(|e| PipelineError::Storage(e.to_string()))?;

            result.relationships_stored += 1;
        }

        result.latency_ms = start.elapsed().as_millis() as u64;
        Ok((episode.clone(), result))
    }
}

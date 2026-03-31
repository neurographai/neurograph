// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Core graph data structures for NeuroGraph.
//!
//! This module defines all node, edge, and metadata types that form
//! the temporal knowledge graph. The design is heavily influenced by:
//! - **Graphiti**: Bi-temporal model (valid_at / invalid_at on edges),
//!   SagaNode for conversation threading
//! - **GraphRAG**: Hierarchical community structure (level, parent_id)
//! - **Cognee**: Typed ontology system for entity classification

pub mod community;
pub mod entity;
pub mod episode;
pub mod fact;
pub mod ontology;
pub mod relationship;
pub mod saga;
pub mod schema;

pub use community::{Community, CommunityId, CommunityLevel};
pub use entity::{Entity, EntityId, EntityType};
pub use episode::{Episode, EpisodeId, EpisodeType};
pub use fact::{TemporalFact, TemporalValidity};
pub use ontology::{EntityTypeDefinition, Ontology, RelationshipTypeDefinition};
pub use relationship::{Relationship, RelationshipId};
pub use saga::{Saga, SagaId};
pub use schema::GraphSchema;

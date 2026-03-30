// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! NeuroGraph configuration with builder pattern and environment variable overrides.
//!
//! Influenced by:
//! - GraphRAG's extensive YAML-based config system
//! - Graphiti's constructor params (uri, user, password, llm_client, embedder)
//! - Cognee's zero-config `cognee.add()` simplicity

use serde::{Deserialize, Serialize};

use crate::graph::ontology::Ontology;
use crate::llm::config::LlmConfig;

/// Storage backend selection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum StorageBackend {
    /// Pure in-memory (no persistence). Fast, great for testing.
    #[default]
    Memory,
    /// Sled-backed embedded storage. Persistent, zero-config.
    Embedded {
        /// Path to the database directory.
        path: String,
    },
    // Future backends (Sprint 4+):
    // Neo4j { uri: String, user: String, password: String },
    // FalkorDB { uri: String },
    // Kuzu { path: String },
}

/// Embedding provider selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingProvider {
    /// Local hash-based embedder (no API key required).
    /// Good for testing and deduplication, not for semantic search.
    Local,
    /// OpenAI text-embedding-3-small.
    OpenAi { model: String },
    // Future:
    // FastEmbed { model: String },
    // Custom { base_url: String, model: String },
}

impl Default for EmbeddingProvider {
    fn default() -> Self {
        // Check if OpenAI API key is available
        if std::env::var("OPENAI_API_KEY").is_ok() {
            EmbeddingProvider::OpenAi {
                model: "text-embedding-3-small".to_string(),
            }
        } else {
            EmbeddingProvider::Local
        }
    }
}

/// Community detection algorithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunityAlgorithm {
    /// Leiden algorithm (from GraphRAG).
    Leiden {
        max_cluster_size: usize,
        resolution: f64,
    },
    /// Louvain algorithm (simpler, faster).
    Louvain { resolution: f64 },
    /// Disabled — no community detection.
    Disabled,
}

impl Default for CommunityAlgorithm {
    fn default() -> Self {
        CommunityAlgorithm::Leiden {
            max_cluster_size: 10,
            resolution: 1.0,
        }
    }
}

/// The main NeuroGraph configuration.
///
/// Supports builder pattern, environment variables, and sensible defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuroGraphConfig {
    /// Display name for this graph.
    pub name: String,

    /// Storage backend.
    pub storage: StorageBackend,

    /// LLM configuration.
    pub llm: LlmConfig,

    /// Embedding provider.
    pub embedding: EmbeddingProvider,

    /// Community detection algorithm.
    pub community: CommunityAlgorithm,

    /// Ontology configuration.
    pub ontology: Ontology,

    /// Default group ID for multi-tenant usage.
    pub default_group_id: String,

    /// Maximum corpus budget in USD (None = unlimited).
    pub budget_usd: Option<f64>,

    /// Whether to store raw episode content (large datasets may want to skip).
    pub store_raw_content: bool,

    /// Maximum concurrent LLM requests.
    pub max_concurrent_llm: usize,

    /// Whether to enable tracing/logging.
    pub enable_tracing: bool,
}

impl NeuroGraphConfig {
    /// Create a configuration builder.
    pub fn builder() -> NeuroGraphConfigBuilder {
        NeuroGraphConfigBuilder::default()
    }

    /// Create a zero-config configuration (in-memory, local embeddings).
    pub fn zero_config() -> Self {
        Self {
            name: "neurograph".into(),
            storage: StorageBackend::Memory,
            llm: LlmConfig::gpt4o_mini(),
            embedding: EmbeddingProvider::Local,
            community: CommunityAlgorithm::default(),
            ontology: Ontology::open(),
            default_group_id: "default".into(),
            budget_usd: None,
            store_raw_content: true,
            max_concurrent_llm: 10,
            enable_tracing: false,
        }
    }

    /// Create a production config with sled storage and OpenAI.
    pub fn production(storage_path: impl Into<String>) -> Self {
        Self {
            name: "neurograph".into(),
            storage: StorageBackend::Embedded {
                path: storage_path.into(),
            },
            llm: LlmConfig::gpt4o_mini(),
            embedding: EmbeddingProvider::default(), // Auto-detect OpenAI
            community: CommunityAlgorithm::default(),
            ontology: Ontology::open(),
            default_group_id: "default".into(),
            budget_usd: Some(10.0), // $10 default budget
            store_raw_content: true,
            max_concurrent_llm: 10,
            enable_tracing: true,
        }
    }
}

impl Default for NeuroGraphConfig {
    fn default() -> Self {
        Self::zero_config()
    }
}

/// Builder for NeuroGraphConfig.
#[derive(Debug, Default)]
pub struct NeuroGraphConfigBuilder {
    name: Option<String>,
    storage: Option<StorageBackend>,
    llm: Option<LlmConfig>,
    embedding: Option<EmbeddingProvider>,
    community: Option<CommunityAlgorithm>,
    ontology: Option<Ontology>,
    default_group_id: Option<String>,
    budget_usd: Option<f64>,
    store_raw_content: Option<bool>,
    max_concurrent_llm: Option<usize>,
    enable_tracing: Option<bool>,
}

impl NeuroGraphConfigBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn storage(mut self, storage: StorageBackend) -> Self {
        self.storage = Some(storage);
        self
    }

    pub fn memory(mut self) -> Self {
        self.storage = Some(StorageBackend::Memory);
        self
    }

    pub fn embedded(mut self, path: impl Into<String>) -> Self {
        self.storage = Some(StorageBackend::Embedded { path: path.into() });
        self
    }

    pub fn llm(mut self, llm: LlmConfig) -> Self {
        self.llm = Some(llm);
        self
    }

    pub fn embedding(mut self, provider: EmbeddingProvider) -> Self {
        self.embedding = Some(provider);
        self
    }

    pub fn openai_embeddings(mut self) -> Self {
        self.embedding = Some(EmbeddingProvider::OpenAi {
            model: "text-embedding-3-small".to_string(),
        });
        self
    }

    pub fn local_embeddings(mut self) -> Self {
        self.embedding = Some(EmbeddingProvider::Local);
        self
    }

    pub fn ontology(mut self, ontology: Ontology) -> Self {
        self.ontology = Some(ontology);
        self
    }

    pub fn group_id(mut self, group_id: impl Into<String>) -> Self {
        self.default_group_id = Some(group_id.into());
        self
    }

    pub fn budget(mut self, budget_usd: f64) -> Self {
        self.budget_usd = Some(budget_usd);
        self
    }

    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent_llm = Some(max);
        self
    }

    pub fn tracing(mut self, enabled: bool) -> Self {
        self.enable_tracing = Some(enabled);
        self
    }

    /// Build the configuration with defaults for unset fields.
    pub fn build(self) -> NeuroGraphConfig {
        let defaults = NeuroGraphConfig::zero_config();
        NeuroGraphConfig {
            name: self.name.unwrap_or(defaults.name),
            storage: self.storage.unwrap_or(defaults.storage),
            llm: self.llm.unwrap_or(defaults.llm),
            embedding: self.embedding.unwrap_or(defaults.embedding),
            community: self.community.unwrap_or(defaults.community),
            ontology: self.ontology.unwrap_or(defaults.ontology),
            default_group_id: self.default_group_id.unwrap_or(defaults.default_group_id),
            budget_usd: self.budget_usd.or(defaults.budget_usd),
            store_raw_content: self.store_raw_content.unwrap_or(defaults.store_raw_content),
            max_concurrent_llm: self
                .max_concurrent_llm
                .unwrap_or(defaults.max_concurrent_llm),
            enable_tracing: self.enable_tracing.unwrap_or(defaults.enable_tracing),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_config() {
        let config = NeuroGraphConfig::zero_config();
        assert!(matches!(config.storage, StorageBackend::Memory));
        assert!(matches!(config.embedding, EmbeddingProvider::Local));
    }

    #[test]
    fn test_builder() {
        let config = NeuroGraphConfig::builder()
            .name("myproject")
            .embedded("./data")
            .budget(5.0)
            .max_concurrent(20)
            .build();

        assert_eq!(config.name, "myproject");
        assert!(matches!(config.storage, StorageBackend::Embedded { .. }));
        assert_eq!(config.budget_usd, Some(5.0));
        assert_eq!(config.max_concurrent_llm, 20);
    }

    #[test]
    fn test_production_config() {
        let config = NeuroGraphConfig::production("./prod-data");
        assert!(matches!(config.storage, StorageBackend::Embedded { .. }));
        assert!(config.budget_usd.is_some());
        assert!(config.enable_tracing);
    }
}

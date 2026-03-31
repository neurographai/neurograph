// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Application state shared across all request handlers.

use crate::chat::history::ConversationHistory;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub graph: Arc<crate::NeuroGraph>,
    pub conversations: Arc<RwLock<HashMap<String, ConversationHistory>>>,
    pub paper_metadata: Arc<RwLock<Vec<PaperEntry>>>,
}

/// Metadata about an ingested paper.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaperEntry {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub pages: u32,
    pub chunks: usize,
    pub entities: usize,
    pub ingested_at: chrono::DateTime<chrono::Utc>,
}

impl AppState {
    pub async fn new(data_dir: Option<&std::path::Path>) -> anyhow::Result<Self> {
        let graph = if let Some(dir) = data_dir {
            crate::NeuroGraph::builder()
                .embedded(dir.to_string_lossy())
                .build()
                .await?
        } else {
            crate::NeuroGraph::builder().memory().build().await?
        };

        Ok(Self {
            graph: Arc::new(graph),
            conversations: Arc::new(RwLock::new(HashMap::new())),
            paper_metadata: Arc::new(RwLock::new(Vec::new())),
        })
    }
}

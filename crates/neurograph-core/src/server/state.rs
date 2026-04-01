// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Application state shared across all request handlers.

use crate::chat::history::ConversationHistory;
use crate::llm::router::LlmRouter;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Metadata about an uploaded file (for the embed pipeline).
#[derive(Debug, Clone)]
pub struct UploadedFile {
    pub path: PathBuf,
    pub filename: String,
    pub size_bytes: usize,
}

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub graph: Arc<crate::NeuroGraph>,
    pub conversations: Arc<RwLock<HashMap<String, ConversationHistory>>>,
    pub paper_metadata: Arc<RwLock<Vec<PaperEntry>>>,
    // LLM routing
    pub llm_router: Arc<LlmRouter>,
    // Embed pipeline fields
    pub upload_dir: PathBuf,
    pub uploaded_files: Arc<DashMap<String, UploadedFile>>,
    pub http_client: reqwest::Client,
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

        let upload_dir = PathBuf::from("./uploads");
        std::fs::create_dir_all(&upload_dir)?;

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        // Initialize LLM router from environment variables
        let llm_router = Arc::new(LlmRouter::from_env());
        tracing::info!(
            providers = ?llm_router.configured_providers().await,
            "LLM router initialized"
        );

        Ok(Self {
            graph: Arc::new(graph),
            conversations: Arc::new(RwLock::new(HashMap::new())),
            paper_metadata: Arc::new(RwLock::new(Vec::new())),
            llm_router,
            upload_dir,
            uploaded_files: Arc::new(DashMap::new()),
            http_client,
        })
    }
}

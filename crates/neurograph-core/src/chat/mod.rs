// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! RAG chat engine for conversational interaction with the knowledge graph.

pub mod context;
pub mod history;
pub mod rag;

#[cfg(feature = "chat")]
pub mod repl;

use serde::{Deserialize, Serialize};

/// A chat message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub sources: Vec<SourceCitation>,
}

/// The role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
    System,
}

/// A source citation attached to a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceCitation {
    pub paper_title: String,
    pub section: String,
    pub page: u32,
    pub chunk_text: String,
    pub relevance_score: f32,
}

/// Response from the chat engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub answer: String,
    pub sources: Vec<SourceCitation>,
    pub model: String,
    pub tokens_used: u32,
    pub thinking_time_ms: u64,
}

/// Configuration for the chat engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub model: String,
    pub provider: String,
    pub max_context_tokens: usize,
    pub max_response_tokens: usize,
    pub temperature: f32,
    pub top_k_chunks: usize,
    pub include_sources: bool,
    pub system_prompt: Option<String>,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            model: "llama3.2".to_string(),
            provider: "ollama".to_string(),
            max_context_tokens: 4096,
            max_response_tokens: 1024,
            temperature: 0.7,
            top_k_chunks: 10,
            include_sources: true,
            system_prompt: None,
        }
    }
}

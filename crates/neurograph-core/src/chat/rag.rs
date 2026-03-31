// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! RAG (Retrieval-Augmented Generation) pipeline.

use super::context::{build_context, build_rag_prompt, build_system_prompt, ScoredChunk};
use super::history::ConversationHistory;
use super::{ChatConfig, ChatResponse};
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;

/// The RAG pipeline connects the knowledge graph to an LLM.
pub struct RagPipeline {
    config: ChatConfig,
    llm_client: Arc<dyn LlmClient>,
}

/// Trait for LLM providers (Ollama, OpenAI, etc.)
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat_completion(
        &self, messages: &[serde_json::Value],
        max_tokens: usize, temperature: f32,
    ) -> Result<LlmResponse>;
    fn model_name(&self) -> &str;
}

/// Raw response from an LLM.
pub struct LlmResponse {
    pub text: String,
    pub tokens_used: u32,
}

impl RagPipeline {
    pub fn new(config: ChatConfig, llm_client: Arc<dyn LlmClient>) -> Self {
        Self { config, llm_client }
    }

    pub async fn answer(
        &self, question: &str, retrieved_chunks: Vec<ScoredChunk>,
        history: &ConversationHistory,
    ) -> Result<ChatResponse> {
        let start = Instant::now();

        let (context_text, sources) = build_context(
            &retrieved_chunks, self.config.max_context_tokens, self.config.top_k_chunks,
        );

        if context_text.is_empty() {
            return Ok(ChatResponse {
                answer: "I don't have enough information in the ingested papers to answer that question. \
                         Try ingesting more relevant papers with `neurograph ingest --pdf`.".to_string(),
                sources: Vec::new(),
                model: self.llm_client.model_name().to_string(),
                tokens_used: 0,
                thinking_time_ms: start.elapsed().as_millis() as u64,
            });
        }

        let system_prompt = build_system_prompt(self.config.system_prompt.as_deref());
        let messages = build_rag_prompt(question, &context_text, history.all(), &system_prompt);

        let llm_response = self.llm_client
            .chat_completion(&messages, self.config.max_response_tokens, self.config.temperature)
            .await?;

        Ok(ChatResponse {
            answer: llm_response.text,
            sources: if self.config.include_sources { sources } else { Vec::new() },
            model: self.llm_client.model_name().to_string(),
            tokens_used: llm_response.tokens_used,
            thinking_time_ms: start.elapsed().as_millis() as u64,
        })
    }
}

/// Ollama LLM client.
pub struct OllamaLlm {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaLlm {
    pub fn new(model: &str) -> Result<Self> {
        let base_url = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        Ok(Self {
            client: reqwest::Client::builder().timeout(std::time::Duration::from_secs(120)).build()?,
            base_url, model: model.to_string(),
        })
    }
}

#[async_trait::async_trait]
impl LlmClient for OllamaLlm {
    async fn chat_completion(
        &self, messages: &[serde_json::Value], _max_tokens: usize, temperature: f32,
    ) -> Result<LlmResponse> {
        let request = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "options": { "temperature": temperature }
        });

        let response = self.client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request).send().await
            .map_err(|e| anyhow::anyhow!("Ollama connection failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama chat failed ({}): {}", status, body));
        }

        let json: serde_json::Value = response.json().await?;
        let text = json.get("message").and_then(|m| m.get("content"))
            .and_then(|c| c.as_str()).unwrap_or("").to_string();
        let tokens = json.get("eval_count").and_then(|c| c.as_u64()).unwrap_or(0) as u32;

        Ok(LlmResponse { text, tokens_used: tokens })
    }

    fn model_name(&self) -> &str { &self.model }
}

/// OpenAI LLM client.
pub struct OpenAiLlm {
    client: reqwest::Client,
    model: String,
    api_key: String,
}

impl OpenAiLlm {
    pub fn new(model: &str) -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
        Ok(Self {
            client: reqwest::Client::builder().timeout(std::time::Duration::from_secs(60)).build()?,
            model: model.to_string(), api_key,
        })
    }
}

#[async_trait::async_trait]
impl LlmClient for OpenAiLlm {
    async fn chat_completion(
        &self, messages: &[serde_json::Value], max_tokens: usize, temperature: f32,
    ) -> Result<LlmResponse> {
        let request = serde_json::json!({
            "model": self.model, "messages": messages,
            "max_tokens": max_tokens, "temperature": temperature
        });

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenAI API error ({}): {}", status, body));
        }

        let json: serde_json::Value = response.json().await?;
        let text = json["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
        let tokens = json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32;

        Ok(LlmResponse { text, tokens_used: tokens })
    }

    fn model_name(&self) -> &str { &self.model }
}

/// Create an LLM client from a "provider:model" spec.
pub fn create_llm_client(spec: &str) -> Result<Arc<dyn LlmClient>> {
    let (provider, model) = spec.split_once(':').unwrap_or(("ollama", spec));
    match provider {
        "ollama" => Ok(Arc::new(OllamaLlm::new(model)?)),
        "openai" => Ok(Arc::new(OpenAiLlm::new(model)?)),
        _ => Err(anyhow::anyhow!("Unknown LLM provider '{}'. Use: ollama, openai", provider)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_llm_client_ollama() {
        assert!(create_llm_client("ollama:llama3.2").is_ok());
    }

    #[test]
    fn test_create_llm_client_default() {
        let client = create_llm_client("llama3.2").unwrap();
        assert_eq!(client.model_name(), "llama3.2");
    }
}

// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! OpenAI LLM client implementation using the `async-openai` crate.

use async_trait::async_trait;
use std::time::Instant;

use super::config::LlmConfig;
use super::traits::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, LlmResult, LlmUsage,
};

/// OpenAI client wrapping the `async-openai` crate.
pub struct OpenAiClient {
    client: async_openai::Client<async_openai::config::OpenAIConfig>,
    config: LlmConfig,
}

impl OpenAiClient {
    /// Create a new OpenAI client with the given config.
    pub fn new(config: LlmConfig) -> LlmResult<Self> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| {
                LlmError::ConfigError(
                    "OPENAI_API_KEY not set. Set it via env var or config.".into(),
                )
            })?;

        let mut oai_config = async_openai::config::OpenAIConfig::new().with_api_key(api_key);

        if let Some(ref base_url) = config.base_url {
            oai_config = oai_config.with_api_base(base_url);
        }

        let client = async_openai::Client::with_config(oai_config);

        Ok(Self { client, config })
    }

    /// Create with the default GPT-4o-mini config.
    pub fn default_model() -> LlmResult<Self> {
        Self::new(LlmConfig::gpt4o_mini())
    }
}

impl std::fmt::Debug for OpenAiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiClient")
            .field("model", &self.config.model)
            .finish()
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    fn model_name(&self) -> &str {
        &self.config.model
    }

    fn cost_per_token(&self) -> (f64, f64) {
        (
            self.config.input_cost_per_million / 1_000_000.0,
            self.config.output_cost_per_million / 1_000_000.0,
        )
    }

    async fn complete(&self, request: CompletionRequest) -> LlmResult<CompletionResponse> {
        use async_openai::types::{
            ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
            ChatCompletionRequestUserMessage, CreateChatCompletionRequestArgs,
        };

        let start = Instant::now();

        let mut messages: Vec<ChatCompletionRequestMessage> = Vec::new();

        // Build system prompt, adding JSON mode hint if needed
        let system_content = match (&request.system_prompt, request.json_mode) {
            (Some(sys), true) => Some(format!("{}\n\nIMPORTANT: You MUST respond with valid JSON only. No markdown, no explanation.", sys)),
            (Some(sys), false) => Some(sys.clone()),
            (None, true) => Some("You MUST respond with valid JSON only. No markdown, no explanation.".to_string()),
            (None, false) => None,
        };

        if let Some(ref system) = system_content {
            messages.push(ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessage {
                    content: async_openai::types::ChatCompletionRequestSystemMessageContent::Text(
                        system.clone(),
                    ),
                    name: None,
                },
            ));
        }

        messages.push(ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessage {
                content: async_openai::types::ChatCompletionRequestUserMessageContent::Text(
                    request.user_prompt.clone(),
                ),
                name: None,
            },
        ));

        let mut req_builder = CreateChatCompletionRequestArgs::default();
        req_builder
            .model(&self.config.model)
            .messages(messages)
            .temperature(request.temperature);

        if let Some(max_tokens) = request.max_tokens {
            req_builder.max_tokens(max_tokens as u16);
        }

        let oai_request = req_builder
            .build()
            .map_err(|e| LlmError::ApiError(e.to_string()))?;

        let response = self
            .client
            .chat()
            .create(oai_request)
            .await
            .map_err(|e| LlmError::ApiError(e.to_string()))?;

        let latency_ms = start.elapsed().as_millis() as u64;

        let content = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let input_tokens = response.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0);
        let output_tokens = response
            .usage
            .as_ref()
            .map(|u| u.completion_tokens)
            .unwrap_or(0);

        let cost_usd = self
            .config
            .calculate_cost(input_tokens, output_tokens);

        Ok(CompletionResponse {
            content,
            usage: LlmUsage {
                input_tokens,
                output_tokens,
                cost_usd,
                latency_ms,
                model: self.config.model.clone(),
            },
        })
    }
}

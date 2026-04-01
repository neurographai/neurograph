// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Settings + LLM provider management API endpoints.
//!
//! Routes:
//! - `GET  /api/v1/llm/providers` — List configured providers + health
//! - `GET  /api/v1/llm/models` — Full model catalog
//! - `POST /api/v1/llm/test` — Test a provider with a key
//! - `GET  /api/v1/llm/usage` — Token + cost breakdown
//! - `GET  /api/v1/llm/router/config` — Get router config
//! - `POST /api/v1/llm/router/config` — Update router config

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::routes::{api_error, ApiResponse};
use super::state::AppState;
use crate::llm::{
    self, registry, traits::LlmProvider, LlmConfig,
};

/// Create the settings/LLM sub-router.
pub fn settings_router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/llm/providers", get(list_providers))
        .route("/api/v1/llm/models", get(list_models))
        .route("/api/v1/llm/test", post(test_provider))
        .route("/api/v1/llm/usage", get(get_usage))
        .route(
            "/api/v1/llm/router/config",
            get(get_router_config).post(update_router_config),
        )
}

// ─── Types ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TestProviderRequest {
    pub provider: String,
    pub api_key: String,
    pub model: Option<String>,
}

#[derive(Serialize)]
pub struct ProviderStatus {
    pub provider: String,
    pub display_name: String,
    pub configured: bool,
    pub healthy: bool,
    pub latency_ms: Option<u64>,
    pub model: String,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct UsageReport {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_calls: u64,
    pub total_cost_usd: f64,
    pub by_prompt_type: Vec<PromptTypeUsage>,
}

#[derive(Serialize)]
pub struct PromptTypeUsage {
    pub prompt_type: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub call_count: u64,
    pub cost_usd: f64,
}

// ─── Handlers ──────────────────────────────────────────────────

/// GET /api/v1/llm/providers — List all providers and their status.
async fn list_providers(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<ProviderStatus>>> {
    let router = &state.llm_router;
    let configured = router.configured_providers().await;

    let all_providers = [
        (LlmProvider::OpenAI, "OpenAI", "gpt-4o-mini"),
        (LlmProvider::Anthropic, "Anthropic", "claude-sonnet-4-5"),
        (LlmProvider::Gemini, "Google Gemini", "gemini-2.5-flash"),
        (LlmProvider::XaiGrok, "xAI Grok", "grok-3-mini"),
        (LlmProvider::Groq, "Groq", "llama-3.3-70b"),
        (LlmProvider::Ollama, "Ollama (Local)", "llama3.2"),
    ];

    let mut statuses = Vec::new();
    for (provider, display_name, default_model) in &all_providers {
        let is_configured = configured.contains(provider);
        let (healthy, latency, model, error) = if is_configured {
            match router.check_health(provider).await {
                Ok(health) => (
                    health.healthy,
                    Some(health.latency_ms),
                    health
                        .available_models
                        .first()
                        .cloned()
                        .unwrap_or_else(|| default_model.to_string()),
                    health.error,
                ),
                Err(e) => (false, None, default_model.to_string(), Some(e.to_string())),
            }
        } else {
            (false, None, default_model.to_string(), None)
        };

        statuses.push(ProviderStatus {
            provider: provider.to_string(),
            display_name: display_name.to_string(),
            configured: is_configured,
            healthy,
            latency_ms: latency,
            model,
            error,
        });
    }

    ApiResponse::ok(statuses)
}

/// GET /api/v1/llm/models — Full model catalog.
async fn list_models() -> Json<ApiResponse<Vec<registry::ModelInfo>>> {
    ApiResponse::ok(registry::get_model_registry())
}

/// POST /api/v1/llm/test — Test a provider with the given API key.
async fn test_provider(
    State(state): State<AppState>,
    Json(body): Json<TestProviderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let provider: LlmProvider = body
        .provider
        .parse()
        .map_err(|_| api_error(&format!("Unknown provider: {}", body.provider)))?;

    let default_model = match provider {
        LlmProvider::OpenAI => "gpt-4o-mini",
        LlmProvider::Anthropic => "claude-3-5-haiku-20241022",
        LlmProvider::Gemini => "gemini-2.5-flash-preview-05-20",
        LlmProvider::XaiGrok => "grok-3-mini",
        LlmProvider::Groq => "llama-3.3-70b-versatile",
        LlmProvider::Ollama => "llama3.2",
    };

    let model = body.model.as_deref().unwrap_or(default_model);

    // Create a test client with the provided key
    let client: Arc<dyn llm::LlmClient> = match provider {
        LlmProvider::OpenAI => {
            let mut config = LlmConfig::gpt4o_mini();
            config.api_key = Some(body.api_key.clone());
            config.model = model.to_string();
            Arc::new(
                llm::openai::OpenAiClient::new(config)
                    .map_err(|e| api_error(&e.to_string()))?,
            )
        }
        LlmProvider::Anthropic => Arc::new(
            llm::providers::AnthropicClient::with_key(body.api_key.clone(), model)
                .map_err(|e| api_error(&e.to_string()))?,
        ),
        LlmProvider::Gemini => Arc::new(
            llm::providers::GeminiClient::with_key(body.api_key.clone(), model)
                .map_err(|e| api_error(&e.to_string()))?,
        ),
        LlmProvider::Groq => Arc::new(llm::providers::OpenAiCompatClient::groq_with_key(
            body.api_key.clone(),
            model,
        )),
        LlmProvider::XaiGrok => Arc::new(llm::providers::OpenAiCompatClient::xai_with_key(
            body.api_key.clone(),
            model,
        )),
        LlmProvider::Ollama => {
            Arc::new(llm::generic::GenericLlmClient::ollama(model))
        }
    };

    // Run health check
    let health = client
        .health_check()
        .await
        .map_err(|e| api_error(&e.to_string()))?;

    // If healthy, register the client with the router
    if health.healthy {
        state.llm_router.set_provider(provider, client).await;
    }

    Ok(ApiResponse::ok(serde_json::json!({
        "provider": provider.to_string(),
        "model": model,
        "healthy": health.healthy,
        "latency_ms": health.latency_ms,
        "error": health.error,
    })))
}

/// GET /api/v1/llm/usage — Token and cost breakdown.
async fn get_usage(State(state): State<AppState>) -> Json<ApiResponse<UsageReport>> {
    let tracker = &state.llm_router.token_tracker;
    let (total_in, total_out, total_calls) = tracker.totals();
    let total_cost = tracker.total_cost().await;
    let summary = tracker.get_summary().await;

    let by_prompt_type: Vec<PromptTypeUsage> = summary
        .iter()
        .map(|(pt, usage)| PromptTypeUsage {
            prompt_type: pt.to_string(),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            call_count: usage.call_count,
            cost_usd: usage.estimated_cost_usd,
        })
        .collect();

    ApiResponse::ok(UsageReport {
        total_input_tokens: total_in,
        total_output_tokens: total_out,
        total_calls,
        total_cost_usd: total_cost,
        by_prompt_type,
    })
}

/// GET /api/v1/llm/router/config — Get current routing config.
async fn get_router_config(
    State(state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let config = state.llm_router.get_config().await;
    ApiResponse::ok(serde_json::to_value(&config).unwrap_or_default())
}

/// POST /api/v1/llm/router/config — Update routing config.
async fn update_router_config(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let config: crate::llm::router::RouterConfig =
        serde_json::from_value(body).map_err(|e| api_error(&format!("Invalid config: {}", e)))?;

    state.llm_router.update_config(config.clone()).await;

    Ok(ApiResponse::ok(serde_json::json!({
        "status": "updated",
        "strategy": format!("{:?}", config.strategy),
    })))
}

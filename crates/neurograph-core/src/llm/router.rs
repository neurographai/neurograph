// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Smart LLM router — selects the best provider for each task.
//!
//! Supports multiple routing strategies:
//! - **TaskAware** (default): picks best provider per task type
//! - **CostOptimized**: cheapest provider that can do the job
//! - **LatencyOptimized**: fastest provider (Groq)
//! - **Fixed**: always use one provider
//! - **Fallback**: try primary, fall back on error

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::config::LlmConfig;
use super::generic::GenericLlmClient;
use super::openai::OpenAiClient;
use super::providers::{AnthropicClient, GeminiClient, OpenAiCompatClient};
use super::registry::TaskType;
use super::token_tracker::TokenTracker;
use super::traits::{LlmClient, LlmError, LlmProvider, LlmResult, ProviderHealth};

/// Routing strategy for multi-provider dispatch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    /// Always use this provider.
    Fixed,
    /// Use cheapest provider that meets quality bar.
    CostOptimized,
    /// Use fastest provider (Groq for most things).
    LatencyOptimized,
    /// Pick best provider for each task type.
    TaskAware,
    /// Try primary, fall back on error.
    Fallback,
}

impl Default for RoutingStrategy {
    fn default() -> Self {
        Self::TaskAware
    }
}

impl std::str::FromStr for RoutingStrategy {
    type Err = LlmError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fixed" => Ok(Self::Fixed),
            "cost_optimized" => Ok(Self::CostOptimized),
            "latency_optimized" => Ok(Self::LatencyOptimized),
            "task_aware" => Ok(Self::TaskAware),
            "fallback" => Ok(Self::Fallback),
            _ => Err(LlmError::ConfigError(format!(
                "Unknown routing strategy: {}",
                s
            ))),
        }
    }
}

/// Per-task override: use a specific provider + model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOverride {
    pub provider: LlmProvider,
    pub model: String,
}

/// Router configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    pub strategy: RoutingStrategy,
    pub preferred_provider: Option<LlmProvider>,
    pub fallback_chain: Vec<LlmProvider>,
    pub budget_limit_usd: Option<f64>,
    pub budget_alert_threshold: Option<f64>,
    pub task_overrides: HashMap<String, TaskOverride>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            strategy: RoutingStrategy::TaskAware,
            preferred_provider: None,
            fallback_chain: vec![
                LlmProvider::Groq,
                LlmProvider::OpenAI,
                LlmProvider::Gemini,
                LlmProvider::Ollama,
            ],
            budget_limit_usd: None,
            budget_alert_threshold: None,
            task_overrides: HashMap::new(),
        }
    }
}

/// The smart LLM router.
pub struct LlmRouter {
    providers: RwLock<HashMap<LlmProvider, Arc<dyn LlmClient>>>,
    config: RwLock<RouterConfig>,
    health_cache: RwLock<HashMap<LlmProvider, ProviderHealth>>,
    pub token_tracker: Arc<TokenTracker>,
}

impl LlmRouter {
    /// Create a new router from environment variables.
    pub fn from_env() -> Self {
        let mut providers: HashMap<LlmProvider, Arc<dyn LlmClient>> = HashMap::new();

        // Try to initialize each provider from env vars
        if let Ok(client) = OpenAiClient::new(LlmConfig::gpt4o_mini()) {
            providers.insert(LlmProvider::OpenAI, Arc::new(client));
        }

        if let Ok(client) = AnthropicClient::new(LlmConfig::anthropic_sonnet()) {
            providers.insert(LlmProvider::Anthropic, Arc::new(client));
        }

        if let Ok(client) = GeminiClient::new(LlmConfig::gemini_flash()) {
            providers.insert(LlmProvider::Gemini, Arc::new(client));
        }

        if std::env::var("XAI_API_KEY").is_ok() {
            providers.insert(
                LlmProvider::XaiGrok,
                Arc::new(OpenAiCompatClient::xai(LlmConfig::xai_grok_mini())),
            );
        }

        if std::env::var("GROQ_API_KEY").is_ok() {
            providers.insert(
                LlmProvider::Groq,
                Arc::new(OpenAiCompatClient::groq(LlmConfig::groq_llama())),
            );
        }

        // Ollama is always available (localhost, no key needed)
        providers.insert(
            LlmProvider::Ollama,
            Arc::new(GenericLlmClient::ollama("llama3.2")),
        );

        Self {
            providers: RwLock::new(providers),
            config: RwLock::new(RouterConfig::default()),
            health_cache: RwLock::new(HashMap::new()),
            token_tracker: Arc::new(TokenTracker::new()),
        }
    }

    /// Create with explicit config.
    pub fn with_config(config: RouterConfig) -> Self {
        let mut router = Self::from_env();
        *router.config.get_mut() = config;
        router
    }

    /// Get a provider client by type.
    pub async fn get_provider(&self, provider: &LlmProvider) -> LlmResult<Arc<dyn LlmClient>> {
        self.providers
            .read()
            .await
            .get(provider)
            .cloned()
            .ok_or_else(|| LlmError::UnknownProvider(format!("{}", provider)))
    }

    /// Route a request to the best provider for the given task.
    pub async fn route(&self, task: TaskType) -> LlmResult<Arc<dyn LlmClient>> {
        // Check budget
        if let Some(limit) = self.config.read().await.budget_limit_usd {
            let spent = self.token_tracker.total_cost().await;
            if spent >= limit {
                return Err(LlmError::BudgetExceeded { spent, limit });
            }
        }

        let config = self.config.read().await.clone();

        // Check for per-task override first
        let task_key = task.to_string();
        if let Some(override_cfg) = config.task_overrides.get(&task_key) {
            if let Ok(client) = self.get_provider(&override_cfg.provider).await {
                if self.is_healthy(&override_cfg.provider).await {
                    return Ok(client);
                }
            }
        }

        match config.strategy {
            RoutingStrategy::TaskAware => self.route_by_task(task, &config).await,
            RoutingStrategy::CostOptimized => self.route_cheapest(&config).await,
            RoutingStrategy::LatencyOptimized => self.route_fastest(&config).await,
            RoutingStrategy::Fallback => {
                let primary = config
                    .preferred_provider
                    .unwrap_or(LlmProvider::OpenAI);
                self.route_with_fallback(primary, &config).await
            }
            RoutingStrategy::Fixed => {
                let provider = config
                    .preferred_provider
                    .unwrap_or(LlmProvider::OpenAI);
                self.get_provider(&provider).await
            }
        }
    }

    /// Task-aware routing — the recommended strategy.
    async fn route_by_task(
        &self,
        task: TaskType,
        config: &RouterConfig,
    ) -> LlmResult<Arc<dyn LlmClient>> {
        let preferred = match task {
            // Intent classification: needs speed, not quality
            TaskType::IntentClassification => LlmProvider::Groq,
            // Entity extraction: needs structured JSON output
            TaskType::EntityExtraction => LlmProvider::OpenAI,
            // RAG answer generation: needs accuracy + long context
            TaskType::RagGeneration => LlmProvider::Anthropic,
            // Community summarization: huge context, medium output
            TaskType::CommunitySummary => LlmProvider::Gemini,
            // Follow-up question generation: fast, creative
            TaskType::FollowUpGeneration => LlmProvider::Groq,
            // Conflict detection: needs reasoning
            TaskType::ConflictDetection => LlmProvider::Anthropic,
            // Temporal analysis: reasoning helpful
            TaskType::TemporalAnalysis => LlmProvider::XaiGrok,
            // Deduplication: structured comparison
            TaskType::Deduplication => LlmProvider::OpenAI,
            // General chat: user preference
            TaskType::GeneralChat => config
                .preferred_provider
                .unwrap_or(LlmProvider::Anthropic),
        };

        if let Ok(client) = self.get_provider(&preferred).await {
            if self.is_healthy(&preferred).await {
                return Ok(client);
            }
        }

        // Fall back
        self.route_with_fallback(preferred, config).await
    }

    /// Cost-optimized: cheapest available provider.
    async fn route_cheapest(&self, config: &RouterConfig) -> LlmResult<Arc<dyn LlmClient>> {
        // Priority: Ollama (free) → Groq → Gemini Flash → GPT-4o-mini → Haiku
        let cost_order = [
            LlmProvider::Ollama,
            LlmProvider::Groq,
            LlmProvider::Gemini,
            LlmProvider::OpenAI,
            LlmProvider::XaiGrok,
            LlmProvider::Anthropic,
        ];

        for provider in &cost_order {
            if let Ok(client) = self.get_provider(provider).await {
                if self.is_healthy(provider).await {
                    return Ok(client);
                }
            }
        }

        self.any_available(config).await
    }

    /// Latency-optimized: fastest available provider.
    async fn route_fastest(&self, config: &RouterConfig) -> LlmResult<Arc<dyn LlmClient>> {
        // Groq is ~500 t/s, then GPT-4o-mini, then Gemini Flash, etc.
        let speed_order = [
            LlmProvider::Groq,
            LlmProvider::OpenAI,
            LlmProvider::Gemini,
            LlmProvider::XaiGrok,
            LlmProvider::Anthropic,
            LlmProvider::Ollama,
        ];

        for provider in &speed_order {
            if let Ok(client) = self.get_provider(provider).await {
                if self.is_healthy(provider).await {
                    return Ok(client);
                }
            }
        }

        self.any_available(config).await
    }

    /// Try primary, walk fallback chain on failure.
    async fn route_with_fallback(
        &self,
        primary: LlmProvider,
        config: &RouterConfig,
    ) -> LlmResult<Arc<dyn LlmClient>> {
        // Try primary
        if let Ok(client) = self.get_provider(&primary).await {
            if self.is_healthy(&primary).await {
                return Ok(client);
            }
        }

        // Walk fallback chain
        for fallback in &config.fallback_chain {
            if *fallback == primary {
                continue;
            }
            if let Ok(client) = self.get_provider(fallback).await {
                if self.is_healthy(fallback).await {
                    tracing::warn!(
                        "Primary {:?} unavailable, falling back to {:?}",
                        primary,
                        fallback
                    );
                    return Ok(client);
                }
            }
        }

        Err(LlmError::AllProvidersUnavailable)
    }

    /// Return any available provider as last resort.
    async fn any_available(&self, _config: &RouterConfig) -> LlmResult<Arc<dyn LlmClient>> {
        let providers = self.providers.read().await;
        for (provider, client) in providers.iter() {
            if self.is_healthy(provider).await {
                return Ok(client.clone());
            }
        }
        // Try Ollama as absolute last resort (always "available")
        providers
            .get(&LlmProvider::Ollama)
            .cloned()
            .ok_or(LlmError::AllProvidersUnavailable)
    }

    /// Check if a provider is healthy (cached, 60s TTL).
    pub async fn is_healthy(&self, provider: &LlmProvider) -> bool {
        let cache = self.health_cache.read().await;
        if let Some(health) = cache.get(provider) {
            let age = chrono::Utc::now() - health.checked_at;
            if age.num_seconds() < 60 {
                return health.healthy;
            }
        }
        // If no cache entry, assume healthy (will fail on actual use)
        true
    }

    /// Run a health check on a specific provider and cache the result.
    pub async fn check_health(&self, provider: &LlmProvider) -> LlmResult<ProviderHealth> {
        let client = self.get_provider(provider).await?;
        let health = client.health_check().await?;
        self.health_cache
            .write()
            .await
            .insert(*provider, health.clone());
        Ok(health)
    }

    /// List all configured providers.
    pub async fn configured_providers(&self) -> Vec<LlmProvider> {
        self.providers.read().await.keys().cloned().collect()
    }

    /// Add or replace a provider client at runtime (for settings flow).
    pub async fn set_provider(&self, provider: LlmProvider, client: Arc<dyn LlmClient>) {
        self.providers.write().await.insert(provider, client);
    }

    /// Remove a provider.
    pub async fn remove_provider(&self, provider: &LlmProvider) {
        self.providers.write().await.remove(provider);
    }

    /// Update routing config at runtime.
    pub async fn update_config(&self, config: RouterConfig) {
        *self.config.write().await = config;
    }

    /// Get current routing config.
    pub async fn get_config(&self) -> RouterConfig {
        self.config.read().await.clone()
    }
}

// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Static model registry — catalog of all supported models with pricing,
//! capabilities, speed tiers, and recommended task assignments.

use serde::{Deserialize, Serialize};

use super::traits::{LlmProvider, ModelCapabilities, SpeedTier};

/// Task types for routing decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    IntentClassification,
    EntityExtraction,
    RagGeneration,
    CommunitySummary,
    FollowUpGeneration,
    ConflictDetection,
    TemporalAnalysis,
    Deduplication,
    GeneralChat,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::IntentClassification => write!(f, "intent_classification"),
            TaskType::EntityExtraction => write!(f, "entity_extraction"),
            TaskType::RagGeneration => write!(f, "rag_generation"),
            TaskType::CommunitySummary => write!(f, "community_summary"),
            TaskType::FollowUpGeneration => write!(f, "follow_up_generation"),
            TaskType::ConflictDetection => write!(f, "conflict_detection"),
            TaskType::TemporalAnalysis => write!(f, "temporal_analysis"),
            TaskType::Deduplication => write!(f, "deduplication"),
            TaskType::GeneralChat => write!(f, "general_chat"),
        }
    }
}

impl std::str::FromStr for TaskType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "intent_classification" => Ok(TaskType::IntentClassification),
            "entity_extraction" => Ok(TaskType::EntityExtraction),
            "rag_generation" => Ok(TaskType::RagGeneration),
            "community_summary" => Ok(TaskType::CommunitySummary),
            "follow_up_generation" => Ok(TaskType::FollowUpGeneration),
            "conflict_detection" => Ok(TaskType::ConflictDetection),
            "temporal_analysis" => Ok(TaskType::TemporalAnalysis),
            "deduplication" => Ok(TaskType::Deduplication),
            "general_chat" => Ok(TaskType::GeneralChat),
            _ => Err(format!("Unknown task type: {}", s)),
        }
    }
}

/// Full model information for the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub provider: LlmProvider,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub input_cost_per_1m: f64,
    pub output_cost_per_1m: f64,
    pub capabilities: ModelCapabilities,
    pub recommended_for: Vec<TaskType>,
    pub speed_tier: SpeedTier,
}

/// Get the complete model registry.
pub fn get_model_registry() -> Vec<ModelInfo> {
    vec![
        // ── OPENAI ──────────────────────────────────────────────────
        ModelInfo {
            id: "gpt-4o".into(),
            display_name: "GPT-4o".into(),
            provider: LlmProvider::OpenAI,
            context_window: 128_000,
            max_output_tokens: 16_384,
            input_cost_per_1m: 2.50,
            output_cost_per_1m: 10.00,
            capabilities: ModelCapabilities {
                context_window: 128_000,
                max_output_tokens: 16_384,
                supports_streaming: true,
                supports_function_calling: true,
                supports_structured_output: true,
                supports_vision: true,
                supports_reasoning: false,
            },
            recommended_for: vec![TaskType::EntityExtraction, TaskType::RagGeneration],
            speed_tier: SpeedTier::Standard,
        },
        ModelInfo {
            id: "gpt-4o-mini".into(),
            display_name: "GPT-4o Mini".into(),
            provider: LlmProvider::OpenAI,
            context_window: 128_000,
            max_output_tokens: 16_384,
            input_cost_per_1m: 0.15,
            output_cost_per_1m: 0.60,
            capabilities: ModelCapabilities {
                context_window: 128_000,
                max_output_tokens: 16_384,
                supports_streaming: true,
                supports_function_calling: true,
                supports_structured_output: true,
                supports_vision: true,
                supports_reasoning: false,
            },
            recommended_for: vec![TaskType::Deduplication, TaskType::FollowUpGeneration],
            speed_tier: SpeedTier::Fast,
        },
        ModelInfo {
            id: "o4-mini".into(),
            display_name: "o4-mini (Reasoning)".into(),
            provider: LlmProvider::OpenAI,
            context_window: 200_000,
            max_output_tokens: 100_000,
            input_cost_per_1m: 1.10,
            output_cost_per_1m: 4.40,
            capabilities: ModelCapabilities {
                context_window: 200_000,
                max_output_tokens: 100_000,
                supports_streaming: true,
                supports_function_calling: true,
                supports_structured_output: true,
                supports_vision: true,
                supports_reasoning: true,
            },
            recommended_for: vec![TaskType::ConflictDetection],
            speed_tier: SpeedTier::Slow,
        },
        // ── ANTHROPIC ───────────────────────────────────────────────
        ModelInfo {
            id: "claude-sonnet-4-5-20250514".into(),
            display_name: "Claude Sonnet 4.5".into(),
            provider: LlmProvider::Anthropic,
            context_window: 200_000,
            max_output_tokens: 16_000,
            input_cost_per_1m: 3.00,
            output_cost_per_1m: 15.00,
            capabilities: ModelCapabilities {
                context_window: 200_000,
                max_output_tokens: 16_000,
                supports_streaming: true,
                supports_function_calling: true,
                supports_structured_output: true,
                supports_vision: true,
                supports_reasoning: false,
            },
            recommended_for: vec![
                TaskType::RagGeneration,
                TaskType::ConflictDetection,
                TaskType::GeneralChat,
            ],
            speed_tier: SpeedTier::Standard,
        },
        ModelInfo {
            id: "claude-3-5-haiku-20241022".into(),
            display_name: "Claude Haiku 3.5".into(),
            provider: LlmProvider::Anthropic,
            context_window: 200_000,
            max_output_tokens: 8_096,
            input_cost_per_1m: 0.80,
            output_cost_per_1m: 4.00,
            capabilities: ModelCapabilities {
                context_window: 200_000,
                max_output_tokens: 8_096,
                supports_streaming: true,
                supports_function_calling: true,
                supports_structured_output: true,
                supports_vision: false,
                supports_reasoning: false,
            },
            recommended_for: vec![
                TaskType::IntentClassification,
                TaskType::Deduplication,
            ],
            speed_tier: SpeedTier::Fast,
        },
        // ── GEMINI ──────────────────────────────────────────────────
        ModelInfo {
            id: "gemini-2.5-flash-preview-05-20".into(),
            display_name: "Gemini 2.5 Flash".into(),
            provider: LlmProvider::Gemini,
            context_window: 1_000_000,
            max_output_tokens: 65_536,
            input_cost_per_1m: 0.075,
            output_cost_per_1m: 0.30,
            capabilities: ModelCapabilities {
                context_window: 1_000_000,
                max_output_tokens: 65_536,
                supports_streaming: true,
                supports_function_calling: true,
                supports_structured_output: true,
                supports_vision: true,
                supports_reasoning: false,
            },
            recommended_for: vec![TaskType::CommunitySummary, TaskType::FollowUpGeneration],
            speed_tier: SpeedTier::Fast,
        },
        ModelInfo {
            id: "gemini-2.5-pro-preview-06-05".into(),
            display_name: "Gemini 2.5 Pro".into(),
            provider: LlmProvider::Gemini,
            context_window: 1_000_000,
            max_output_tokens: 65_536,
            input_cost_per_1m: 1.25,
            output_cost_per_1m: 10.00,
            capabilities: ModelCapabilities {
                context_window: 1_000_000,
                max_output_tokens: 65_536,
                supports_streaming: true,
                supports_function_calling: true,
                supports_structured_output: true,
                supports_vision: true,
                supports_reasoning: true,
            },
            recommended_for: vec![TaskType::CommunitySummary, TaskType::TemporalAnalysis],
            speed_tier: SpeedTier::Slow,
        },
        // ── XAI GROK ────────────────────────────────────────────────
        ModelInfo {
            id: "grok-3".into(),
            display_name: "Grok 3".into(),
            provider: LlmProvider::XaiGrok,
            context_window: 131_072,
            max_output_tokens: 131_072,
            input_cost_per_1m: 3.00,
            output_cost_per_1m: 15.00,
            capabilities: ModelCapabilities {
                context_window: 131_072,
                max_output_tokens: 131_072,
                supports_streaming: true,
                supports_function_calling: true,
                supports_structured_output: true,
                supports_vision: false,
                supports_reasoning: false,
            },
            recommended_for: vec![TaskType::TemporalAnalysis, TaskType::GeneralChat],
            speed_tier: SpeedTier::Standard,
        },
        ModelInfo {
            id: "grok-3-mini".into(),
            display_name: "Grok 3 Mini".into(),
            provider: LlmProvider::XaiGrok,
            context_window: 131_072,
            max_output_tokens: 131_072,
            input_cost_per_1m: 0.30,
            output_cost_per_1m: 0.50,
            capabilities: ModelCapabilities {
                context_window: 131_072,
                max_output_tokens: 131_072,
                supports_streaming: true,
                supports_function_calling: true,
                supports_structured_output: true,
                supports_vision: false,
                supports_reasoning: true,
            },
            recommended_for: vec![TaskType::IntentClassification],
            speed_tier: SpeedTier::Fast,
        },
        // ── GROQ ────────────────────────────────────────────────────
        ModelInfo {
            id: "llama-3.3-70b-versatile".into(),
            display_name: "Llama 3.3 70B (Groq)".into(),
            provider: LlmProvider::Groq,
            context_window: 128_000,
            max_output_tokens: 32_768,
            input_cost_per_1m: 0.59,
            output_cost_per_1m: 0.79,
            capabilities: ModelCapabilities {
                context_window: 128_000,
                max_output_tokens: 32_768,
                supports_streaming: true,
                supports_function_calling: true,
                supports_structured_output: true,
                supports_vision: false,
                supports_reasoning: false,
            },
            recommended_for: vec![
                TaskType::IntentClassification,
                TaskType::FollowUpGeneration,
                TaskType::GeneralChat,
            ],
            speed_tier: SpeedTier::Instant,
        },
        ModelInfo {
            id: "deepseek-r1-distill-llama-70b".into(),
            display_name: "DeepSeek R1 (Groq)".into(),
            provider: LlmProvider::Groq,
            context_window: 128_000,
            max_output_tokens: 32_768,
            input_cost_per_1m: 0.75,
            output_cost_per_1m: 0.99,
            capabilities: ModelCapabilities {
                context_window: 128_000,
                max_output_tokens: 32_768,
                supports_streaming: true,
                supports_function_calling: false,
                supports_structured_output: false,
                supports_vision: false,
                supports_reasoning: true,
            },
            recommended_for: vec![TaskType::ConflictDetection, TaskType::TemporalAnalysis],
            speed_tier: SpeedTier::Fast,
        },
    ]
}

/// Get models for a specific provider.
pub fn models_for_provider(provider: LlmProvider) -> Vec<ModelInfo> {
    get_model_registry()
        .into_iter()
        .filter(|m| m.provider == provider)
        .collect()
}

/// Get models recommended for a specific task.
pub fn models_for_task(task: TaskType) -> Vec<ModelInfo> {
    get_model_registry()
        .into_iter()
        .filter(|m| m.recommended_for.contains(&task))
        .collect()
}

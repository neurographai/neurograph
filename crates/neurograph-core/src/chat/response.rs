// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Agent response types — the structured 5-part response protocol.

use serde::{Deserialize, Serialize};

use super::intent::{ChatIntent, ClassifiedIntent};

/// The full structured response from the chat agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// Part 1: The answer text.
    pub answer: String,
    /// Answer confidence (0.0–1.0).
    pub confidence: f32,
    
    /// Part 2: Evidence that supports the answer.
    pub evidence: Vec<EvidenceChunk>,
    
    /// Part 3: Graph mutations for the dashboard.
    pub graph_actions: Vec<GraphAction>,
    
    /// Part 4: Suggested follow-up questions.
    pub follow_ups: Vec<FollowUpQuestion>,
    
    /// Part 5: Meta information about this response.
    pub meta: ResponseMeta,
}

/// A single piece of evidence supporting the answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceChunk {
    pub text: String,
    pub source: EvidenceSource,
    pub relevance_score: f32,
}

/// Where evidence comes from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvidenceSource {
    /// From an ingested paper.
    Paper {
        title: String,
        section: String,
        page: u32,
    },
    /// From a knowledge graph entity.
    Entity {
        entity_id: String,
        entity_name: String,
    },
    /// From a community summary.
    Community {
        community_id: u32,
        topic: String,
    },
    /// From a temporal snapshot.
    Temporal {
        timestamp: String,
        description: String,
    },
}

/// A graph action the dashboard should execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphAction {
    pub action_type: GraphActionType,
    pub description: String,
}

/// Types of graph mutations for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphActionType {
    /// Highlight specific nodes by ID.
    HighlightNodes { node_ids: Vec<String> },
    /// Expand a node to show its neighbors.
    ExpandSubgraph { center_node_id: String, depth: u32 },
    /// Switch the graph view filter.
    SwitchView { view: String },
    /// Filter edges by type.
    FilterEdges { edge_types: Vec<String> },
    /// Jump the timeline to a point.
    JumpTimeline { timestamp: String },
    /// Open the detail panel for a node.
    OpenNodePanel { node_id: String },
    /// Dim (de-emphasize) all nodes except these.
    FocusNodes { node_ids: Vec<String> },
    /// Reset graph to default state.
    ResetView,
}

/// A suggested follow-up question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUpQuestion {
    pub question: String,
    pub intent_hint: ChatIntent,
    pub rationale: String,
}

/// Metadata about the response generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    pub intent: ClassifiedIntent,
    pub tools_used: Vec<String>,
    pub model_used: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost_usd: f64,
    pub latency_ms: u64,
    pub session_id: String,
}

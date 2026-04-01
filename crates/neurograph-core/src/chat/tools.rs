// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Agent tools — retrieval tools, graph mutation tools, meta tools.
//!
//! Each tool is an enum variant with typed parameters.
//! The `ToolPlanner` maps intents to ordered tool sequences.
//! The `ToolExecutor` runs tools against the NeuroGraph instance.

use serde::{Deserialize, Serialize};

use super::intent::ChatIntent;
use super::response::{EvidenceChunk, EvidenceSource, GraphAction, GraphActionType};

/// All tools available to the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool", rename_all = "snake_case")]
pub enum AgentTool {
    // ── Retrieval tools (read graph, produce evidence) ──────────
    /// RAG retrieve from ingested chunks.
    RagRetrieve { query: String, top_k: usize },
    /// Look up a specific entity by name or ID.
    EntityLookup { entity_name: String },
    /// Get entity change history.
    EntityHistory { entity_id: String },
    /// Query community summaries.
    CommunityQuery { topic: String },
    /// Get a temporal snapshot at a specific time.
    TemporalSnapshot { timestamp: String },
    /// Find what changed between two timestamps.
    WhatChanged { from: String, to: String },
    /// Search academic papers.
    PaperSearch { query: String, limit: usize },
    /// Find contradictions in the graph.
    FindContradictions { entity_name: String },
    /// Trace relationship chain between two entities.
    TraceRelationship { from: String, to: String, max_depth: u32 },

    // ── Graph mutation tools (tell dashboard to update) ──────────
    /// Highlight specific nodes.
    HighlightNodes { node_ids: Vec<String> },
    /// Expand a subgraph around a node.
    ExpandSubgraph { center_node_id: String, depth: u32 },
    /// Switch the graph view mode.
    SwitchGraphView { view: String },
    /// Filter edges by type.
    FilterGraphEdges { edge_types: Vec<String> },
    /// Jump timeline to a timestamp.
    JumpToTimeline { timestamp: String },
    /// Open the detail panel for a node.
    OpenNodePanel { node_id: String },
    /// Reset the graph to default state.
    ResetGraphView,

    // ── Meta tools ──────────────────────────────────────────────
    /// Generate follow-up question suggestions.
    SuggestFollowUps { context: String },
    /// Explain the reasoning chain.
    ExplainReasoning { steps: Vec<String> },
}

impl AgentTool {
    /// Name for tracking.
    pub fn name(&self) -> &'static str {
        match self {
            AgentTool::RagRetrieve { .. } => "rag_retrieve",
            AgentTool::EntityLookup { .. } => "entity_lookup",
            AgentTool::EntityHistory { .. } => "entity_history",
            AgentTool::CommunityQuery { .. } => "community_query",
            AgentTool::TemporalSnapshot { .. } => "temporal_snapshot",
            AgentTool::WhatChanged { .. } => "what_changed",
            AgentTool::PaperSearch { .. } => "paper_search",
            AgentTool::FindContradictions { .. } => "find_contradictions",
            AgentTool::TraceRelationship { .. } => "trace_relationship",
            AgentTool::HighlightNodes { .. } => "highlight_nodes",
            AgentTool::ExpandSubgraph { .. } => "expand_subgraph",
            AgentTool::SwitchGraphView { .. } => "switch_graph_view",
            AgentTool::FilterGraphEdges { .. } => "filter_graph_edges",
            AgentTool::JumpToTimeline { .. } => "jump_to_timeline",
            AgentTool::OpenNodePanel { .. } => "open_node_panel",
            AgentTool::ResetGraphView => "reset_graph_view",
            AgentTool::SuggestFollowUps { .. } => "suggest_follow_ups",
            AgentTool::ExplainReasoning { .. } => "explain_reasoning",
        }
    }

    /// Whether this tool can run in parallel with others.
    pub fn is_parallel_safe(&self) -> bool {
        matches!(
            self,
            AgentTool::RagRetrieve { .. }
                | AgentTool::EntityLookup { .. }
                | AgentTool::EntityHistory { .. }
                | AgentTool::CommunityQuery { .. }
                | AgentTool::TemporalSnapshot { .. }
                | AgentTool::PaperSearch { .. }
        )
    }
}

/// Result after executing a tool.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_name: String,
    /// Evidence gathered (for retrieval tools).
    pub evidence: Vec<EvidenceChunk>,
    /// Graph actions to emit (for mutation tools).
    pub graph_actions: Vec<GraphAction>,
    /// Raw context text for the LLM prompt.
    pub context_text: String,
}

/// Plans which tools to use for a given intent.
pub struct ToolPlanner;

impl ToolPlanner {
    /// Given an intent and the user query, produce an ordered list of tools.
    pub fn plan(intent: &ChatIntent, query: &str, entities: &[String]) -> Vec<AgentTool> {
        match intent {
            ChatIntent::Explain => {
                let mut tools = vec![
                    AgentTool::RagRetrieve {
                        query: query.to_string(),
                        top_k: 10,
                    },
                ];
                for entity in entities.iter().take(3) {
                    tools.push(AgentTool::EntityLookup {
                        entity_name: entity.clone(),
                    });
                }
                if !entities.is_empty() {
                    tools.push(AgentTool::HighlightNodes {
                        node_ids: entities.to_vec(),
                    });
                }
                tools.push(AgentTool::SuggestFollowUps {
                    context: query.to_string(),
                });
                tools
            }

            ChatIntent::Explore => {
                let center = entities.first().cloned().unwrap_or_default();
                vec![
                    AgentTool::EntityLookup {
                        entity_name: center.clone(),
                    },
                    AgentTool::ExpandSubgraph {
                        center_node_id: center,
                        depth: 2,
                    },
                    AgentTool::SuggestFollowUps {
                        context: query.to_string(),
                    },
                ]
            }

            ChatIntent::TemporalCompare => {
                let entity = entities.first().cloned().unwrap_or_default();
                vec![
                    AgentTool::EntityHistory {
                        entity_id: entity.clone(),
                    },
                    AgentTool::RagRetrieve {
                        query: query.to_string(),
                        top_k: 10,
                    },
                    AgentTool::HighlightNodes {
                        node_ids: vec![entity],
                    },
                    AgentTool::SuggestFollowUps {
                        context: query.to_string(),
                    },
                ]
            }

            ChatIntent::TimeTravel => {
                vec![
                    AgentTool::TemporalSnapshot {
                        timestamp: "latest".to_string(),
                    },
                    AgentTool::RagRetrieve {
                        query: query.to_string(),
                        top_k: 5,
                    },
                    AgentTool::SuggestFollowUps {
                        context: query.to_string(),
                    },
                ]
            }

            ChatIntent::FindContradictions => {
                let entity = entities.first().cloned().unwrap_or_else(|| query.to_string());
                vec![
                    AgentTool::FindContradictions {
                        entity_name: entity.clone(),
                    },
                    AgentTool::RagRetrieve {
                        query: query.to_string(),
                        top_k: 10,
                    },
                    AgentTool::HighlightNodes {
                        node_ids: vec![entity],
                    },
                    AgentTool::SuggestFollowUps {
                        context: query.to_string(),
                    },
                ]
            }

            ChatIntent::Summarize => {
                vec![
                    AgentTool::CommunityQuery {
                        topic: query.to_string(),
                    },
                    AgentTool::RagRetrieve {
                        query: query.to_string(),
                        top_k: 15,
                    },
                    AgentTool::SuggestFollowUps {
                        context: query.to_string(),
                    },
                ]
            }

            ChatIntent::Search => {
                vec![
                    AgentTool::RagRetrieve {
                        query: query.to_string(),
                        top_k: 10,
                    },
                    AgentTool::PaperSearch {
                        query: query.to_string(),
                        limit: 10,
                    },
                    AgentTool::SuggestFollowUps {
                        context: query.to_string(),
                    },
                ]
            }

            ChatIntent::TraceRelationship => {
                let from = entities.first().cloned().unwrap_or_default();
                let to = entities.get(1).cloned().unwrap_or_default();
                vec![
                    AgentTool::TraceRelationship {
                        from: from.clone(),
                        to: to.clone(),
                        max_depth: 5,
                    },
                    AgentTool::HighlightNodes {
                        node_ids: vec![from, to],
                    },
                    AgentTool::SuggestFollowUps {
                        context: query.to_string(),
                    },
                ]
            }

            ChatIntent::DiscoverThemes => {
                vec![
                    AgentTool::CommunityQuery {
                        topic: query.to_string(),
                    },
                    AgentTool::SwitchGraphView {
                        view: "semantic".to_string(),
                    },
                    AgentTool::SuggestFollowUps {
                        context: query.to_string(),
                    },
                ]
            }

            ChatIntent::FilterGraph => {
                vec![
                    AgentTool::FilterGraphEdges {
                        edge_types: entities.to_vec(),
                    },
                    AgentTool::RagRetrieve {
                        query: query.to_string(),
                        top_k: 5,
                    },
                    AgentTool::SuggestFollowUps {
                        context: query.to_string(),
                    },
                ]
            }

            ChatIntent::General => {
                vec![
                    AgentTool::RagRetrieve {
                        query: query.to_string(),
                        top_k: 10,
                    },
                    AgentTool::SuggestFollowUps {
                        context: query.to_string(),
                    },
                ]
            }
        }
    }
}

/// Executes tools against the NeuroGraph instance.
///
/// This is the bridge between the abstract tool definitions and
/// the actual NeuroGraph engine.
pub struct ToolExecutor;

impl ToolExecutor {
    /// Execute a tool against the graph.
    ///
    /// For retrieval tools: queries the graph and returns evidence.
    /// For mutation tools: returns graph actions for the dashboard.
    pub async fn execute(
        tool: &AgentTool,
        graph: &crate::NeuroGraph,
    ) -> anyhow::Result<ToolResult> {
        match tool {
            AgentTool::RagRetrieve { query, top_k } => {
                let result = graph.query(query).await?;
                let evidence: Vec<EvidenceChunk> = result
                    .entities
                    .iter()
                    .take(*top_k)
                    .map(|e| EvidenceChunk {
                        text: format!("{}: {}", e.name, e.summary),
                        source: EvidenceSource::Entity {
                            entity_id: e.id.as_str(),
                            entity_name: e.name.clone(),
                        },
                        relevance_score: result.confidence as f32,
                    })
                    .collect();

                let context_text = result.answer.clone();
                Ok(ToolResult {
                    tool_name: "rag_retrieve".to_string(),
                    evidence,
                    graph_actions: vec![],
                    context_text,
                })
            }

            AgentTool::EntityLookup { entity_name } => {
                let results = graph.search_entities(entity_name, 5).await?;
                let evidence: Vec<EvidenceChunk> = results
                    .iter()
                    .map(|e| EvidenceChunk {
                        text: format!("{}: {}", e.name, e.summary),
                        source: EvidenceSource::Entity {
                            entity_id: e.id.as_str(),
                            entity_name: e.name.clone(),
                        },
                        relevance_score: 1.0,
                    })
                    .collect();

                let node_ids: Vec<String> = results.iter().map(|e| e.id.as_str()).collect();
                let context_text = results
                    .iter()
                    .map(|e| format!("Entity: {} — {}", e.name, e.summary))
                    .collect::<Vec<_>>()
                    .join("\n");

                let graph_actions = if !node_ids.is_empty() {
                    vec![GraphAction {
                        action_type: GraphActionType::HighlightNodes { node_ids },
                        description: format!("Highlighting entity: {}", entity_name),
                    }]
                } else {
                    vec![]
                };

                Ok(ToolResult {
                    tool_name: "entity_lookup".to_string(),
                    evidence,
                    graph_actions,
                    context_text,
                })
            }

            // Graph mutation tools — produce actions only, no evidence
            AgentTool::HighlightNodes { node_ids } => Ok(ToolResult {
                tool_name: "highlight_nodes".to_string(),
                evidence: vec![],
                graph_actions: vec![GraphAction {
                    action_type: GraphActionType::HighlightNodes {
                        node_ids: node_ids.clone(),
                    },
                    description: format!("Highlighting {} nodes", node_ids.len()),
                }],
                context_text: String::new(),
            }),

            AgentTool::ExpandSubgraph {
                center_node_id,
                depth,
            } => Ok(ToolResult {
                tool_name: "expand_subgraph".to_string(),
                evidence: vec![],
                graph_actions: vec![GraphAction {
                    action_type: GraphActionType::ExpandSubgraph {
                        center_node_id: center_node_id.clone(),
                        depth: *depth,
                    },
                    description: format!("Expanding subgraph from {}", center_node_id),
                }],
                context_text: String::new(),
            }),

            AgentTool::SwitchGraphView { view } => Ok(ToolResult {
                tool_name: "switch_graph_view".to_string(),
                evidence: vec![],
                graph_actions: vec![GraphAction {
                    action_type: GraphActionType::SwitchView {
                        view: view.clone(),
                    },
                    description: format!("Switching to {} view", view),
                }],
                context_text: String::new(),
            }),

            AgentTool::FilterGraphEdges { edge_types } => Ok(ToolResult {
                tool_name: "filter_graph_edges".to_string(),
                evidence: vec![],
                graph_actions: vec![GraphAction {
                    action_type: GraphActionType::FilterEdges {
                        edge_types: edge_types.clone(),
                    },
                    description: format!("Filtering edges: {:?}", edge_types),
                }],
                context_text: String::new(),
            }),

            AgentTool::ResetGraphView => Ok(ToolResult {
                tool_name: "reset_graph_view".to_string(),
                evidence: vec![],
                graph_actions: vec![GraphAction {
                    action_type: GraphActionType::ResetView,
                    description: "Resetting graph view".to_string(),
                }],
                context_text: String::new(),
            }),

            // Tools that need more graph internals — return placeholder for now
            _ => Ok(ToolResult {
                tool_name: tool.name().to_string(),
                evidence: vec![],
                graph_actions: vec![],
                context_text: format!("[{} — not yet implemented]", tool.name()),
            }),
        }
    }
}

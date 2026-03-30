// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! MCP tool implementations for NeuroGraph.
//!
//! Each tool maps to a core NeuroGraph operation:
//! - `add_memory` — Ingest text into the knowledge graph
//! - `search_memory` — Hybrid retrieval over the graph
//! - `time_travel` — Query the graph at a specific point in time
//! - `detect_communities` — Run community detection
//! - `memory_stats` — Get storage statistics

use neurograph_core::NeuroGraph;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::OnceCell;

/// Handler that manages the NeuroGraph instance and dispatches tool calls.
pub struct ToolHandler {
    ng: Arc<OnceCell<NeuroGraph>>,
}

impl ToolHandler {
    pub fn new() -> Self {
        Self {
            ng: Arc::new(OnceCell::new()),
        }
    }

    /// Lazily initialize the NeuroGraph instance with zero-config defaults.
    async fn get_ng(&self) -> Result<&NeuroGraph, String> {
        self.ng
            .get_or_try_init(|| async {
                NeuroGraph::builder()
                    .build()
                    .await
                    .map_err(|e| format!("Failed to initialize NeuroGraph: {}", e))
            })
            .await
            .map_err(|e| format!("{}", e))
    }

    /// List all available tools.
    pub fn list_tools(&self) -> Vec<Value> {
        vec![
            serde_json::json!({
                "name": "add_memory",
                "description": "Add a piece of knowledge to the NeuroGraph memory. Extracts entities, relationships, and temporal facts automatically.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "The text content to memorize"
                        },
                        "group_id": {
                            "type": "string",
                            "description": "Optional group/tenant ID for multi-tenant isolation"
                        }
                    },
                    "required": ["text"]
                }
            }),
            serde_json::json!({
                "name": "search_memory",
                "description": "Search the knowledge graph using hybrid retrieval (semantic + keyword + graph traversal). Returns relevant entities and relationships.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Natural language search query"
                        },
                        "max_results": {
                            "type": "integer",
                            "description": "Maximum number of results to return (default: 10)"
                        }
                    },
                    "required": ["query"]
                }
            }),
            serde_json::json!({
                "name": "time_travel",
                "description": "Query the knowledge graph as it existed at a specific point in time. Uses the bi-temporal model to reconstruct historical state.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "timestamp": {
                            "type": "string",
                            "description": "ISO 8601 timestamp to travel to (e.g., '2024-01-15T00:00:00Z')"
                        },
                        "query": {
                            "type": "string",
                            "description": "Optional query to run against the historical snapshot"
                        }
                    },
                    "required": ["timestamp"]
                }
            }),
            serde_json::json!({
                "name": "detect_communities",
                "description": "Run community detection (Louvain/Leiden) on the knowledge graph and return discovered communities with summaries.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "algorithm": {
                            "type": "string",
                            "description": "Algorithm to use: 'louvain' or 'leiden' (default: louvain)",
                            "enum": ["louvain", "leiden"]
                        }
                    }
                }
            }),
            serde_json::json!({
                "name": "memory_stats",
                "description": "Get comprehensive statistics about the NeuroGraph memory: entity count, relationship count, community count, storage usage, etc.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }),
            serde_json::json!({
                "name": "add_episode",
                "description": "Add an episodic memory from a conversation or interaction. Automatically extracts facts and updates the knowledge graph.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "The conversation or interaction content"
                        },
                        "source": {
                            "type": "string",
                            "description": "Source identifier (e.g., 'chat', 'email', 'meeting')"
                        }
                    },
                    "required": ["content"]
                }
            }),
            serde_json::json!({
                "name": "get_related_entities",
                "description": "Find entities related to a given entity through graph traversal. Returns connected entities within N hops.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_name": {
                            "type": "string",
                            "description": "Name of the entity to find relations for"
                        },
                        "max_hops": {
                            "type": "integer",
                            "description": "Maximum traversal depth (default: 2)"
                        }
                    },
                    "required": ["entity_name"]
                }
            }),
        ]
    }

    /// Dispatch a tool call to the appropriate handler.
    pub async fn call(&self, tool_name: &str, args: Value) -> Result<Value, String> {
        match tool_name {
            "add_memory" => self.add_memory(args).await,
            "search_memory" => self.search_memory(args).await,
            "time_travel" => self.time_travel(args).await,
            "detect_communities" => self.detect_communities(args).await,
            "memory_stats" => self.memory_stats(args).await,
            "add_episode" => self.add_episode(args).await,
            "get_related_entities" => self.get_related_entities(args).await,
            _ => Err(format!("Unknown tool: {}", tool_name)),
        }
    }

    async fn add_memory(&self, args: Value) -> Result<Value, String> {
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'text' argument")?;
        let group_id = args.get("group_id").and_then(|v| v.as_str());

        let ng = self.get_ng().await?;

        match ng.add_text(text).await {
            Ok(_) => Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("✅ Memorized: \"{}\" (group: {})", truncate(text, 100), group_id.unwrap_or("default"))
                }]
            })),
            Err(e) => Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("❌ Failed to add memory: {}", e)
                }],
                "isError": true
            })),
        }
    }

    async fn search_memory(&self, args: Value) -> Result<Value, String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'query' argument")?;

        let ng = self.get_ng().await?;

        match ng.query(query).await {
            Ok(result) => {
                let entities: Vec<Value> = result
                    .entities
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "name": e.name,
                            "type": e.entity_type,
                            "summary": e.summary
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": format!(
                            "🔍 Found {} entities (confidence: {:.0}%, cost: ${:.4})\n\n{}",
                            result.entities.len(),
                            result.confidence * 100.0,
                            result.cost_usd,
                            result.answer
                        )
                    }],
                    "entities": entities
                }))
            }
            Err(e) => Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("❌ Search failed: {}", e)
                }],
                "isError": true
            })),
        }
    }

    async fn time_travel(&self, args: Value) -> Result<Value, String> {
        let timestamp = args
            .get("timestamp")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'timestamp' argument")?;

        let ng = self.get_ng().await?;

        match ng.at(timestamp).await {
            Ok(snapshot) => {
                let query = args.get("query").and_then(|v| v.as_str());
                if let Some(q) = query {
                    match snapshot.query(q).await {
                        Ok(result) => Ok(serde_json::json!({
                            "content": [{
                                "type": "text",
                                "text": format!(
                                    "⏰ Time travel to {}\n\n{}",
                                    timestamp, result.answer
                                )
                            }]
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "content": [{
                                "type": "text",
                                "text": format!("❌ Query at {} failed: {}", timestamp, e)
                            }],
                            "isError": true
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": format!("⏰ Snapshot at {} created. Provide a query to search.", timestamp)
                        }]
                    }))
                }
            }
            Err(e) => Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("❌ Time travel failed: {}", e)
                }],
                "isError": true
            })),
        }
    }

    async fn detect_communities(&self, _args: Value) -> Result<Value, String> {
        let ng = self.get_ng().await?;

        match ng.detect_communities().await {
            Ok(result) => {
                let summaries: Vec<Value> = result
                    .communities
                    .iter()
                    .take(10)
                    .map(|c| {
                        serde_json::json!({
                            "id": c.id.as_str(),
                            "level": c.level,
                            "member_count": c.member_entity_ids.len(),
                            "summary": c.summary
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": format!(
                            "🏘️ Detected {} communities (modularity: {:.4})",
                            result.communities.len(),
                            result.modularity
                        )
                    }],
                    "communities": summaries
                }))
            }
            Err(e) => Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("❌ Community detection failed: {}", e)
                }],
                "isError": true
            })),
        }
    }

    async fn memory_stats(&self, _args: Value) -> Result<Value, String> {
        let ng = self.get_ng().await?;

        match ng.stats().await {
            Ok(stats) => Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "📊 NeuroGraph Stats:\n\
                        • Entities: {}\n\
                        • Relationships: {}\n\
                        • Episodes: {}\n\
                        • Communities: {}",
                        stats.get("entities").unwrap_or(&0),
                        stats.get("relationships").unwrap_or(&0),
                        stats.get("episodes").unwrap_or(&0),
                        stats.get("communities").unwrap_or(&0),
                    )
                }]
            })),
            Err(e) => Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("❌ Stats failed: {}", e)
                }],
                "isError": true
            })),
        }
    }

    async fn add_episode(&self, args: Value) -> Result<Value, String> {
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'content' argument")?;
        let source = args
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("chat");

        let ng = self.get_ng().await?;

        match ng.add_text(content).await {
            Ok(_) => Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("✅ Episode recorded from '{}' ({} chars)", source, content.len())
                }]
            })),
            Err(e) => Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("❌ Episode recording failed: {}", e)
                }],
                "isError": true
            })),
        }
    }

    async fn get_related_entities(&self, args: Value) -> Result<Value, String> {
        let entity_name = args
            .get("entity_name")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'entity_name' argument")?;

        let ng = self.get_ng().await?;

        match ng.query(&format!("entities related to {}", entity_name)).await {
            Ok(result) => {
                let entities: Vec<String> = result
                    .entities
                    .iter()
                    .map(|e| format!("• {} ({})", e.name, e.entity_type))
                    .collect();

                Ok(serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": format!(
                            "🔗 Entities related to '{}':\n\n{}",
                            entity_name,
                            if entities.is_empty() { "No related entities found.".to_string() } else { entities.join("\n") }
                        )
                    }]
                }))
            }
            Err(e) => Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("❌ Related entity lookup failed: {}", e)
                }],
                "isError": true
            })),
        }
    }
}

/// Truncate a string to max_len characters with "..." suffix.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

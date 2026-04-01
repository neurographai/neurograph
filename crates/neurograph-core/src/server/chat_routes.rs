// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Chat agent API endpoints + WebSocket streaming.
//!
//! Routes:
//! - `POST /api/v1/chat/agent` — Full agent loop → AgentResponse
//! - `POST /api/v1/chat/intent` — Classify intent only (for UI preview)
//! - `GET  /api/v1/chat/sessions` — List sessions
//! - `GET  /api/v1/chat/sessions/:id` — Get session history
//! - `DELETE /api/v1/chat/sessions/:id` — Delete session

use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

use super::routes::{api_error, ApiResponse};
use super::state::AppState;
use crate::chat::agent::NeuroGraphAgent;
use crate::chat::history::ConversationHistory;
use crate::chat::intent::IntentClassifier;
use crate::chat::response::AgentResponse;

/// Create the chat sub-router.
pub fn chat_router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/chat/agent", post(agent_chat))
        .route("/api/v1/chat/intent", post(classify_intent))
        .route("/api/v1/chat/sessions", get(list_sessions))
        .route(
            "/api/v1/chat/sessions/{id}",
            get(get_session).delete(delete_session),
        )
        .route("/api/v1/chat/suggest", post(suggest_questions))
}

// ─── Request / Response Types ──────────────────────────────────

#[derive(Deserialize)]
pub struct AgentChatRequest {
    pub message: String,
    pub session_id: Option<String>,
}

#[derive(Deserialize)]
pub struct IntentRequest {
    pub message: String,
}

#[derive(Deserialize)]
pub struct SuggestRequest {
    pub context: String,
}

#[derive(Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub message_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

// ─── Handlers ──────────────────────────────────────────────────

/// POST /api/v1/chat/agent — Run the full agent loop.
async fn agent_chat(
    State(state): State<AppState>,
    Json(body): Json<AgentChatRequest>,
) -> Result<Json<ApiResponse<AgentResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let session_id = body
        .session_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Get or create conversation history
    let mut history = {
        let conversations = state.conversations.read();
        conversations
            .get(&session_id)
            .cloned()
            .unwrap_or_else(ConversationHistory::new)
    };

    // Create the agent
    let agent = NeuroGraphAgent::new(state.graph.clone(), state.llm_router.clone());

    // Process the message
    let response = agent
        .process(&body.message, &session_id, &mut history)
        .await
        .map_err(|e| api_error(&format!("Agent error: {}", e)))?;

    // Save updated history
    {
        let mut conversations = state.conversations.write();
        conversations.insert(session_id.clone(), history);
    }

    Ok(ApiResponse::ok(response))
}

/// POST /api/v1/chat/intent — Classify intent only (fast, for UI preview).
async fn classify_intent(
    Json(body): Json<IntentRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let classifier = IntentClassifier::new();
    let classified = classifier.classify(&body.message);
    ApiResponse::ok(serde_json::json!({
        "intent": classified.intent,
        "confidence": classified.confidence,
        "method": classified.method,
        "label": classified.intent.display_label(),
        "entities": classified.extracted_entities,
    }))
}

/// GET /api/v1/chat/sessions — List all active sessions.
async fn list_sessions(State(state): State<AppState>) -> Json<ApiResponse<Vec<SessionInfo>>> {
    let conversations = state.conversations.read();
    let sessions: Vec<SessionInfo> = conversations
        .iter()
        .map(|(id, history)| SessionInfo {
            id: id.clone(),
            message_count: history.len(),
            created_at: history.created_at.to_rfc3339(),
            updated_at: history.updated_at.to_rfc3339(),
        })
        .collect();
    ApiResponse::ok(sessions)
}

/// GET /api/v1/chat/sessions/:id — Get session history.
async fn get_session(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let conversations = state.conversations.read();
    let history = conversations
        .get(&id)
        .ok_or_else(|| api_error("Session not found"))?;
    Ok(ApiResponse::ok(
        serde_json::to_value(history).unwrap_or_default(),
    ))
}

/// DELETE /api/v1/chat/sessions/:id — Delete a session.
async fn delete_session(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mut conversations = state.conversations.write();
    let removed = conversations.remove(&id).is_some();
    ApiResponse::ok(serde_json::json!({
        "removed": removed,
        "session_id": id,
    }))
}

/// POST /api/v1/chat/suggest — Get suggested initial questions.
async fn suggest_questions(
    State(state): State<AppState>,
    Json(_body): Json<SuggestRequest>,
) -> Json<ApiResponse<Vec<String>>> {
    // Suggest questions based on what's in the graph
    let stats = state.graph.stats().await.unwrap_or_default();
    let entity_count = stats.get("entities").copied().unwrap_or(0);

    let suggestions = if entity_count > 0 {
        vec![
            "What are the main topics in my knowledge graph?".to_string(),
            "Summarize the key relationships between entities".to_string(),
            "What themes emerge from the ingested papers?".to_string(),
            "Show me the most connected entities".to_string(),
            "Are there any contradictions in the data?".to_string(),
        ]
    } else {
        vec![
            "Upload a PDF to get started!".to_string(),
            "You can ingest text via the API too".to_string(),
        ]
    };

    ApiResponse::ok(suggestions)
}

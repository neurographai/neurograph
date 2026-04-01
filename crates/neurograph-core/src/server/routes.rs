// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! REST API route definitions and handlers.

use super::embed;
use super::state::{AppState, PaperEntry};
use axum::{
    extract::{DefaultBodyLimit, Multipart, Path as AxumPath, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

/// Create the API router with all routes.
pub fn create_router(state: AppState, _cors_origins: &[String]) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Merge chat agent + settings sub-routers
    let chat = super::chat_routes::chat_router();
    let settings = super::settings::settings_router();

    Router::new()
        // Health
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/stats", get(stats))
        // Papers
        .route("/api/v1/papers", get(list_papers))
        .route("/api/v1/papers/ingest", post(ingest_pdf))
        .route("/api/v1/papers/ingest-text", post(ingest_text))
        .route("/api/v1/papers/ingest-url", post(ingest_url))
        .route("/api/v1/papers/{id}", get(get_paper).delete(delete_paper))
        // Search
        .route("/api/v1/search/papers", post(search_papers))
        .route("/api/v1/search/entities", post(search_entities))
        // Knowledge graph
        .route("/api/v1/query", post(query))
        .route("/api/v1/graph", get(get_graph))
        // Chat (legacy simple endpoint)
        .route("/api/v1/chat", post(chat_simple))
        .route("/api/v1/chat/history", get(chat_history))
        // Embed pipeline (merged from neurograph-embed-server)
        .route("/api/v1/upload", post(embed::upload_pdf_handler))
        .route("/api/v1/models", get(embed::models_handler))
        .route("/ws/process", get(embed::ws_upgrade_handler))
        // Chat agent + settings sub-routers
        .merge(chat)
        .merge(settings)
        .with_state(state)
        .layer(cors)
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
}

// ─── Request / Response Types ──────────────────────────────────

#[derive(Deserialize)]
pub struct IngestTextRequest {
    pub text: String,
}

#[derive(Deserialize)]
pub struct IngestUrlRequest {
    pub url: String,
}

#[derive(Deserialize)]
pub struct QueryRequest {
    pub query: String,
    pub top_k: Option<usize>,
}

#[derive(Deserialize)]
pub struct SearchPapersRequest {
    pub query: String,
    pub source: Option<String>,
    pub limit: Option<usize>,
    pub since: Option<u16>,
}

#[derive(Deserialize)]
pub struct SearchEntitiesRequest {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub conversation_id: Option<String>,
}

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub(crate) fn ok(data: T) -> Json<Self> {
        Json(Self {
            success: true,
            data: Some(data),
            error: None,
        })
    }
}

pub(crate) fn api_error(msg: &str) -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        }),
    )
}

// ─── Handlers ──────────────────────────────────────────────────

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "service": "neurograph"
    }))
}

async fn stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let stats_map = state
        .graph
        .stats()
        .await
        .map_err(|e| api_error(&e.to_string()))?;
    let paper_count = state.paper_metadata.read().len();

    Ok(ApiResponse::ok(serde_json::json!({
        "entities": stats_map.get("entities").copied().unwrap_or(0),
        "relationships": stats_map.get("relationships").copied().unwrap_or(0),
        "papers": paper_count,
        "episodes": stats_map.get("episodes").copied().unwrap_or(0),
    })))
}

async fn ingest_text(
    State(state): State<AppState>,
    Json(body): Json<IngestTextRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    state
        .graph
        .add_text(&body.text)
        .await
        .map_err(|e| api_error(&e.to_string()))?;
    Ok(ApiResponse::ok(serde_json::json!({
        "status": "ingested",
        "text_length": body.text.len()
    })))
}

async fn ingest_pdf(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let filename = field.file_name().unwrap_or("upload.pdf").to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| api_error(&format!("Failed to read upload: {}", e)))?;

            // Write to a temp file for PDF parsing
            let tmp = tempfile::Builder::new()
                .prefix("neurograph-upload-")
                .suffix(".pdf")
                .tempfile()
                .map_err(|e| api_error(&format!("Temp file error: {}", e)))?;

            std::io::Write::write_all(&mut &tmp, &data)
                .map_err(|e| api_error(&format!("Write error: {}", e)))?;

            let parser = crate::pdf::PdfParser::new(crate::pdf::ParseStrategy::Auto);
            let doc = parser
                .parse_document(tmp.path())
                .map_err(|e| api_error(&format!("PDF parsing failed: {}", e)))?;

            for chunk in &doc.chunks {
                state
                    .graph
                    .add_text(&chunk.text)
                    .await
                    .map_err(|e| api_error(&e.to_string()))?;
            }

            let paper_id = uuid::Uuid::new_v4().to_string();
            let entry = PaperEntry {
                id: paper_id.clone(),
                title: filename.clone(),
                authors: Vec::new(),
                pages: doc.metadata.page_count,
                chunks: doc.chunks.len(),
                entities: 0,
                ingested_at: chrono::Utc::now(),
            };
            state.paper_metadata.write().push(entry);

            return Ok(ApiResponse::ok(serde_json::json!({
                "paper_id": paper_id,
                "title": filename,
                "pages": doc.metadata.page_count,
                "chunks": doc.chunks.len(),
                "words": doc.metadata.word_count,
            })));
        }
    }
    Err(api_error("No file field found in multipart upload"))
}

async fn ingest_url(
    State(state): State<AppState>,
    Json(body): Json<IngestUrlRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let client = reqwest::Client::new();
    let response = client
        .get(&body.url)
        .send()
        .await
        .map_err(|e| api_error(&format!("Failed to download: {}", e)))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|e| api_error(&format!("Failed to read response: {}", e)))?;

    // Write to temp file
    let tmp = tempfile::Builder::new()
        .prefix("neurograph-url-")
        .suffix(".pdf")
        .tempfile()
        .map_err(|e| api_error(&format!("Temp file error: {}", e)))?;

    std::io::Write::write_all(&mut &tmp, &bytes)
        .map_err(|e| api_error(&format!("Write error: {}", e)))?;

    let parser = crate::pdf::PdfParser::new(crate::pdf::ParseStrategy::Auto);
    let doc = parser
        .parse_document(tmp.path())
        .map_err(|e| api_error(&format!("PDF parsing failed: {}", e)))?;

    for chunk in &doc.chunks {
        state
            .graph
            .add_text(&chunk.text)
            .await
            .map_err(|e| api_error(&e.to_string()))?;
    }

    let paper_id = uuid::Uuid::new_v4().to_string();
    let entry = PaperEntry {
        id: paper_id.clone(),
        title: body.url.clone(),
        authors: Vec::new(),
        pages: doc.metadata.page_count,
        chunks: doc.chunks.len(),
        entities: 0,
        ingested_at: chrono::Utc::now(),
    };
    state.paper_metadata.write().push(entry);

    Ok(ApiResponse::ok(serde_json::json!({
        "paper_id": paper_id,
        "source_url": body.url,
        "chunks": doc.chunks.len(),
    })))
}

async fn list_papers(State(state): State<AppState>) -> Json<ApiResponse<Vec<PaperEntry>>> {
    let papers = state.paper_metadata.read().clone();
    ApiResponse::ok(papers)
}

async fn get_paper(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<ApiResponse<PaperEntry>>, (StatusCode, Json<ApiResponse<()>>)> {
    let papers = state.paper_metadata.read();
    let paper = papers
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| api_error("Paper not found"))?;
    Ok(ApiResponse::ok(paper))
}

async fn delete_paper(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mut papers = state.paper_metadata.write();
    let before = papers.len();
    papers.retain(|p| p.id != id);
    ApiResponse::ok(serde_json::json!({
        "removed": before != papers.len(),
        "paper_id": id
    }))
}

async fn query(
    State(state): State<AppState>,
    Json(body): Json<QueryRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let result = state
        .graph
        .query(&body.query)
        .await
        .map_err(|e| api_error(&e.to_string()))?;

    // Build reasoning path from entities involved
    let reasoning_path: Vec<serde_json::Value> = result
        .entities
        .iter()
        .enumerate()
        .map(|(i, e)| {
            serde_json::json!({
                "node": e.name,
                "relation": if i < result.relationships.len() {
                    result.relationships[i].relationship_type.clone()
                } else {
                    "related".to_string()
                },
                "confidence": result.confidence,
            })
        })
        .collect();

    Ok(ApiResponse::ok(serde_json::json!({
        "answer": result.answer,
        "confidence": result.confidence,
        "sources": result.entities.iter().map(|e| &e.name).collect::<Vec<_>>(),
        "entities": serde_json::to_value(&result.entities).unwrap_or_default(),
        "relationships": serde_json::to_value(&result.relationships).unwrap_or_default(),
        "reasoning_path": reasoning_path,
        "cost": {
            "model": "neurograph-local",
            "tokens": 0,
            "usd": result.cost_usd,
            "latency_ms": result.latency_ms,
        },
    })))
}

async fn search_papers(
    State(_state): State<AppState>,
    Json(body): Json<SearchPapersRequest>,
) -> Result<Json<ApiResponse<Vec<crate::papers::PaperResult>>>, (StatusCode, Json<ApiResponse<()>>)>
{
    #[cfg(feature = "paper-search")]
    {
        let search = crate::papers::aggregator::UnifiedPaperSearch::new();
        let config = crate::papers::SearchConfig {
            sources: if let Some(ref source) = body.source {
                vec![source
                    .parse()
                    .map_err(|e: anyhow::Error| api_error(&e.to_string()))?]
            } else {
                vec![
                    crate::papers::PaperSource::ArXiv,
                    crate::papers::PaperSource::SemanticScholar,
                ]
            },
            limit: body.limit.unwrap_or(20),
            since_year: body.since,
            sort_by: crate::papers::SortOrder::Relevance,
        };
        let results = search
            .search(&body.query, &config)
            .await
            .map_err(|e| api_error(&e.to_string()))?;
        Ok(ApiResponse::ok(results))
    }
    #[cfg(not(feature = "paper-search"))]
    {
        let _ = body;
        Err(api_error(
            "Paper search not enabled. Compile with --features paper-search",
        ))
    }
}

async fn search_entities(
    State(state): State<AppState>,
    Json(body): Json<SearchEntitiesRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let results = state
        .graph
        .search_entities(&body.query, body.limit.unwrap_or(10))
        .await
        .map_err(|e| api_error(&e.to_string()))?;
    Ok(ApiResponse::ok(
        serde_json::to_value(&results).unwrap_or_default(),
    ))
}

async fn get_graph(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let limit: usize = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(500);

    // Fetch entities
    let entities = state
        .graph
        .driver
        .list_entities(None, limit)
        .await
        .map_err(|e| api_error(&e.to_string()))?;

    // Fetch relationships for each entity (deduplicate by id)
    let mut relationships = Vec::new();
    let mut seen_rel_ids = std::collections::HashSet::new();
    for entity in &entities {
        if let Ok(rels) = state.graph.driver.get_entity_relationships(&entity.id).await {
            for rel in rels {
                if seen_rel_ids.insert(rel.id.clone()) {
                    relationships.push(rel);
                }
            }
        }
    }

    // Fetch communities
    let communities = state
        .graph
        .driver
        .list_communities(None)
        .await
        .unwrap_or_default();

    Ok(ApiResponse::ok(serde_json::json!({
        "entities": serde_json::to_value(&entities).unwrap_or_default(),
        "relationships": serde_json::to_value(&relationships).unwrap_or_default(),
        "communities": serde_json::to_value(&communities).unwrap_or_default(),
    })))
}

async fn chat_simple(
    State(state): State<AppState>,
    Json(body): Json<ChatRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let conv_id = body
        .conversation_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Add user message under lock, then drop the lock before awaiting
    {
        let mut conversations = state.conversations.write();
        let history = conversations
            .entry(conv_id.clone())
            .or_insert_with(crate::chat::history::ConversationHistory::new);
        history.add_user_message(&body.message);
    }

    let query_result = state
        .graph
        .query(&body.message)
        .await
        .map_err(|e| api_error(&e.to_string()))?;
    let answer = query_result.answer.clone();

    // Re-acquire lock to save assistant response
    {
        let mut conversations = state.conversations.write();
        if let Some(history) = conversations.get_mut(&conv_id) {
            history.add_assistant_message(&answer, vec![]);
        }
    }

    Ok(ApiResponse::ok(serde_json::json!({
        "conversation_id": conv_id,
        "answer": answer,
        "confidence": query_result.confidence,
        "sources": query_result.entities.iter().map(|e| &e.name).collect::<Vec<_>>(),
    })))
}

async fn chat_history(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let conversations = state.conversations.read();
    if let Some(conv_id) = params.get("conversation_id") {
        if let Some(history) = conversations.get(conv_id) {
            return ApiResponse::ok(serde_json::to_value(history).unwrap_or_default());
        }
    }
    let ids: Vec<&String> = conversations.keys().collect();
    ApiResponse::ok(serde_json::json!({ "conversations": ids }))
}

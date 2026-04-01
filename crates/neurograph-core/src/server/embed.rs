// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Embedding pipeline — PDF → Chunk → Embed → Similarity Graph via WebSocket.
//!
//! Merged from the standalone `neurograph-embed-server` crate into the core server
//! so that the dashboard, REST API, and embedding pipeline all run on a single port.

use super::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Multipart, Query, State,
    },
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{info, warn};

// ═══════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════

const DEFAULT_MODEL: &str = "qwen3-embedding:8b";
const CHUNK_SIZE: usize = 512;
const CHUNK_OVERLAP: usize = 64;
const DEFAULT_THRESHOLD: f64 = 0.82;
const MAX_EDGES_PER_NODE: usize = 5;

fn ollama_host() -> String {
    std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string())
}

// ═══════════════════════════════════════════════════════════════════
// WS Protocol Messages
// ═══════════════════════════════════════════════════════════════════

#[derive(Serialize)]
#[serde(tag = "type")]
enum WsOut {
    #[serde(rename = "status")]
    Status {
        step: String,
        message: String,
        progress: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        total_chunks: Option<usize>,
    },
    #[serde(rename = "node")]
    Node {
        id: String,
        label: String,
        category: String,
        x: f64,
        y: f64,
    },
    #[serde(rename = "edge")]
    Edge {
        source: String,
        target: String,
        similarity: f64,
    },
    #[serde(rename = "done")]
    Done {
        message: String,
        progress: u32,
        stats: PipelineStats,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Serialize, Clone)]
pub struct PipelineStats {
    pub total_chunks: usize,
    pub total_nodes: usize,
    pub concept_nodes: usize,
    pub chunk_nodes: usize,
    pub total_edges: usize,
    pub model: String,
    pub threshold: f64,
    pub text_length: usize,
    pub elapsed_ms: u64,
}

// ═══════════════════════════════════════════════════════════════════
// Text Chunking
// ═══════════════════════════════════════════════════════════════════

fn chunk_text(text: &str) -> Vec<String> {
    let text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chunks = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0;
    while start < bytes.len() {
        let end = (start + CHUNK_SIZE).min(bytes.len());
        let end = if end < bytes.len() {
            let mut e = end;
            while e > start && !text.is_char_boundary(e) {
                e -= 1;
            }
            e
        } else {
            end
        };
        let chunk = text[start..end].trim();
        if !chunk.is_empty() {
            chunks.push(chunk.to_string());
        }
        let step = if CHUNK_SIZE > CHUNK_OVERLAP {
            CHUNK_SIZE - CHUNK_OVERLAP
        } else {
            CHUNK_SIZE
        };
        start += step;
    }
    chunks
}

// ═══════════════════════════════════════════════════════════════════
// Concept Extraction
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
struct Concept {
    label: String,
    category: String,
}

fn extract_concepts(text: &str) -> Vec<Concept> {
    use std::collections::HashSet;

    static SKIP: &[&str] = &[
        "abstract",
        "introduction",
        "conclusion",
        "references",
        "acknowledgements",
        "appendix",
        "the",
        "and",
        "for",
        "with",
        "this",
        "that",
        "from",
        "which",
        "their",
        "these",
        "those",
        "been",
        "have",
        "will",
        "would",
        "could",
        "should",
        "also",
        "such",
    ];

    let skip_set: HashSet<&str> = SKIP.iter().copied().collect();
    let mut concepts = Vec::new();
    let mut seen = HashSet::new();

    // Capitalised noun phrases (e.g., "Knowledge Graph")
    let cap_re = regex::Regex::new(r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)\b").unwrap();
    for cap in cap_re.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let phrase = m.as_str();
            let key = phrase.to_lowercase();
            if key.len() > 4 && !skip_set.contains(key.as_str()) && seen.insert(key) {
                concepts.push(Concept {
                    label: phrase.to_string(),
                    category: "entity".into(),
                });
            }
        }
    }

    // Acronyms (2-6 uppercase letters)
    let acr_re = regex::Regex::new(r"\b([A-Z]{2,6})\b").unwrap();
    for cap in acr_re.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let acr = m.as_str();
            let key = acr.to_lowercase();
            if !skip_set.contains(key.as_str()) && seen.insert(key) {
                concepts.push(Concept {
                    label: acr.to_string(),
                    category: "acronym".into(),
                });
            }
        }
    }

    // Technical terms (common suffixes)
    let tech_re = regex::Regex::new(
        r"(?i)\b([a-z]{3,}(?:tion|sion|ment|ence|ance|ity|ism|ous|ive|ical|graph|net|former))\b",
    )
    .unwrap();
    for cap in tech_re.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let word = m.as_str();
            let key = word.to_lowercase();
            if key.len() > 5 && !skip_set.contains(key.as_str()) && seen.insert(key) {
                let mut c = word.chars();
                let titled: String = match c.next() {
                    Some(f) => f.to_uppercase().chain(c).collect(),
                    None => String::new(),
                };
                concepts.push(Concept {
                    label: titled,
                    category: "concept".into(),
                });
            }
        }
    }

    concepts
}

// ═══════════════════════════════════════════════════════════════════
// Ollama Embedding Client
// ═══════════════════════════════════════════════════════════════════

#[derive(Serialize)]
struct OllamaEmbedReq<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize)]
struct OllamaEmbedResp {
    #[serde(default)]
    embeddings: Vec<Vec<f64>>,
    #[serde(default)]
    embedding: Vec<f64>,
}

#[derive(Serialize)]
struct OllamaEmbedReqLegacy<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct OllamaEmbedRespLegacy {
    #[serde(default)]
    embedding: Vec<f64>,
}

async fn get_embedding(
    client: &reqwest::Client,
    text: &str,
    model: &str,
) -> anyhow::Result<Vec<f64>> {
    let host = ollama_host();

    // Try new /api/embed first
    let url = format!("{}/api/embed", host);
    let payload = OllamaEmbedReq {
        model,
        input: text,
    };

    match client.post(&url).json(&payload).send().await {
        Ok(resp) if resp.status().is_success() => {
            let data: OllamaEmbedResp = resp.json().await?;
            if let Some(first) = data.embeddings.into_iter().next() {
                return Ok(first);
            }
            if !data.embedding.is_empty() {
                return Ok(data.embedding);
            }
        }
        _ => {}
    }

    // Fallback to /api/embeddings
    let url2 = format!("{}/api/embeddings", host);
    let payload2 = OllamaEmbedReqLegacy {
        model,
        prompt: text,
    };
    let resp2 = client.post(&url2).json(&payload2).send().await?;
    resp2.error_for_status_ref()?;
    let data2: OllamaEmbedRespLegacy = resp2.json().await?;
    Ok(data2.embedding)
}

/// SHA256-based fallback embedding (deterministic, 32-dim)
fn hash_embedding(text: &str) -> Vec<f64> {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(text.as_bytes());
    hash.iter().map(|&b| b as f64 / 255.0).collect()
}

pub async fn list_ollama_models(client: &reqwest::Client) -> Vec<String> {
    let host = ollama_host();
    let url = format!("{}/api/tags", host);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            #[derive(Deserialize)]
            struct TagResp {
                #[serde(default)]
                models: Vec<ModelEntry>,
            }
            #[derive(Deserialize)]
            struct ModelEntry {
                name: String,
            }
            if let Ok(data) = resp.json::<TagResp>().await {
                return data.models.into_iter().map(|m| m.name).collect();
            }
        }
        _ => {}
    }
    vec![DEFAULT_MODEL.to_string()]
}

// ═══════════════════════════════════════════════════════════════════
// Cosine Similarity
// ═══════════════════════════════════════════════════════════════════

fn cosine_sim(a: &[f64], b: &[f64]) -> f64 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f64, 0.0f64, 0.0f64);
    for i in 0..len {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

// ═══════════════════════════════════════════════════════════════════
// Circle layout
// ═══════════════════════════════════════════════════════════════════

fn circle_positions(n: usize, spread: f64) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / n.max(1) as f64;
            (
                (spread * angle.cos() + spread).round(),
                (spread * angle.sin() + spread).round(),
            )
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════
// REST Endpoints
// ═══════════════════════════════════════════════════════════════════

pub async fn models_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let available = list_ollama_models(&state.http_client).await;
    Json(serde_json::json!({
        "models": available,
        "default": DEFAULT_MODEL
    }))
}

pub async fn upload_pdf_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    while let Ok(Some(field)) = multipart.next_field().await {
        let filename = field
            .file_name()
            .unwrap_or("upload.pdf")
            .to_string();

        if !filename.to_lowercase().ends_with(".pdf") {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Only PDF files are accepted"})),
            ));
        }

        let data = field.bytes().await.map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Read error: {}", e)})),
            )
        })?;

        let file_id = uuid::Uuid::new_v4().to_string();
        let dest = state.upload_dir.join(format!("{}.pdf", file_id));
        std::fs::write(&dest, &data).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Write error: {}", e)})),
            )
        })?;

        let size = data.len();
        state.uploaded_files.insert(
            file_id.clone(),
            super::state::UploadedFile {
                path: dest,
                filename: filename.clone(),
                size_bytes: size,
            },
        );

        info!("Uploaded {} → {} ({} bytes)", filename, file_id, size);

        return Ok(Json(serde_json::json!({
            "file_id": file_id,
            "filename": filename,
            "size_bytes": size,
        })));
    }

    Err((
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "No file in request"})),
    ))
}

// ═══════════════════════════════════════════════════════════════════
// WebSocket — Real-Time Pipeline
// ═══════════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct WsParams {
    file_id: String,
    #[serde(default = "default_model")]
    model: String,
    #[serde(default = "default_threshold")]
    threshold: f64,
}

fn default_model() -> String {
    DEFAULT_MODEL.to_string()
}
fn default_threshold() -> f64 {
    DEFAULT_THRESHOLD
}

pub async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_pipeline(socket, params, state))
}

async fn ws_pipeline(mut ws: WebSocket, params: WsParams, state: AppState) {
    info!(
        "WS connected — file_id={}, model={}, threshold={}",
        params.file_id, params.model, params.threshold
    );

    let pipeline_start = Instant::now();

    macro_rules! send {
        ($msg:expr) => {
            let json_str = serde_json::to_string(&$msg).unwrap();
            if ws
                .send(Message::Text(json_str.into()))
                .await
                .is_err()
            {
                warn!("WS send failed (client disconnected)");
                return;
            }
        };
    }

    // ── Validate ────────────────────────────────────────────────
    let file_entry = match state.uploaded_files.get(&params.file_id) {
        Some(f) => f,
        None => {
            send!(WsOut::Error {
                message: "File not found".into(),
            });
            return;
        }
    };
    let file_path = file_entry.path.clone();
    drop(file_entry);

    // ── Step 1: Extract text via PDF parser ──────────────────────
    send!(WsOut::Status {
        step: "extracting".into(),
        message: "Extracting text from PDF…".into(),
        progress: 0,
        total_chunks: None,
    });

    let raw_text = match tokio::task::spawn_blocking({
        let path = file_path.clone();
        move || {
            let parser =
                crate::pdf::PdfParser::new(crate::pdf::ParseStrategy::Fast);
            let doc = parser.parse_document(&path)?;
            Ok::<_, anyhow::Error>(doc.raw_text)
        }
    })
    .await
    {
        Ok(Ok(text)) => text,
        Ok(Err(e)) => {
            send!(WsOut::Error {
                message: format!("PDF extraction failed: {}", e),
            });
            return;
        }
        Err(e) => {
            send!(WsOut::Error {
                message: format!("Task panicked: {}", e),
            });
            return;
        }
    };

    if raw_text.trim().is_empty() {
        send!(WsOut::Error {
            message: "No text found in PDF".into(),
        });
        return;
    }

    let text_len = raw_text.len();
    send!(WsOut::Status {
        step: "extracting".into(),
        message: format!("Extracted {} characters", text_len),
        progress: 10,
        total_chunks: None,
    });

    // ── Step 2: Chunk ───────────────────────────────────────────
    send!(WsOut::Status {
        step: "chunking".into(),
        message: "Splitting into chunks…".into(),
        progress: 15,
        total_chunks: None,
    });

    let chunks = chunk_text(&raw_text);
    let total_chunks = chunks.len();

    send!(WsOut::Status {
        step: "chunking".into(),
        message: format!("Created {} chunks", total_chunks),
        progress: 20,
        total_chunks: Some(total_chunks),
    });

    // ── Step 3: Extract concepts + embed ────────────────────────
    send!(WsOut::Status {
        step: "embedding".into(),
        message: "Extracting concepts and computing embeddings…".into(),
        progress: 25,
        total_chunks: None,
    });

    let mut all_concepts: Vec<Concept> = Vec::new();
    let mut seen_labels = std::collections::HashSet::new();

    for chunk in &chunks {
        for c in extract_concepts(chunk) {
            let key = c.label.to_lowercase();
            if seen_labels.insert(key) {
                all_concepts.push(c);
            }
        }
    }

    // Add chunk nodes
    for chunk in &chunks {
        let short = if chunk.len() > 60 {
            format!("{}…", &chunk[..chunk.char_indices().nth(58).map(|(i, _)| i).unwrap_or(58)])
        } else {
            chunk.clone()
        };
        all_concepts.push(Concept {
            label: short,
            category: "chunk".into(),
        });
    }

    let total_nodes = all_concepts.len();
    let positions = circle_positions(total_nodes, 400.0);

    struct NodeEntry {
        id: String,
        #[allow(dead_code)]
        label: String,
        #[allow(dead_code)]
        category: String,
        embedding: Vec<f64>,
    }

    let mut nodes: Vec<NodeEntry> = Vec::with_capacity(total_nodes);

    for (idx, concept) in all_concepts.iter().enumerate() {
        let vec = match get_embedding(&state.http_client, &concept.label, &params.model).await {
            Ok(v) if !v.is_empty() => v,
            Ok(_) | Err(_) => {
                warn!("Embedding failed for '{}', using hash fallback", concept.label);
                hash_embedding(&concept.label)
            }
        };

        let node_id = format!("n-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let (x, y) = positions[idx];

        send!(WsOut::Node {
            id: node_id.clone(),
            label: concept.label.clone(),
            category: concept.category.clone(),
            x,
            y,
        });

        nodes.push(NodeEntry {
            id: node_id,
            label: concept.label.clone(),
            category: concept.category.clone(),
            embedding: vec,
        });

        let progress =
            25 + ((50 * (idx + 1)) as f64 / total_nodes as f64).round() as u32;
        if idx % (total_nodes / 20).max(1) == 0 || idx == total_nodes - 1 {
            send!(WsOut::Status {
                step: "embedding".into(),
                message: format!("Embedded {}/{} concepts", idx + 1, total_nodes),
                progress,
                total_chunks: None,
            });
        }

        tokio::task::yield_now().await;
    }

    // ── Step 4: Build similarity graph ──────────────────────────
    send!(WsOut::Status {
        step: "graphing".into(),
        message: "Computing similarity graph…".into(),
        progress: 78,
        total_chunks: None,
    });

    let mut edge_count = 0usize;

    for i in 0..total_nodes {
        let mut sims: Vec<(usize, f64)> = Vec::new();
        for j in (i + 1)..total_nodes {
            let s = cosine_sim(&nodes[i].embedding, &nodes[j].embedding);
            if s >= params.threshold {
                sims.push((j, s));
            }
        }
        sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        for (j, s) in sims.into_iter().take(MAX_EDGES_PER_NODE) {
            send!(WsOut::Edge {
                source: nodes[i].id.clone(),
                target: nodes[j].id.clone(),
                similarity: (s * 10000.0).round() / 10000.0,
            });
            edge_count += 1;
        }

        let progress = 78 + ((17 * (i + 1)) as f64 / total_nodes as f64).round() as u32;
        if i % (total_nodes / 10).max(1) == 0 || i == total_nodes - 1 {
            send!(WsOut::Status {
                step: "graphing".into(),
                message: format!("Processed {}/{} nodes for edges", i + 1, total_nodes),
                progress,
                total_chunks: None,
            });
        }

        tokio::task::yield_now().await;
    }

    // ── Done ────────────────────────────────────────────────────
    let elapsed = pipeline_start.elapsed().as_millis() as u64;
    let concept_count = total_nodes - chunks.len();

    let stats = PipelineStats {
        total_chunks,
        total_nodes,
        concept_nodes: concept_count,
        chunk_nodes: chunks.len(),
        total_edges: edge_count,
        model: params.model,
        threshold: params.threshold,
        text_length: text_len,
        elapsed_ms: elapsed,
    };

    info!(
        "Pipeline complete in {}ms — {} nodes, {} edges",
        elapsed, total_nodes, edge_count
    );

    send!(WsOut::Done {
        message: format!("Complete in {:.1}s", elapsed as f64 / 1000.0),
        progress: 100,
        stats,
    });
}

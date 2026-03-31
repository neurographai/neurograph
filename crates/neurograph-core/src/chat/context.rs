// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Context window builder for RAG.

use super::SourceCitation;

/// A text chunk reference for context building.
#[derive(Debug, Clone)]
pub struct ChunkRef {
    pub text: String,
    pub section: String,
    pub page: u32,
    pub token_count: usize,
}

/// A chunk with its relevance score for context building.
#[derive(Debug, Clone)]
pub struct ScoredChunk {
    pub chunk: ChunkRef,
    pub score: f32,
    pub paper_title: String,
}

/// Build the context window for a RAG query.
pub fn build_context(
    scored_chunks: &[ScoredChunk], max_tokens: usize, top_k: usize,
) -> (String, Vec<SourceCitation>) {
    let mut selected = scored_chunks.to_vec();
    selected.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    selected.truncate(top_k);
    selected = deduplicate_chunks(selected);

    let mut context = String::new();
    let mut total_tokens = 0;
    let mut sources = Vec::new();

    for (i, scored) in selected.iter().enumerate() {
        let chunk_tokens = scored.chunk.token_count;
        if total_tokens + chunk_tokens > max_tokens { continue; }

        context.push_str(&format!(
            "[Source {}] (Paper: \"{}\", Section: {}, Page {})\n{}\n\n",
            i + 1, scored.paper_title, scored.chunk.section, scored.chunk.page, scored.chunk.text
        ));

        sources.push(SourceCitation {
            paper_title: scored.paper_title.clone(),
            section: scored.chunk.section.clone(),
            page: scored.chunk.page,
            chunk_text: scored.chunk.text.clone(),
            relevance_score: scored.score,
        });

        total_tokens += chunk_tokens;
    }

    (context, sources)
}

fn deduplicate_chunks(chunks: Vec<ScoredChunk>) -> Vec<ScoredChunk> {
    let mut unique = Vec::new();
    for chunk in chunks {
        let chunk_words: std::collections::HashSet<&str> = chunk.chunk.text.split_whitespace().collect();
        let is_duplicate = unique.iter().any(|existing: &ScoredChunk| {
            let existing_words: std::collections::HashSet<&str> =
                existing.chunk.text.split_whitespace().collect();
            let overlap = chunk_words.intersection(&existing_words).count();
            let min_size = chunk_words.len().min(existing_words.len());
            if min_size == 0 { return false; }
            (overlap as f64 / min_size as f64) > 0.5
        });
        if !is_duplicate { unique.push(chunk); }
    }
    unique
}

/// Build the system prompt for RAG.
pub fn build_system_prompt(custom_prompt: Option<&str>) -> String {
    custom_prompt.unwrap_or(
        "You are NeuroGraph, a research paper intelligence assistant. \
         You answer questions based on the provided source documents. \
         Always cite your sources using [Source N] references. \
         If the provided context doesn't contain enough information, \
         say so honestly rather than guessing."
    ).to_string()
}

/// Build the full prompt for a RAG query.
pub fn build_rag_prompt(
    question: &str, context: &str,
    conversation_history: &[super::Message], system_prompt: &str,
) -> Vec<serde_json::Value> {
    let mut messages = Vec::new();
    messages.push(serde_json::json!({ "role": "system", "content": system_prompt }));

    let history_start = conversation_history.len().saturating_sub(5);
    for msg in &conversation_history[history_start..] {
        messages.push(serde_json::json!({
            "role": match msg.role {
                super::Role::User => "user",
                super::Role::Assistant => "assistant",
                super::Role::System => "system",
            },
            "content": msg.content
        }));
    }

    messages.push(serde_json::json!({
        "role": "user",
        "content": format!("Context from research papers:\n\n{}\n\n---\n\nQuestion: {}", context, question)
    }));

    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_scored_chunk(text: &str, score: f32) -> ScoredChunk {
        ScoredChunk {
            chunk: ChunkRef {
                text: text.to_string(),
                section: "Test".to_string(),
                page: 1,
                token_count: text.split_whitespace().count(),
            },
            score, paper_title: "Test Paper".to_string(),
        }
    }

    #[test]
    fn test_build_context_respects_budget() {
        let chunks = vec![
            make_scored_chunk(&"word ".repeat(100).trim(), 0.9),
            make_scored_chunk("different content here", 0.8),
        ];
        let (_context, sources) = build_context(&chunks, 50, 10);
        assert!(!sources.is_empty());
    }

    #[test]
    fn test_dedup_overlapping_chunks() {
        let chunks = vec![
            make_scored_chunk("the quick brown fox jumps over the lazy dog", 0.9),
            make_scored_chunk("the quick brown fox jumps over the lazy cat", 0.8),
            make_scored_chunk("completely different text about something else", 0.7),
        ];
        let deduped = deduplicate_chunks(chunks);
        assert_eq!(deduped.len(), 2);
    }
}

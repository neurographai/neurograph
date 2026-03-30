// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Text preprocessing and normalization utilities.

/// Normalize entity name for matching and deduplication.
pub fn normalize_name(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Extract key terms from text (simple whitespace tokenization).
pub fn extract_terms(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 2) // Skip very short words
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| !w.is_empty())
        .collect()
}

/// Truncate text to fit within a token budget.
/// Rough approximation: 1 token ≈ 4 characters for English text.
pub fn truncate_to_tokens(text: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens * 4;
    if text.len() <= max_chars {
        text.to_string()
    } else {
        let truncated = &text[..max_chars];
        // Find last complete word
        if let Some(last_space) = truncated.rfind(' ') {
            format!("{}...", &truncated[..last_space])
        } else {
            format!("{}...", truncated)
        }
    }
}

/// Estimate the number of tokens in a text.
/// Rough approximation: 1 token ≈ 4 characters for English.
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// Split text into chunks of approximately `max_tokens` each.
pub fn chunk_text(text: &str, max_tokens: usize) -> Vec<String> {
    let max_chars = max_tokens * 4;
    let sentences: Vec<&str> = text.split(['.', '!', '?', '\n']).collect();

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    for sentence in sentences {
        let sentence = sentence.trim();
        if sentence.is_empty() {
            continue;
        }

        if current_chunk.len() + sentence.len() + 2 > max_chars && !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
            current_chunk = String::new();
        }

        if !current_chunk.is_empty() {
            current_chunk.push_str(". ");
        }
        current_chunk.push_str(sentence);
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    if chunks.is_empty() && !text.is_empty() {
        chunks.push(text.to_string());
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_name() {
        assert_eq!(normalize_name("  Alice_Smith  "), "alice smith");
        assert_eq!(normalize_name("ALICE-SMITH"), "alice smith");
        assert_eq!(normalize_name("Alice  Smith"), "alice smith");
    }

    #[test]
    fn test_extract_terms() {
        let terms = extract_terms("Alice works at Anthropic in SF");
        assert!(terms.contains(&"alice".to_string()));
        assert!(terms.contains(&"works".to_string()));
        assert!(terms.contains(&"anthropic".to_string()));
        // "at", "in", "SF" are ≤2 chars (SF=2) so filtered
        assert!(!terms.contains(&"at".to_string()));
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("hello"), 2); // 5 chars → 2 tokens
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_chunk_text() {
        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.";
        let chunks = chunk_text(text, 10); // ~40 chars per chunk
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(chunk.len() <= 44); // Some tolerance for sentence boundaries
        }
    }

    #[test]
    fn test_truncate_to_tokens() {
        let text = "This is a test sentence that should be truncated";
        let truncated = truncate_to_tokens(text, 5); // ~20 chars
        assert!(truncated.len() <= 24); // 20 + "..."
        assert!(truncated.ends_with("..."));
    }
}

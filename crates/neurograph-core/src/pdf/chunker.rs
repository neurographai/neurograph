// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Section-aware smart chunking for parsed documents.

use super::types::{Chunk, Section};
use uuid::Uuid;

/// Configuration for the text chunker.
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    pub max_tokens: usize,
    pub min_tokens: usize,
    pub overlap_sentences: usize,
    pub respect_sections: bool,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            min_tokens: 50,
            overlap_sentences: 1,
            respect_sections: true,
        }
    }
}

/// Chunk raw text without section awareness.
pub fn chunk_text(text: &str, paper_id: &str, config: &ChunkConfig) -> Vec<Chunk> {
    let section = Section {
        heading: "Document".to_string(),
        level: 1,
        content: text.to_string(),
        page_start: 1,
        page_end: 1,
    };
    chunk_sections(&[section], paper_id, config)
}

/// Chunk text using section boundaries.
pub fn chunk_sections(sections: &[Section], paper_id: &str, config: &ChunkConfig) -> Vec<Chunk> {
    let mut all_chunks = Vec::new();

    for section in sections {
        if section.content.trim().is_empty() {
            continue;
        }
        let section_chunks = chunk_section_content(section, paper_id, config);
        all_chunks.extend(section_chunks);
    }

    merge_small_chunks(&mut all_chunks, config.min_tokens);
    all_chunks
}

fn chunk_section_content(section: &Section, paper_id: &str, config: &ChunkConfig) -> Vec<Chunk> {
    let paragraphs = split_paragraphs(&section.content);
    let mut chunks = Vec::new();
    let mut current_text = String::new();
    let mut current_tokens = 0usize;
    let mut char_offset = 0usize;

    for para in &paragraphs {
        let para_tokens = count_tokens(para);

        if current_tokens + para_tokens > config.max_tokens && !current_text.trim().is_empty() {
            chunks.push(create_chunk(
                &current_text, &section.heading, section.page_start,
                char_offset, current_tokens, paper_id,
            ));

            let overlap = extract_overlap(&current_text, config.overlap_sentences);
            char_offset += current_text.len() - overlap.len();
            current_text = overlap;
            current_tokens = count_tokens(&current_text);
        }

        if para_tokens > config.max_tokens {
            if !current_text.trim().is_empty() {
                chunks.push(create_chunk(
                    &current_text, &section.heading, section.page_start,
                    char_offset, current_tokens, paper_id,
                ));
                char_offset += current_text.len();
                current_text.clear();
                current_tokens = 0;
            }

            let sentence_chunks = split_large_paragraph(para, section, char_offset, paper_id, config);
            char_offset += para.len();
            chunks.extend(sentence_chunks);
            continue;
        }

        if !current_text.is_empty() {
            current_text.push_str("\n\n");
        }
        current_text.push_str(para);
        current_tokens += para_tokens;
    }

    if !current_text.trim().is_empty() && current_tokens >= config.min_tokens {
        chunks.push(create_chunk(
            &current_text, &section.heading, section.page_start,
            char_offset, current_tokens, paper_id,
        ));
    } else if !current_text.trim().is_empty() && !chunks.is_empty() {
        if let Some(last) = chunks.last_mut() {
            last.text.push_str("\n\n");
            last.text.push_str(current_text.trim());
            last.token_count += current_tokens;
        }
    } else if !current_text.trim().is_empty() {
        chunks.push(create_chunk(
            &current_text, &section.heading, section.page_start,
            char_offset, current_tokens, paper_id,
        ));
    }

    chunks
}

fn split_large_paragraph(
    text: &str, section: &Section, base_offset: usize,
    paper_id: &str, config: &ChunkConfig,
) -> Vec<Chunk> {
    let sentences = split_sentences(text);
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_tokens = 0;
    let mut offset = base_offset;

    for sentence in &sentences {
        let sent_tokens = count_tokens(sentence);

        if current_tokens + sent_tokens > config.max_tokens && !current.trim().is_empty() {
            chunks.push(create_chunk(
                &current, &section.heading, section.page_start,
                offset, current_tokens, paper_id,
            ));
            offset += current.len();
            current.clear();
            current_tokens = 0;
        }

        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(sentence);
        current_tokens += sent_tokens;
    }

    if !current.trim().is_empty() {
        chunks.push(create_chunk(
            &current, &section.heading, section.page_start,
            offset, current_tokens, paper_id,
        ));
    }

    chunks
}

fn create_chunk(
    text: &str, section: &str, page: u32,
    char_offset: usize, token_count: usize, paper_id: &str,
) -> Chunk {
    Chunk {
        id: Uuid::new_v4().to_string(),
        text: text.trim().to_string(),
        section: section.to_string(),
        page,
        char_offset,
        token_count,
        paper_id: paper_id.to_string(),
    }
}

fn split_paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

pub fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if (ch == '.' || ch == '!' || ch == '?') && current.len() > 10 {
            let trimmed = current.trim();
            let is_abbreviation = trimmed.ends_with("et al.")
                || trimmed.ends_with("Fig.")
                || trimmed.ends_with("Eq.")
                || trimmed.ends_with("Dr.")
                || trimmed.ends_with("Mr.")
                || trimmed.ends_with("vs.")
                || trimmed.ends_with("i.e.")
                || trimmed.ends_with("e.g.");

            if !is_abbreviation {
                sentences.push(current.trim().to_string());
                current = String::new();
            }
        }
    }

    if !current.trim().is_empty() {
        sentences.push(current.trim().to_string());
    }

    sentences
}

fn extract_overlap(text: &str, n: usize) -> String {
    if n == 0 {
        return String::new();
    }
    let sentences = split_sentences(text);
    let start = sentences.len().saturating_sub(n);
    sentences[start..].join(" ")
}

fn count_tokens(text: &str) -> usize {
    text.split_whitespace().count()
}

fn merge_small_chunks(chunks: &mut Vec<Chunk>, min_tokens: usize) {
    if chunks.len() <= 1 {
        return;
    }
    let mut i = 0;
    while i < chunks.len() {
        if chunks[i].token_count < min_tokens && i + 1 < chunks.len() {
            let next_text = chunks[i + 1].text.clone();
            let next_tokens = chunks[i + 1].token_count;
            chunks[i].text.push_str("\n\n");
            chunks[i].text.push_str(&next_text);
            chunks[i].token_count += next_tokens;
            chunks.remove(i + 1);
        } else {
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_basic() {
        let text = "This is a test paragraph with enough words to be chunked properly. \
                    It has multiple sentences. Each one adds to the total token count.\n\n\
                    Second paragraph here with more content for testing purposes. \
                    We want to make sure the chunker handles multiple paragraphs.";

        let config = ChunkConfig {
            max_tokens: 20,
            min_tokens: 5,
            ..Default::default()
        };

        let chunks = chunk_text(text, "test-paper", &config);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert_eq!(chunk.paper_id, "test-paper");
        }
    }

    #[test]
    fn test_count_tokens() {
        assert_eq!(count_tokens("hello world"), 2);
        assert_eq!(count_tokens(""), 0);
    }

    #[test]
    fn test_split_sentences() {
        let text = "First sentence here and more. Second sentence here too. Third one here.";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 3);
    }
}

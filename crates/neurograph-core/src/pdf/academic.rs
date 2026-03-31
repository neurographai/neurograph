// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Academic paper structure detection — titles, authors, abstracts, sections, references.

use super::chunker::{chunk_sections, ChunkConfig};
use super::types::{PaperMetadata, ParseStrategy, ParsedPaper, Reference, Section};
use chrono::Utc;
use std::path::Path;
use uuid::Uuid;

/// Parse raw text into an academic paper structure.
pub fn parse_academic_paper(
    raw_text: &str,
    source_path: Option<&Path>,
    strategy: ParseStrategy,
    parse_time_ms: u64,
) -> ParsedPaper {
    let paper_id = Uuid::new_v4().to_string();
    let title = detect_title(raw_text);
    let authors = detect_authors(raw_text);
    let abstract_text = detect_abstract(raw_text);
    let sections = detect_sections(raw_text);
    let references = extract_references(raw_text);

    let chunk_config = ChunkConfig::default();
    let chunks = if sections.is_empty() {
        super::chunker::chunk_text(raw_text, &paper_id, &chunk_config)
    } else {
        chunk_sections(&sections, &paper_id, &chunk_config)
    };

    let word_count = raw_text.split_whitespace().count();

    ParsedPaper {
        id: paper_id,
        title,
        authors,
        abstract_text,
        sections,
        references,
        chunks,
        metadata: PaperMetadata {
            source_path: source_path.map(|p| p.to_path_buf()),
            source_url: None,
            page_count: 0,
            word_count,
            char_count: raw_text.len(),
            parse_strategy: strategy,
            parse_time_ms,
            parsed_at: Utc::now(),
        },
        raw_text: raw_text.to_string(),
    }
}

fn detect_title(text: &str) -> String {
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    for line in lines.iter().take(10) {
        if line.len() < 5 { continue; }
        if line.starts_with("arXiv:") || line.starts_with("http") { continue; }
        if line.parse::<f64>().is_ok() { continue; }
        if line.len() < 200 {
            return line.to_string();
        }
    }
    "Untitled Document".to_string()
}

fn detect_authors(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    for line in lines.iter().skip(1).take(7) {
        let lower = line.to_lowercase();
        if lower.starts_with("abstract") || lower.starts_with("1.") || lower.starts_with("introduction") || line.len() > 300 {
            continue;
        }
        let authors = parse_author_line(line);
        if !authors.is_empty() {
            return authors;
        }
    }

    Vec::new()
}

fn parse_author_line(line: &str) -> Vec<String> {
    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if parts.len() >= 2 {
        let mut authors = Vec::new();
        for part in parts {
            let clean = part
                .replace(" and ", "")
                .replace('∗', "")
                .replace('†', "")
                .replace('‡', "")
                .trim()
                .to_string();

            let words: Vec<&str> = clean.split_whitespace().collect();
            if words.len() >= 2 && words.len() <= 5
                && !clean.chars().any(|c| c.is_ascii_digit())
                && clean.len() < 50
            {
                authors.push(clean);
            }
        }
        if authors.len() >= 2 {
            return authors;
        }
    }

    if line.contains('·') || line.contains('•') {
        let parts: Vec<&str> = line
            .split(|c| c == '·' || c == '•')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && s.len() < 50)
            .collect();
        if parts.len() >= 2 {
            return parts.iter().map(|s| s.to_string()).collect();
        }
    }

    Vec::new()
}

fn detect_abstract(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let abstract_markers = ["abstract\n", "abstract.", "abstract—", "abstract:"];

    for marker in &abstract_markers {
        if let Some(start_pos) = lower.find(marker) {
            let after_marker = start_pos + marker.len();
            let remaining = &text[after_marker..];
            let end = find_abstract_end(remaining);
            let abstract_text = remaining[..end].trim().to_string();
            if abstract_text.len() > 20 && abstract_text.len() < 5000 {
                return Some(abstract_text);
            }
        }
    }

    None
}

fn find_abstract_end(text: &str) -> usize {
    let section_markers = [
        "\n1.", "\n1 ", "\nI.", "\nI ",
        "\nintroduction", "\nINTRODUCTION",
        "\nkeywords", "\nKEYWORDS",
        "\nindex terms",
    ];

    let lower = text.to_lowercase();
    let mut earliest = text.len();
    for marker in &section_markers {
        if let Some(pos) = lower.find(marker) {
            if pos < earliest && pos > 20 {
                earliest = pos;
            }
        }
    }
    earliest
}

/// Detect section headings and split content into sections.
pub fn detect_sections(text: &str) -> Vec<Section> {
    let lines: Vec<&str> = text.lines().collect();
    let mut sections = Vec::new();
    let mut current_heading = String::new();
    let mut current_level: u8 = 1;
    let mut current_content = String::new();
    let mut current_page: u32 = 1;

    for line in &lines {
        let trimmed = line.trim();
        if let Some((level, heading)) = detect_heading(trimmed) {
            if !current_content.trim().is_empty() || !current_heading.is_empty() {
                sections.push(Section {
                    heading: if current_heading.is_empty() { "Preamble".to_string() } else { current_heading.clone() },
                    level: current_level,
                    content: current_content.trim().to_string(),
                    page_start: current_page,
                    page_end: current_page,
                });
            }
            current_heading = heading;
            current_level = level;
            current_content = String::new();
        } else {
            current_content.push_str(trimmed);
            current_content.push('\n');
        }
        if trimmed.contains('\x0C') { current_page += 1; }
    }

    if !current_content.trim().is_empty() {
        sections.push(Section {
            heading: if current_heading.is_empty() { "Content".to_string() } else { current_heading },
            level: current_level,
            content: current_content.trim().to_string(),
            page_start: current_page,
            page_end: current_page,
        });
    }

    sections
}

fn detect_heading(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.len() > 120 { return None; }

    if is_numbered_heading(trimmed, 3) { return Some((3, trimmed.to_string())); }
    if is_numbered_heading(trimmed, 2) { return Some((2, trimmed.to_string())); }
    if is_numbered_heading(trimmed, 1) { return Some((1, trimmed.to_string())); }

    // ALL CAPS short lines
    if trimmed.len() >= 3 && trimmed.len() <= 50
        && trimmed.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c.is_ascii_punctuation())
        && trimmed.chars().filter(|c| c.is_alphabetic()).count() >= 3
    {
        return Some((1, trimmed.to_string()));
    }

    let known_sections = [
        "abstract", "introduction", "background", "related work",
        "methodology", "methods", "method", "approach",
        "experiments", "experimental setup", "evaluation",
        "results", "discussion", "analysis",
        "conclusion", "conclusions", "future work",
        "references", "bibliography", "acknowledgments", "acknowledgements", "appendix",
    ];

    let lower = trimmed.to_lowercase();
    for section_name in &known_sections {
        if lower == *section_name || lower.starts_with(&format!("{} ", section_name)) {
            return Some((1, trimmed.to_string()));
        }
    }

    None
}

fn is_numbered_heading(line: &str, level: u8) -> bool {
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() { return false; }

    match level {
        1 => {
            chars[0].is_ascii_digit()
                && (chars.get(1) == Some(&'.') || chars.get(1) == Some(&' '))
                && line.len() < 100
                && line.split_whitespace().count() <= 8
        }
        2 => {
            chars[0].is_ascii_digit()
                && chars.get(1) == Some(&'.')
                && chars.get(2).map_or(false, |c| c.is_ascii_digit())
                && line.len() < 100
        }
        3 => {
            chars[0].is_ascii_digit()
                && line.contains('.')
                && line.matches('.').count() >= 2
                && line.len() < 100
        }
        _ => false,
    }
}

/// Extract references from the end of the paper.
pub fn extract_references(text: &str) -> Vec<Reference> {
    let lower = text.to_lowercase();
    let ref_markers = ["\nreferences\n", "\nreferences\r\n", "\nbibliography\n"];

    let mut ref_start = None;
    for marker in &ref_markers {
        if let Some(pos) = lower.find(marker) {
            ref_start = Some(pos + marker.len());
            break;
        }
    }

    let ref_text = match ref_start {
        Some(start) => &text[start..],
        None => return Vec::new(),
    };

    parse_reference_list(ref_text)
}

fn parse_reference_list(text: &str) -> Vec<Reference> {
    let mut references = Vec::new();
    let mut current_ref = String::new();
    let mut ref_index = 0usize;

    for line in text.lines() {
        let trimmed = line.trim();
        let is_new_ref = trimmed.starts_with('[')
            || (trimmed.chars().next().map_or(false, |c| c.is_ascii_digit())
                && (trimmed.contains(". ") || trimmed.contains("] ")));

        if is_new_ref && !current_ref.is_empty() {
            if let Some(reference) = parse_single_reference(&current_ref, ref_index) {
                references.push(reference);
                ref_index += 1;
            }
            current_ref = trimmed.to_string();
        } else if !trimmed.is_empty() {
            if !current_ref.is_empty() { current_ref.push(' '); }
            current_ref.push_str(trimmed);
        }
    }

    if !current_ref.is_empty() {
        if let Some(reference) = parse_single_reference(&current_ref, ref_index) {
            references.push(reference);
        }
    }

    references
}

fn parse_single_reference(raw: &str, index: usize) -> Option<Reference> {
    if raw.len() < 10 { return None; }

    Some(Reference {
        raw: raw.to_string(),
        title: extract_ref_title(raw),
        authors: extract_ref_authors(raw),
        year: extract_year(raw),
        doi: extract_doi(raw),
        arxiv_id: extract_arxiv_id(raw),
        index,
    })
}

fn extract_year(text: &str) -> Option<u16> {
    for i in 0..text.len().saturating_sub(3) {
        if let Ok(year) = text[i..i + 4].parse::<u16>() {
            if (1900..=2030).contains(&year) {
                return Some(year);
            }
        }
    }
    None
}

fn extract_doi(text: &str) -> Option<String> {
    if let Some(pos) = text.find("10.") {
        let rest = &text[pos..];
        let end = rest
            .find(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == ')')
            .unwrap_or(rest.len());
        let doi = rest[..end].trim_end_matches('.').to_string();
        if doi.contains('/') {
            return Some(doi);
        }
    }
    None
}

fn extract_arxiv_id(text: &str) -> Option<String> {
    if let Some(pos) = text.find("arXiv:") {
        let start = pos + 6;
        let rest = &text[start..];
        let end = rest
            .find(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == ']')
            .unwrap_or(rest.len());
        return Some(rest[..end].trim().to_string());
    }
    None
}

fn extract_ref_title(text: &str) -> Option<String> {
    if let Some(start) = text.find('"') {
        if let Some(end) = text[start + 1..].find('"') {
            let title = &text[start + 1..start + 1 + end];
            if title.len() > 5 {
                return Some(title.to_string());
            }
        }
    }
    None
}

fn extract_ref_authors(text: &str) -> Vec<String> {
    let year_pos = text.find(|c: char| c.is_ascii_digit()).unwrap_or(text.len());
    let quote_pos = text.find('"').unwrap_or(text.len());
    let end = year_pos.min(quote_pos);
    if end < 5 { return Vec::new(); }

    let author_text = &text[..end];
    let cleaned = author_text
        .trim_start_matches(|c: char| c == '[' || c.is_ascii_digit() || c == ']' || c == '.')
        .trim();

    if cleaned.is_empty() { return Vec::new(); }

    cleaned.split(|c| c == ',' || c == ';')
        .map(|s| s.trim().replace(" and ", "").trim().to_string())
        .filter(|s| !s.is_empty() && s.len() > 1)
        .take(10)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_title() {
        let text = "Attention Is All You Need\n\nAshish Vaswani et al.\n\nAbstract\n...";
        assert_eq!(detect_title(text), "Attention Is All You Need");
    }

    #[test]
    fn test_detect_abstract() {
        let text = "Title\n\nAbstract\nThis paper proposes a new architecture for the task.\n\n1. Introduction\nHere we begin.";
        let abs = detect_abstract(text);
        assert!(abs.is_some());
        assert!(abs.unwrap().contains("proposes a new architecture"));
    }

    #[test]
    fn test_extract_year() {
        assert_eq!(extract_year("Published in 2023"), Some(2023));
        assert_eq!(extract_year("(2017)"), Some(2017));
        assert_eq!(extract_year("no year"), None);
    }

    #[test]
    fn test_extract_doi() {
        let text = "doi: 10.1038/nature12373";
        assert_eq!(extract_doi(text), Some("10.1038/nature12373".to_string()));
    }

    #[test]
    fn test_extract_arxiv_id() {
        let text = "arXiv:1706.03762v5";
        assert_eq!(extract_arxiv_id(text), Some("1706.03762v5".to_string()));
    }

    #[test]
    fn test_detect_sections() {
        let text = "1. Introduction\nThis is the intro.\n\n2. Methods\nWe did this.\n\n3. Results\nResults here.";
        let sections = detect_sections(text);
        assert!(sections.len() >= 3);
    }
}

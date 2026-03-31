// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Auto-detect document characteristics and recommend a parse strategy.

use super::types::ParseStrategy;

/// Document characteristics detected by probing.
#[derive(Debug, Clone)]
pub struct DocumentProfile {
    pub has_extractable_text: bool,
    pub estimated_columns: u8,
    pub has_math_symbols: bool,
    pub has_tables: bool,
    pub has_section_numbers: bool,
    pub average_line_length: f32,
    pub word_count: usize,
    pub is_academic: bool,
}

/// Probe extracted text and classify the document.
pub fn classify_document(text: &str) -> DocumentProfile {
    let has_extractable_text = !text.trim().is_empty() && text.len() > 100;
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();

    let average_line_length = if lines.is_empty() {
        0.0
    } else {
        lines.iter().map(|l| l.len() as f32).sum::<f32>() / lines.len() as f32
    };

    let word_count = text.split_whitespace().count();

    let estimated_columns = if average_line_length < 45.0 && lines.len() > 50 {
        2
    } else {
        1
    };

    let math_chars = ['∑', '∫', '∂', '∇', '∀', '∃', '∈', '⊂', '≤', '≥', 'α', 'β', 'γ', 'θ', 'λ'];
    let has_math_symbols = math_chars.iter().any(|c| text.contains(*c))
        || text.contains("\\frac")
        || text.contains("\\sum")
        || text.contains("\\int");

    let has_tables = lines.iter().any(|l| {
        l.matches('|').count() >= 2 || l.matches('\t').count() >= 2
    });

    let has_section_numbers = detect_section_numbers(text);
    let is_academic = detect_academic_paper(text);

    DocumentProfile {
        has_extractable_text,
        estimated_columns,
        has_math_symbols,
        has_tables,
        has_section_numbers,
        average_line_length,
        word_count,
        is_academic,
    }
}

/// Recommend a parse strategy based on document profile.
pub fn recommend_strategy(profile: &DocumentProfile) -> ParseStrategy {
    if !profile.has_extractable_text {
        tracing::warn!("Document has no extractable text. Consider using an OCR tool first.");
        return ParseStrategy::Fast;
    }

    if profile.estimated_columns > 1 || profile.has_math_symbols || profile.has_tables {
        ParseStrategy::Structured
    } else {
        ParseStrategy::Fast
    }
}

fn detect_section_numbers(text: &str) -> bool {
    let patterns = ["1. ", "1 ", "2. ", "2 ", "I. ", "II. ", "1.1", "1.2", "2.1", "2.2"];
    let lines: Vec<&str> = text.lines().collect();
    let mut matches = 0;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.len() < 80 {
            for pat in &patterns {
                if trimmed.starts_with(pat) {
                    matches += 1;
                    break;
                }
            }
        }
    }

    matches >= 2
}

fn detect_academic_paper(text: &str) -> bool {
    let lower = text.to_lowercase();
    let mut score = 0u32;

    let strong_indicators = [
        "abstract", "introduction", "methodology", "related work",
        "conclusion", "references", "bibliography", "acknowledgments", "acknowledgements",
    ];

    for indicator in &strong_indicators {
        if lower.contains(indicator) {
            score += 2;
        }
    }

    let weak_indicators = [
        "et al.", "fig.", "table ", "equation", "theorem", "proof",
        "lemma", "corollary", "arxiv", "doi:", "issn", "journal", "proceedings",
    ];

    for indicator in &weak_indicators {
        if lower.contains(indicator) {
            score += 1;
        }
    }

    score >= 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_academic() {
        let text = r#"
            Attention Is All You Need

            Abstract
            The dominant sequence transduction models are based on complex recurrent or
            convolutional neural networks.

            1. Introduction
            Recurrent neural networks (Vaswani et al., 2017)

            2. Related Work
            The goal of reducing sequential computation...

            References
            [1] Bahdanau, D. Neural machine translation...
        "#;

        let profile = classify_document(text);
        assert!(profile.is_academic);
        assert!(profile.has_extractable_text);
    }

    #[test]
    fn test_classify_simple() {
        let text = "This is a simple document with no academic structure.";
        let profile = classify_document(text);
        assert!(!profile.is_academic);
        assert_eq!(recommend_strategy(&profile), ParseStrategy::Fast);
    }

    #[test]
    fn test_empty_document() {
        let profile = classify_document("");
        assert!(!profile.has_extractable_text);
    }
}

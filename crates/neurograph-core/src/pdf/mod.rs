// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! PDF parsing and ingestion module.
//!
//! Provides a two-tier PDF parsing architecture:
//! - **Fast**: `pdf-extract` for rapid text extraction (~1-5ms/page)
//! - **Structured**: Heuristic section detection for academic papers

pub mod academic;
pub mod chunker;
pub mod classifier;
pub mod fast;
pub mod types;

pub use types::*;

use std::path::Path;
use std::time::Instant;

/// Main PDF parser with configurable strategy.
pub struct PdfParser {
    config: PdfConfig,
}

impl PdfParser {
    /// Create a new parser with the given strategy.
    pub fn new(strategy: ParseStrategy) -> Self {
        Self {
            config: PdfConfig {
                strategy,
                ..Default::default()
            },
        }
    }

    /// Create a parser with full configuration.
    pub fn with_config(config: PdfConfig) -> Self {
        Self { config }
    }

    /// Parse a PDF file into a structured academic paper.
    pub fn parse_paper(&self, path: &Path) -> anyhow::Result<ParsedPaper> {
        let start = Instant::now();

        let raw_text = fast::extract_text(path)?;

        if raw_text.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "No text extracted from '{}'. The PDF may be image-based (scanned). \
                 Consider using an OCR tool first.",
                path.display()
            ));
        }

        let strategy = match self.config.strategy {
            ParseStrategy::Auto => {
                let profile = classifier::classify_document(&raw_text);
                let recommended = classifier::recommend_strategy(&profile);
                tracing::info!(
                    "Auto-detected strategy: {:?} (columns={}, academic={}, math={})",
                    recommended,
                    profile.estimated_columns,
                    profile.is_academic,
                    profile.has_math_symbols
                );
                recommended
            }
            other => other,
        };

        let parse_time_ms = start.elapsed().as_millis() as u64;

        let mut paper =
            academic::parse_academic_paper(&raw_text, Some(path), strategy, parse_time_ms);
        paper.metadata.page_count = fast::estimate_page_count(path).unwrap_or(1);

        Ok(paper)
    }

    /// Parse a PDF from bytes in memory.
    pub fn parse_paper_from_bytes(
        &self,
        bytes: &[u8],
        source_name: &str,
    ) -> anyhow::Result<ParsedPaper> {
        let start = Instant::now();

        let raw_text = fast::extract_text_from_bytes(bytes)?;

        if raw_text.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "No text extracted from '{}'. The PDF may be image-based.",
                source_name
            ));
        }

        let strategy = match self.config.strategy {
            ParseStrategy::Auto => {
                let profile = classifier::classify_document(&raw_text);
                classifier::recommend_strategy(&profile)
            }
            other => other,
        };

        let parse_time_ms = start.elapsed().as_millis() as u64;
        let paper = academic::parse_academic_paper(&raw_text, None, strategy, parse_time_ms);

        Ok(paper)
    }

    /// Parse a PDF into a simple document (non-academic).
    pub fn parse_document(&self, path: &Path) -> anyhow::Result<ParsedDocument> {
        let start = Instant::now();

        let raw_text = fast::extract_text(path)?;
        let pages = fast::extract_pages(path)?;

        let paper_id = uuid::Uuid::new_v4().to_string();
        let chunk_config = chunker::ChunkConfig {
            max_tokens: self.config.chunk_max_tokens,
            overlap_sentences: self.config.chunk_overlap_sentences,
            ..Default::default()
        };
        let chunks = chunker::chunk_text(&raw_text, &paper_id, &chunk_config);
        let parse_time_ms = start.elapsed().as_millis() as u64;

        Ok(ParsedDocument {
            id: paper_id,
            pages,
            chunks,
            metadata: PaperMetadata {
                source_path: Some(path.to_path_buf()),
                source_url: None,
                page_count: fast::estimate_page_count(path).unwrap_or(1),
                word_count: raw_text.split_whitespace().count(),
                char_count: raw_text.len(),
                parse_strategy: self.config.strategy,
                parse_time_ms,
                parsed_at: chrono::Utc::now(),
            },
            raw_text,
        })
    }
}

/// Parse all PDFs in a directory.
pub fn parse_directory(
    dir: &Path,
    strategy: ParseStrategy,
) -> anyhow::Result<Vec<anyhow::Result<ParsedPaper>>> {
    let parser = PdfParser::new(strategy);
    let mut results = Vec::new();

    let entries = std::fs::read_dir(dir)
        .map_err(|e| anyhow::anyhow!("Failed to read directory '{}': {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("pdf") {
            tracing::info!("Parsing: {}", path.display());
            results.push(parser.parse_paper(&path));
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = PdfParser::new(ParseStrategy::Fast);
        assert_eq!(parser.config.strategy, ParseStrategy::Fast);
    }

    #[test]
    fn test_strategy_from_str() {
        assert_eq!("fast".parse::<ParseStrategy>().unwrap(), ParseStrategy::Fast);
        assert_eq!("auto".parse::<ParseStrategy>().unwrap(), ParseStrategy::Auto);
        assert_eq!("structured".parse::<ParseStrategy>().unwrap(), ParseStrategy::Structured);
        assert!("invalid".parse::<ParseStrategy>().is_err());
    }
}

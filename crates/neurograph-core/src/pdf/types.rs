// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Core types for the PDF parser module.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Strategy for parsing PDFs — trades speed for accuracy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParseStrategy {
    /// Fast text extraction (~1ms/page). Best for simple, single-column docs.
    Fast,
    /// Structural analysis (~50-100ms/page). Detects sections, columns, headings.
    Structured,
    /// Auto-detect: probe the document and pick the best strategy.
    Auto,
}

impl Default for ParseStrategy {
    fn default() -> Self {
        Self::Auto
    }
}

impl std::str::FromStr for ParseStrategy {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fast" => Ok(Self::Fast),
            "structured" | "balanced" => Ok(Self::Structured),
            "auto" => Ok(Self::Auto),
            _ => Err(anyhow::anyhow!(
                "Unknown parse strategy '{}'. Use: fast, structured, auto",
                s
            )),
        }
    }
}

/// Raw content extracted from a single PDF page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContent {
    pub page_num: u32,
    pub text: String,
    pub word_count: usize,
}

/// A chunk of text ready for embedding and indexing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub text: String,
    pub section: String,
    pub page: u32,
    pub char_offset: usize,
    pub token_count: usize,
    pub paper_id: String,
}

/// A detected section in a paper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub heading: String,
    pub level: u8,
    pub content: String,
    pub page_start: u32,
    pub page_end: u32,
}

/// A parsed bibliographic reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub raw: String,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub year: Option<u16>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub index: usize,
}

/// Metadata about the paper and parsing process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperMetadata {
    pub source_path: Option<PathBuf>,
    pub source_url: Option<String>,
    pub page_count: u32,
    pub word_count: usize,
    pub char_count: usize,
    pub parse_strategy: ParseStrategy,
    pub parse_time_ms: u64,
    pub parsed_at: DateTime<Utc>,
}

/// Complete parsed representation of an academic paper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPaper {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: Option<String>,
    pub sections: Vec<Section>,
    pub references: Vec<Reference>,
    pub chunks: Vec<Chunk>,
    pub metadata: PaperMetadata,
    pub raw_text: String,
}

/// Complete parsed document (non-academic fallback).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDocument {
    pub id: String,
    pub pages: Vec<PageContent>,
    pub chunks: Vec<Chunk>,
    pub metadata: PaperMetadata,
    pub raw_text: String,
}

/// Report returned after ingesting a PDF into the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestReport {
    pub paper_id: String,
    pub title: String,
    pub pages_parsed: u32,
    pub chunks_created: usize,
    pub entities_extracted: usize,
    pub relationships_extracted: usize,
    pub parse_strategy: ParseStrategy,
    pub parse_time_ms: u64,
    pub embed_time_ms: u64,
    pub total_time_ms: u64,
}

/// Configuration for the PDF parser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfConfig {
    pub strategy: ParseStrategy,
    pub chunk_max_tokens: usize,
    pub chunk_overlap_sentences: usize,
    pub extract_references: bool,
    pub extract_metadata: bool,
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            strategy: ParseStrategy::Auto,
            chunk_max_tokens: 512,
            chunk_overlap_sentences: 1,
            extract_references: true,
            extract_metadata: true,
        }
    }
}

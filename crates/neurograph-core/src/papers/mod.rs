// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Research paper search across multiple academic sources.

#[cfg(feature = "paper-search")]
pub mod arxiv;
#[cfg(feature = "paper-search")]
pub mod semantic_scholar;
#[cfg(feature = "paper-search")]
pub mod pubmed;
#[cfg(feature = "paper-search")]
pub mod aggregator;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A research paper result from any source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperResult {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: Option<String>,
    pub year: Option<u16>,
    pub citation_count: Option<u32>,
    pub pdf_url: Option<String>,
    pub web_url: Option<String>,
    pub doi: Option<String>,
    pub source: PaperSource,
    pub categories: Vec<String>,
    pub published: Option<DateTime<Utc>>,
}

/// Source of a paper search result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaperSource {
    ArXiv,
    SemanticScholar,
    PubMed,
}

impl std::fmt::Display for PaperSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaperSource::ArXiv => write!(f, "arXiv"),
            PaperSource::SemanticScholar => write!(f, "S2"),
            PaperSource::PubMed => write!(f, "PubMed"),
        }
    }
}

/// Configuration for paper search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub sources: Vec<PaperSource>,
    pub limit: usize,
    pub since_year: Option<u16>,
    pub sort_by: SortOrder,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            sources: vec![PaperSource::ArXiv, PaperSource::SemanticScholar],
            limit: 20,
            since_year: None,
            sort_by: SortOrder::Relevance,
        }
    }
}

/// Sort order for search results.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SortOrder {
    Relevance,
    Citations,
    Date,
}

impl std::str::FromStr for PaperSource {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "arxiv" => Ok(PaperSource::ArXiv),
            "s2" | "semantic_scholar" | "semanticscholar" => Ok(PaperSource::SemanticScholar),
            "pubmed" | "pmc" => Ok(PaperSource::PubMed),
            _ => Err(anyhow::anyhow!("Unknown paper source '{}'. Use: arxiv, s2, pubmed", s)),
        }
    }
}

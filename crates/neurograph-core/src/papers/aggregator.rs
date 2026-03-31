// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Unified paper search across all sources with deduplication.

use super::*;
use anyhow::Result;

/// Unified paper search across arXiv, Semantic Scholar, and PubMed.
pub struct UnifiedPaperSearch {
    arxiv: arxiv::ArxivSearch,
    s2: semantic_scholar::SemanticScholarSearch,
    pubmed: pubmed::PubMedSearch,
}

impl UnifiedPaperSearch {
    pub fn new() -> Self {
        Self {
            arxiv: arxiv::ArxivSearch::new(),
            s2: semantic_scholar::SemanticScholarSearch::new(),
            pubmed: pubmed::PubMedSearch::new(),
        }
    }

    /// Search all configured sources in parallel.
    pub async fn search(&self, query: &str, config: &SearchConfig) -> Result<Vec<PaperResult>> {
        let per_source_limit = config.limit;
        let mut all_results = Vec::new();

        if config.sources.contains(&PaperSource::ArXiv) {
            match self.arxiv.search(query, per_source_limit, None).await {
                Ok(results) => all_results.extend(results),
                Err(e) => tracing::warn!("arXiv search failed: {}", e),
            }
        }

        if config.sources.contains(&PaperSource::SemanticScholar) {
            match self.s2.search(query, per_source_limit).await {
                Ok(results) => all_results.extend(results),
                Err(e) => tracing::warn!("Semantic Scholar search failed: {}", e),
            }
        }

        if config.sources.contains(&PaperSource::PubMed) {
            match self.pubmed.search(query, per_source_limit).await {
                Ok(results) => all_results.extend(results),
                Err(e) => tracing::warn!("PubMed search failed: {}", e),
            }
        }

        if let Some(since) = config.since_year {
            all_results.retain(|r| r.year.map_or(true, |y| y >= since));
        }

        let mut sorted = deduplicate(all_results);
        match config.sort_by {
            SortOrder::Citations => {
                sorted.sort_by(|a, b| b.citation_count.unwrap_or(0).cmp(&a.citation_count.unwrap_or(0)));
            }
            SortOrder::Date => {
                sorted.sort_by(|a, b| b.year.unwrap_or(0).cmp(&a.year.unwrap_or(0)));
            }
            SortOrder::Relevance => {}
        }

        sorted.truncate(config.limit);
        tracing::info!("Unified search: {} results after dedup for '{}'", sorted.len(), query);
        Ok(sorted)
    }

    pub fn arxiv(&self) -> &arxiv::ArxivSearch { &self.arxiv }
    pub fn semantic_scholar(&self) -> &semantic_scholar::SemanticScholarSearch { &self.s2 }
}

impl Default for UnifiedPaperSearch {
    fn default() -> Self { Self::new() }
}

fn deduplicate(papers: Vec<PaperResult>) -> Vec<PaperResult> {
    let mut unique: Vec<PaperResult> = Vec::new();

    for paper in papers {
        let normalized = normalize_title(&paper.title);
        let is_duplicate = unique.iter().any(|existing| {
            if let (Some(doi1), Some(doi2)) = (&paper.doi, &existing.doi) {
                if doi1 == doi2 { return true; }
            }
            title_similarity(&normalized, &normalize_title(&existing.title)) > 0.85
        });

        if !is_duplicate {
            unique.push(paper);
        } else {
            if let Some(existing) = unique.iter_mut().find(|e| {
                title_similarity(&normalize_title(&e.title), &normalized) > 0.85
            }) {
                if existing.citation_count.is_none() { existing.citation_count = paper.citation_count; }
                if existing.doi.is_none() { existing.doi = paper.doi; }
                if existing.pdf_url.is_none() { existing.pdf_url = paper.pdf_url; }
                if existing.abstract_text.is_none() { existing.abstract_text = paper.abstract_text; }
            }
        }
    }

    unique
}

fn normalize_title(title: &str) -> String {
    title.to_lowercase().chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace().collect::<Vec<_>>().join(" ")
}

fn title_similarity(a: &str, b: &str) -> f64 {
    let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if words_a.is_empty() && words_b.is_empty() { return 1.0; }
    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();
    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_title() {
        assert_eq!(normalize_title("Attention Is All You Need"), "attention is all you need");
    }

    #[test]
    fn test_title_similarity() {
        assert!(title_similarity("attention is all you need", "attention is all you need") > 0.99);
        assert!(title_similarity("attention is all you need", "completely different paper") < 0.3);
    }

    #[test]
    fn test_deduplicate() {
        let papers = vec![
            PaperResult {
                id: "1".into(), title: "Attention Is All You Need".into(),
                authors: vec![], abstract_text: None, year: Some(2017),
                citation_count: Some(100000), pdf_url: None, web_url: None,
                doi: Some("10.5555/3295222.3295349".into()),
                source: PaperSource::SemanticScholar, categories: vec![], published: None,
            },
            PaperResult {
                id: "2".into(), title: "Attention is All You Need".into(),
                authors: vec![], abstract_text: Some("We propose...".into()), year: Some(2017),
                citation_count: None, pdf_url: Some("https://arxiv.org/pdf/1706.03762".into()),
                web_url: None, doi: Some("10.5555/3295222.3295349".into()),
                source: PaperSource::ArXiv, categories: vec![], published: None,
            },
        ];
        let deduped = deduplicate(papers);
        assert_eq!(deduped.len(), 1);
        assert!(deduped[0].citation_count.is_some());
        assert!(deduped[0].pdf_url.is_some());
    }
}

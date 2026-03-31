// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Semantic Scholar paper search using the public API.

use super::{PaperResult, PaperSource};
use anyhow::Result;
use serde::Deserialize;

/// Semantic Scholar search client.
pub struct SemanticScholarSearch {
    client: reqwest::Client,
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct S2SearchResponse {
    #[allow(dead_code)]
    total: Option<u64>,
    data: Vec<S2Paper>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct S2Paper {
    paper_id: String,
    title: Option<String>,
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
    year: Option<u16>,
    citation_count: Option<u32>,
    authors: Option<Vec<S2Author>>,
    external_ids: Option<S2ExternalIds>,
    open_access_pdf: Option<S2Pdf>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct S2Author {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct S2ExternalIds {
    #[serde(rename = "DOI")]
    doi: Option<String>,
    #[serde(rename = "ArXiv")]
    #[allow(dead_code)]
    arxiv: Option<String>,
    #[serde(rename = "PubMed")]
    #[allow(dead_code)]
    pubmed: Option<String>,
}

#[derive(Debug, Deserialize)]
struct S2Pdf {
    url: Option<String>,
}

impl SemanticScholarSearch {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("NeuroGraph/0.1")
                .build()
                .expect("Failed to build HTTP client"),
            api_key: std::env::var("S2_API_KEY").ok(),
        }
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<PaperResult>> {
        let fields = "title,abstract,year,citationCount,authors,externalIds,openAccessPdf,url";
        let url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/search?query={}&limit={}&fields={}",
            urlencoding::encode(query), limit.min(100), fields
        );

        let mut request = self.client.get(&url);
        if let Some(ref key) = self.api_key {
            request = request.header("x-api-key", key);
        }

        let response = request.send().await
            .map_err(|e| anyhow::anyhow!("Failed to connect to Semantic Scholar API: {}", e))?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(anyhow::anyhow!("Semantic Scholar rate limit exceeded. Set S2_API_KEY for higher limits."));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Semantic Scholar API error ({}): {}", status, body));
        }

        let search_response: S2SearchResponse = response.json().await?;
        let results: Vec<PaperResult> = search_response.data.into_iter()
            .filter_map(convert_s2_paper).collect();

        tracing::info!("Semantic Scholar returned {} results for '{}'", results.len(), query);
        Ok(results)
    }

    pub async fn get_paper(&self, paper_id: &str) -> Result<PaperResult> {
        let fields = "title,abstract,year,citationCount,authors,externalIds,openAccessPdf,url";
        let url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/{}?fields={}",
            paper_id, fields
        );

        let mut request = self.client.get(&url);
        if let Some(ref key) = self.api_key {
            request = request.header("x-api-key", key);
        }

        let response = request.send().await?;
        let paper: S2Paper = response.json().await?;
        convert_s2_paper(paper).ok_or_else(|| anyhow::anyhow!("Failed to parse paper {}", paper_id))
    }

    pub async fn citations(&self, paper_id: &str, limit: usize) -> Result<Vec<PaperResult>> {
        let fields = "title,abstract,year,citationCount,authors,externalIds,url";
        let url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/{}/citations?fields={}&limit={}",
            paper_id, fields, limit
        );

        let mut request = self.client.get(&url);
        if let Some(ref key) = self.api_key {
            request = request.header("x-api-key", key);
        }

        let response = request.send().await?;
        let data: serde_json::Value = response.json().await?;

        let mut results = Vec::new();
        if let Some(entries) = data.get("data").and_then(|d| d.as_array()) {
            for entry in entries {
                if let Some(citing_paper) = entry.get("citingPaper") {
                    if let Ok(paper) = serde_json::from_value::<S2Paper>(citing_paper.clone()) {
                        if let Some(result) = convert_s2_paper(paper) {
                            results.push(result);
                        }
                    }
                }
            }
        }
        Ok(results)
    }
}

impl Default for SemanticScholarSearch {
    fn default() -> Self { Self::new() }
}

fn convert_s2_paper(paper: S2Paper) -> Option<PaperResult> {
    let title = paper.title.unwrap_or_default();
    if title.is_empty() { return None; }

    let authors = paper.authors.unwrap_or_default().into_iter()
        .filter_map(|a| a.name).collect();

    let external_ids = paper.external_ids.unwrap_or(S2ExternalIds {
        doi: None, arxiv: None, pubmed: None,
    });

    let pdf_url = paper.open_access_pdf.and_then(|p| p.url);

    Some(PaperResult {
        id: paper.paper_id,
        title, authors,
        abstract_text: paper.abstract_text,
        year: paper.year,
        citation_count: paper.citation_count,
        pdf_url,
        web_url: paper.url,
        doi: external_ids.doi,
        source: PaperSource::SemanticScholar,
        categories: Vec::new(),
        published: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_s2_paper() {
        let paper = S2Paper {
            paper_id: "abc123".to_string(),
            title: Some("Test Paper".to_string()),
            abstract_text: Some("A test.".to_string()),
            year: Some(2023),
            citation_count: Some(42),
            authors: Some(vec![S2Author { name: Some("John Doe".to_string()) }]),
            external_ids: Some(S2ExternalIds { doi: Some("10.1234/test".into()), arxiv: None, pubmed: None }),
            open_access_pdf: None,
            url: Some("https://example.com".to_string()),
        };

        let result = convert_s2_paper(paper).unwrap();
        assert_eq!(result.title, "Test Paper");
        assert_eq!(result.source, PaperSource::SemanticScholar);
    }

    #[test]
    fn test_skip_empty_title() {
        let paper = S2Paper {
            paper_id: "abc".into(), title: None, abstract_text: None, year: None,
            citation_count: None, authors: None, external_ids: None,
            open_access_pdf: None, url: None,
        };
        assert!(convert_s2_paper(paper).is_none());
    }
}

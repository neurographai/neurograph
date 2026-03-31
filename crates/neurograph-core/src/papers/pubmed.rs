// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! PubMed paper search using E-utilities API.

use super::{PaperResult, PaperSource};
use anyhow::Result;

/// PubMed search client.
pub struct PubMedSearch {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl PubMedSearch {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("NeuroGraph/0.1")
                .build()
                .expect("Failed to build HTTP client"),
            api_key: std::env::var("NCBI_API_KEY").ok(),
        }
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<PaperResult>> {
        let ids = self.esearch(query, limit).await?;
        if ids.is_empty() { return Ok(Vec::new()); }
        let results = self.efetch(&ids).await?;
        tracing::info!("PubMed returned {} results for '{}'", results.len(), query);
        Ok(results)
    }

    async fn esearch(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let mut url = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?\
             db=pubmed&term={}&retmax={}&retmode=json&sort=relevance",
            urlencoding::encode(query), limit.min(100)
        );
        if let Some(ref key) = self.api_key {
            url.push_str(&format!("&api_key={}", key));
        }

        let response = self.client.get(&url).send().await?;
        let json: serde_json::Value = response.json().await?;

        Ok(json.get("esearchresult")
            .and_then(|r| r.get("idlist"))
            .and_then(|l| l.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default())
    }

    async fn efetch(&self, ids: &[String]) -> Result<Vec<PaperResult>> {
        if ids.is_empty() { return Ok(Vec::new()); }
        let id_str = ids.join(",");
        let mut url = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?\
             db=pubmed&id={}&retmode=json",
            id_str
        );
        if let Some(ref key) = self.api_key {
            url.push_str(&format!("&api_key={}", key));
        }

        let response = self.client.get(&url).send().await?;
        let json: serde_json::Value = response.json().await?;

        let mut results = Vec::new();
        if let Some(result_obj) = json.get("result") {
            for id in ids {
                if let Some(paper_json) = result_obj.get(id) {
                    if let Some(paper) = parse_pubmed_summary(paper_json, id) {
                        results.push(paper);
                    }
                }
            }
        }
        Ok(results)
    }
}

impl Default for PubMedSearch {
    fn default() -> Self { Self::new() }
}

fn parse_pubmed_summary(json: &serde_json::Value, pmid: &str) -> Option<PaperResult> {
    let title = json.get("title")?.as_str()?.to_string();
    let authors: Vec<String> = json.get("authors")
        .and_then(|a| a.as_array())
        .map(|arr| arr.iter()
            .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
            .map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let year = json.get("pubdate").and_then(|d| d.as_str())
        .and_then(|s| s.split_whitespace().next())
        .and_then(|y| y.parse::<u16>().ok());

    let doi = json.get("elocationid").and_then(|e| e.as_str())
        .and_then(|s| s.strip_prefix("doi: "))
        .map(|s| s.to_string());

    Some(PaperResult {
        id: format!("pmid:{}", pmid),
        title, authors,
        abstract_text: None,
        year, citation_count: None, pdf_url: None,
        web_url: Some(format!("https://pubmed.ncbi.nlm.nih.gov/{}/", pmid)),
        doi,
        source: PaperSource::PubMed,
        categories: Vec::new(),
        published: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pubmed_summary() {
        let json = serde_json::json!({
            "title": "A Novel Approach to Something",
            "authors": [{"name": "Smith J"}, {"name": "Doe A"}],
            "pubdate": "2023 Jan",
            "elocationid": "doi: 10.1234/test.2023"
        });

        let result = parse_pubmed_summary(&json, "12345678").unwrap();
        assert_eq!(result.title, "A Novel Approach to Something");
        assert_eq!(result.year, Some(2023));
        assert_eq!(result.source, PaperSource::PubMed);
    }
}

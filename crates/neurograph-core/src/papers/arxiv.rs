// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! arXiv paper search using the Atom XML API.
//! Free, no API key required. Rate limit: ~3 requests/second.

use super::{PaperResult, PaperSource};
use anyhow::Result;
use chrono::NaiveDateTime;

/// arXiv search client.
pub struct ArxivSearch {
    client: reqwest::Client,
}

impl ArxivSearch {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("NeuroGraph/0.1 (https://github.com/neurographai/neurograph)")
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    /// Search arXiv for papers matching the query.
    pub async fn search(
        &self, query: &str, limit: usize, category: Option<&str>,
    ) -> Result<Vec<PaperResult>> {
        let search_query = if let Some(cat) = category {
            format!("all:{} AND cat:{}", urlencoding::encode(query), cat)
        } else {
            format!("all:{}", urlencoding::encode(query))
        };

        let url = format!(
            "http://export.arxiv.org/api/query?search_query={}&start=0&max_results={}&sortBy=relevance&sortOrder=descending",
            search_query, limit.min(100)
        );

        let response = self.client.get(&url).send().await
            .map_err(|e| anyhow::anyhow!("Failed to connect to arXiv API: {}", e))?;

        let xml_text = response.text().await?;
        let results = parse_arxiv_response(&xml_text)?;

        tracing::info!("arXiv returned {} results for '{}'", results.len(), query);
        Ok(results)
    }

    /// Download a PDF from arXiv by its ID.
    pub async fn download_pdf(
        &self, arxiv_id: &str, output_dir: &std::path::Path,
    ) -> Result<std::path::PathBuf> {
        let clean_id = arxiv_id
            .trim_start_matches("http://arxiv.org/abs/")
            .trim_start_matches("https://arxiv.org/abs/");

        let pdf_url = format!("https://arxiv.org/pdf/{}.pdf", clean_id);
        let output_path = output_dir.join(format!("{}.pdf", clean_id.replace('/', "_")));

        let response = self.client.get(&pdf_url).send().await
            .map_err(|e| anyhow::anyhow!("Failed to download PDF from arXiv: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("arXiv returned {} for {}", response.status(), pdf_url));
        }

        let bytes = response.bytes().await?;
        std::fs::create_dir_all(output_dir)?;
        std::fs::write(&output_path, &bytes)?;

        tracing::info!("Downloaded {} bytes to {}", bytes.len(), output_path.display());
        Ok(output_path)
    }
}

impl Default for ArxivSearch {
    fn default() -> Self { Self::new() }
}

fn parse_arxiv_response(xml: &str) -> Result<Vec<PaperResult>> {
    let mut results = Vec::new();
    let entries: Vec<&str> = xml.split("<entry>").skip(1).collect();

    for entry_xml in entries {
        let end = entry_xml.find("</entry>").unwrap_or(entry_xml.len());
        let entry_str = &entry_xml[..end];
        if let Some(paper) = parse_entry_xml(entry_str) {
            results.push(paper);
        }
    }

    Ok(results)
}

fn parse_entry_xml(xml: &str) -> Option<PaperResult> {
    let id = extract_xml_value(xml, "id")?;
    let title = extract_xml_value(xml, "title")?
        .replace('\n', " ").trim().to_string();
    let summary = extract_xml_value(xml, "summary")
        .map(|s| s.replace('\n', " ").trim().to_string());
    let published_str = extract_xml_value(xml, "published");

    let mut authors = Vec::new();
    for name_block in xml.split("<author>").skip(1) {
        if let Some(name) = extract_xml_value(name_block, "name") {
            authors.push(name.trim().to_string());
        }
    }

    let pdf_url = Some(id.replace("/abs/", "/pdf/") + ".pdf");

    let mut categories = Vec::new();
    let mut search_start = 0;
    while let Some(pos) = xml[search_start..].find("term=\"") {
        let abs_pos = search_start + pos + 6;
        if let Some(end) = xml[abs_pos..].find('"') {
            categories.push(xml[abs_pos..abs_pos + end].to_string());
        }
        search_start = abs_pos + 1;
    }

    let published = published_str.as_ref().and_then(|s| {
        NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%dT%H:%M:%SZ")
            .ok().map(|dt| dt.and_utc())
    });

    let year = published.map(|d| d.format("%Y").to_string().parse::<u16>().unwrap_or(0));

    let arxiv_id = id
        .trim_start_matches("http://arxiv.org/abs/")
        .trim_start_matches("https://arxiv.org/abs/")
        .to_string();

    Some(PaperResult {
        id: arxiv_id,
        title,
        authors,
        abstract_text: summary,
        year,
        citation_count: None,
        pdf_url,
        web_url: Some(id),
        doi: None,
        source: PaperSource::ArXiv,
        categories,
        published,
    })
}

fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let start_pos = xml.find(&open)?;
    let content_start = xml[start_pos..].find('>')? + start_pos + 1;
    let end_pos = xml[content_start..].find(&close)? + content_start;
    Some(xml[content_start..end_pos].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_entry_xml() {
        let xml = r#"
            <id>http://arxiv.org/abs/1706.03762v5</id>
            <title>Attention Is All You Need</title>
            <summary>We propose a new architecture...</summary>
            <published>2017-06-12T00:00:00Z</published>
            <author><name>Ashish Vaswani</name></author>
            <author><name>Noam Shazeer</name></author>
            <category term="cs.CL"/>
        "#;

        let result = parse_entry_xml(xml).unwrap();
        assert_eq!(result.title, "Attention Is All You Need");
        assert_eq!(result.authors.len(), 2);
        assert_eq!(result.source, PaperSource::ArXiv);
    }

    #[test]
    fn test_extract_xml_value() {
        let xml = "<title>Hello World</title>";
        assert_eq!(extract_xml_value(xml, "title"), Some("Hello World".into()));
    }
}

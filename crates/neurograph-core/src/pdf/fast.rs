// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Tier 1: Fast PDF text extraction using pdf-extract.
//!
//! Performance: ~1ms/page for text-based PDFs.
//! Does NOT detect structure (sections, headings). Use `classifier` + `academic` for that.

use std::path::Path;

use super::types::PageContent;

/// Extract all text from a PDF in one call.
pub fn extract_text(path: &Path) -> anyhow::Result<String> {
    pdf_extract::extract_text(path)
        .map_err(|e| anyhow::anyhow!("PDF text extraction failed for '{}': {}", path.display(), e))
}

/// Extract text from PDF bytes in memory.
pub fn extract_text_from_mem(bytes: &[u8]) -> anyhow::Result<String> {
    pdf_extract::extract_text_from_mem(bytes)
        .map_err(|e| anyhow::anyhow!("PDF text extraction failed: {}", e))
}

/// Extract text page by page.
///
/// Uses `lopdf` to get page count, then `pdf-extract` for full text,
/// then splits by form-feed characters (\x0C).
/// Falls back to estimated splitting if no page markers are found.
pub fn extract_pages(path: &Path) -> anyhow::Result<Vec<PageContent>> {
    let full_text = extract_text(path)?;

    if full_text.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "No text extracted from '{}'. The PDF may be scanned/image-only.",
            path.display()
        ));
    }

    let page_count = get_page_count(path).unwrap_or(1);

    // Split by form-feed character (common PDF page separator)
    let raw_pages: Vec<&str> = full_text.split('\x0C').collect();

    let pages = if raw_pages.len() > 1 {
        raw_pages
            .iter()
            .enumerate()
            .filter(|(_, text)| !text.trim().is_empty())
            .map(|(i, text)| PageContent {
                page_num: (i + 1) as u32,
                text: text.trim().to_string(),
                word_count: text.split_whitespace().count(),
            })
            .collect()
    } else {
        // No form-feeds — estimate pages by character count
        let chars_per_page = if page_count > 0 {
            full_text.len() / page_count as usize
        } else {
            3000
        };
        split_into_pages(&full_text, chars_per_page)
    };

    Ok(pages)
}

/// Get accurate page count using lopdf (fast metadata read).
pub fn get_page_count(path: &Path) -> anyhow::Result<u32> {
    let doc = lopdf::Document::load(path)
        .map_err(|e| anyhow::anyhow!("Failed to open PDF '{}': {}", path.display(), e))?;
    Ok(doc.get_pages().len() as u32)
}

/// Get page count from bytes.
pub fn get_page_count_from_mem(bytes: &[u8]) -> anyhow::Result<u32> {
    let doc = lopdf::Document::load_mem(bytes)
        .map_err(|e| anyhow::anyhow!("Failed to parse PDF from memory: {}", e))?;
    Ok(doc.get_pages().len() as u32)
}

/// Split text into estimated pages by character count.
fn split_into_pages(text: &str, chars_per_page: usize) -> Vec<PageContent> {
    let chars_per_page = chars_per_page.max(500);
    let mut pages = Vec::new();
    let mut offset = 0;
    let mut page_num = 1u32;

    while offset < text.len() {
        let end = (offset + chars_per_page).min(text.len());

        // Try to break at a paragraph boundary
        let actual_end = if end < text.len() {
            text[offset..end]
                .rfind("\n\n")
                .map(|pos| offset + pos + 2)
                .unwrap_or(end)
        } else {
            end
        };

        let page_text = text[offset..actual_end].trim().to_string();
        if !page_text.is_empty() {
            pages.push(PageContent {
                page_num,
                text: page_text.clone(),
                word_count: page_text.split_whitespace().count(),
            });
        }

        offset = actual_end;
        page_num += 1;
    }

    pages
}

/// Alias for `extract_text_from_mem` — used by the existing mod.rs API.
pub fn extract_text_from_bytes(bytes: &[u8]) -> anyhow::Result<String> {
    extract_text_from_mem(bytes)
}

/// Alias for `get_page_count` — used by the existing mod.rs API.
pub fn estimate_page_count(path: &Path) -> anyhow::Result<u32> {
    get_page_count(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_into_pages() {
        let text = "Page one content.\n\nPage two content.\n\nPage three content.";
        let pages = split_into_pages(text, 25);
        assert!(!pages.is_empty());
        assert_eq!(pages[0].page_num, 1);
    }

    #[test]
    fn test_split_empty_text() {
        let pages = split_into_pages("", 3000);
        assert!(pages.is_empty());
    }
}

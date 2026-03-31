// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Serve embedded dashboard static files.

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    response::{IntoResponse, Response},
};

/// Embedded dashboard assets (compiled React build).
#[derive(rust_embed::Embed)]
#[folder = "../../dashboard/dist/"]
#[allow(dead_code)]
struct DashboardAssets;

/// Serve a static file from embedded assets.
pub async fn serve_static(req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match DashboardAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(content.data.to_vec()))
                .unwrap()
        }
        None => {
            // SPA fallback: serve index.html for any unknown path
            match DashboardAssets::get("index.html") {
                Some(content) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html")
                    .body(Body::from(content.data.to_vec()))
                    .unwrap(),
                None => {
                    let html = r#"<!DOCTYPE html>
<html>
<head><title>NeuroGraph</title></head>
<body style="background:#0d1117;color:#c9d1d9;font-family:sans-serif;padding:40px">
<h1>NeuroGraph Dashboard</h1>
<p>Dashboard not built. Run <code>cd dashboard && npm run build</code> first.</p>
<p>API is available at <a href="/api/v1/health" style="color:#58a6ff">/api/v1/health</a></p>
</body>
</html>"#;
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "text/html")
                        .body(Body::from(html))
                        .unwrap()
                }
            }
        }
    }
}

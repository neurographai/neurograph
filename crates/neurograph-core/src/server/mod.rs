// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Axum-based REST API server for NeuroGraph.

#[cfg(feature = "server")]
pub mod chat_routes;
#[cfg(feature = "server")]
pub mod embed;
#[cfg(feature = "server")]
pub mod routes;
#[cfg(feature = "server")]
pub mod settings;
#[cfg(feature = "server")]
pub mod state;
#[cfg(feature = "server")]
pub mod static_files;

#[cfg(feature = "server")]
pub use routes::create_router;
#[cfg(feature = "server")]
pub use state::AppState;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub data_dir: Option<std::path::PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8000,
            cors_origins: vec!["http://localhost:3000".to_string()],
            data_dir: None,
        }
    }
}

/// Start the REST API server.
#[cfg(feature = "server")]
pub async fn start_server(config: ServerConfig) -> anyhow::Result<()> {
    let state = state::AppState::new(config.data_dir.as_deref()).await?;
    let app = create_router(state, &config.cors_origins);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("NeuroGraph API server listening on http://{}", addr);
    tracing::info!("Dashboard: http://{}/", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

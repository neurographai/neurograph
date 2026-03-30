# Developer Guide

Welcome to the internal documentation for developing NeuroGraph! 🛠️

This guide covers everything you need to set up a local development environment, understand the codebase, run tests, debug common issues, and contribute production-quality code.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Architecture Overview](#architecture-overview)
- [Repository Structure](#repository-structure)
- [Setting Up Your Dev Environment](#setting-up-your-dev-environment)
  - [Rust Toolchain](#rust-toolchain)
  - [Node.js & Dashboard](#nodejs--dashboard)
  - [Docker (Optional)](#docker-optional)
  - [Environment Variables](#environment-variables)
- [Building](#building)
  - [Backend (Rust)](#backend-rust)
  - [Frontend (Dashboard)](#frontend-dashboard)
  - [WASM Module](#wasm-module)
- [Running the Application Locally](#running-the-application-locally)
- [Testing](#testing)
  - [Unit Tests](#unit-tests)
  - [Integration Tests](#integration-tests)
  - [Dashboard Tests](#dashboard-tests)
  - [Test Coverage](#test-coverage)
- [Code Quality](#code-quality)
  - [Formatting](#formatting)
  - [Linting](#linting)
  - [Security Auditing](#security-auditing)
- [Debugging](#debugging)
  - [Logging](#logging)
  - [Tracing](#tracing)
  - [Common Issues](#common-issues)
- [Performance Profiling](#performance-profiling)
- [Working with the Graph Driver](#working-with-the-graph-driver)
- [Adding a New Feature](#adding-a-new-feature)
- [Creating a Release](#creating-a-release)
- [IDE Setup](#ide-setup)

---

## Prerequisites

| Tool | Version | Required For |
|------|---------|-------------|
| **Rust** | 1.82+ (stable) | Core engine, CLI, WASM |
| **Node.js** | 18+ | Dashboard |
| **npm** | 9+ | Dashboard dependencies |
| **wasm-pack** | Latest | WASM graph layouts (optional) |
| **Docker** | Latest | Integration tests, deployment |
| **Docker Compose** | v2+ | Full-stack local deployment |

Install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
rustup target add wasm32-unknown-unknown  # For WASM builds
```

Install wasm-pack:
```bash
cargo install wasm-pack
```

---

## Architecture Overview

The repository is structured as a monolithic Rust workspace with a TypeScript dashboard:

```
neurograph/
├── crates/
│   └── neurograph-core/       # The main Rust engine
│       └── src/
│           ├── lib.rs          # Public API (NeuroGraph struct)
│           ├── config.rs       # Configuration & builder
│           ├── drivers/        # Storage backends (memory, sled, neo4j)
│           ├── embedders/      # Embedding providers (hash, openai)
│           ├── engine/         # Query routing & context assembly
│           ├── graph/          # Data model (Entity, Relationship, Community)
│           ├── ingestion/      # Extraction & dedup pipeline
│           ├── llm/            # LLM client abstraction
│           ├── retrieval/      # Hybrid search (semantic + BM25 + graph)
│           ├── temporal/       # Bi-temporal engine & forgetting
│           ├── community/      # Louvain & Leiden community detection
│           └── utils/          # Concurrency, cost tracking, helpers
├── dashboard/                 # React + TypeScript + Vite (AntV G6)
├── benchmarks/                # Performance benchmark suite
├── examples/                  # Usage examples
├── docker/                    # Docker build files
├── deploy/                    # Docker Compose & deployment configs
└── docs/                      # Architecture & deep-dive documentation
```

For a deep-dive into the architecture, see [docs/architecture.md](docs/architecture.md).

---

## Repository Structure

### Key Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace manifest — defines members, shared dependencies |
| `Cargo.lock` | Locked dependency versions |
| `.clippy.toml` | Clippy lint configuration |
| `.rustfmt.toml` | Rustfmt formatting rules |
| `.env.example` | Example environment variables |
| `docker-compose.yml` | Full-stack Docker configuration |
| `cliff.toml` | git-cliff changelog generation config |
| `deny.toml` | cargo-deny license & vulnerability rules |

### Crate Modules

| Module | Lines | Purpose |
|--------|-------|---------|
| `lib.rs` | ~770 | Public API surface + NeuroGraph struct |
| `config.rs` | ~300 | Config builder, storage/LLM/embedding provider setup |
| `drivers/` | ~1500 | Storage backends (memory, embedded, neo4j) |
| `graph/` | ~1200 | Core data model (Entity, Relationship, Episode, Community) |
| `ingestion/` | ~1400 | Extraction, dedup, conflict resolution |
| `retrieval/` | ~600 | Hybrid search with RRF fusion |
| `temporal/` | ~800 | Bi-temporal engine, forgetting |
| `community/` | ~1400 | Louvain, Leiden, summarizer, incremental |
| `llm/` | ~400 | LLM client trait + OpenAI impl |
| `embedders/` | ~300 | Embedder trait + hash/OpenAI impls |

---

## Setting Up Your Dev Environment

### Rust Toolchain

```bash
# Install Rust (if not already)
rustup default stable

# Verify version (must be 1.82+)
rustc --version

# Install additional components
rustup component add clippy rustfmt

# Install development tools
cargo install cargo-watch    # Auto-rebuild on file changes
cargo install cargo-nextest  # Faster test runner
cargo install cargo-audit    # Security vulnerability scanning
cargo install cargo-deny     # License & vulnerability policy enforcement
cargo install cargo-tarpaulin # Code coverage (Linux only)
```

### Node.js & Dashboard

```bash
cd dashboard
npm install
```

### Docker (Optional)

Docker is only needed for:
- Integration tests with Neo4j/FalkorDB
- Building production containers
- Running the full-stack deployment

### Environment Variables

Copy the example and configure:

```bash
cp .env.example .env
```

Key variables:

| Variable | Default | Purpose |
|----------|---------|---------|
| `OPENAI_API_KEY` | (none) | Enables LLM extraction + OpenAI embeddings |
| `NEUROGRAPH_STORAGE` | `memory` | Storage backend (`memory`, `embedded`, `neo4j`) |
| `NEUROGRAPH_EMBED_PATH` | `./data/graph.db` | Path for embedded (sled) storage |
| `NEUROGRAPH_LOG` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |
| `NEUROGRAPH_BUDGET_USD` | `10.0` | Maximum LLM spend per session |
| `NEO4J_URI` | `bolt://localhost:7687` | Neo4j connection string |
| `NEO4J_USER` | `neo4j` | Neo4j username |
| `NEO4J_PASS` | (none) | Neo4j password |

**Zero-config mode:** If no `OPENAI_API_KEY` is set, NeuroGraph automatically uses:
- Regex-based entity extraction (no LLM)
- Hash-based embeddings (no API calls)
- In-memory storage

---

## Building

### Backend (Rust)

```bash
# Debug build (faster compilation, slower runtime)
cargo build --workspace

# Release build (slower compilation, optimized runtime)
cargo build --workspace --release

# Build only the core crate
cargo build -p neurograph-core
```

### Frontend (Dashboard)

```bash
cd dashboard
npm install
npm run build
```

### WASM Module

```bash
cd crates/neurograph-wasm  # Not yet available — Sprint 4
wasm-pack build --target web --release
```

---

## Running the Application Locally

### Backend only (Cargo)

```bash
# Run the server (Sprint 4)
cd crates/neurograph-core
cargo run --release -- --serve

# Run with auto-reload
cargo watch -x 'run --release -- --serve'
```

### Frontend only (Vite)

```bash
cd dashboard
npm install
npm run dev
# → http://localhost:5173
```

### Full stack (Docker Compose)

```bash
docker compose up
# API:       http://localhost:8000
# Dashboard: http://localhost:3000
# Neo4j:     http://localhost:7474 (browser)
```

---

## Testing

### Unit Tests

```bash
# Run all unit tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p neurograph-core

# Run a specific test
cargo test -p neurograph-core test_louvain_two_cliques

# Run tests with output
cargo test --workspace -- --nocapture

# Run tests with nextest (faster, parallel)
cargo nextest run --workspace
```

### Integration Tests

Integration tests require Docker for database backends:

```bash
# Start dependencies
docker compose -f docker-compose.test.yml up -d

# Run integration tests
cargo test --test integration_tests

# Shut down
docker compose -f docker-compose.test.yml down
```

### Dashboard Tests

```bash
cd dashboard
npm run lint        # ESLint
npm run build       # TypeScript type checking
```

### Test Coverage

```bash
# Linux only (tarpaulin)
cargo tarpaulin --workspace --out html
open tarpaulin-report.html

# All platforms (nextest + llvm-cov)
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --html
```

---

## Code Quality

### Formatting

```bash
# Check formatting
cargo fmt --all -- --check

# Auto-format
cargo fmt --all

# Dashboard
cd dashboard && npx prettier --check src/
cd dashboard && npx prettier --write src/
```

### Linting

```bash
# Clippy (Rust)
cargo clippy --workspace --all-targets -- -D warnings

# ESLint (Dashboard)
cd dashboard && npm run lint
```

### Security Auditing

```bash
# Check for known vulnerabilities (Rust dependencies)
cargo audit

# Check license compliance + vulnerabilities
cargo deny check

# Generate SBOM
cargo install cargo-cyclonedx
cargo cyclonedx --all
```

---

## Debugging

### Logging

NeuroGraph uses `tracing` for structured logging. Control the level with `RUST_LOG`:

```bash
# Verbose logging
RUST_LOG=debug cargo run

# Module-specific logging
RUST_LOG=neurograph_core::ingestion=trace,neurograph_core::retrieval=debug cargo run

# JSON-formatted logs (for log aggregation)
RUST_LOG=info NEUROGRAPH_LOG_FORMAT=json cargo run
```

### Tracing

For distributed tracing (OpenTelemetry):

```bash
# Start Jaeger (or any OTEL collector)
docker run -d -p 16686:16686 -p 4317:4317 jaegertracing/all-in-one

# Run with tracing enabled
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 cargo run

# View traces at http://localhost:16686
```

### Common Issues

| Issue | Cause | Fix |
|-------|-------|-----|
| `OPENAI_API_KEY not set` | Expected behavior | NeuroGraph works without it (regex extraction) |
| `sled lock file` | Previous process didn't clean up | Delete `./data/graph.db/lock` |
| `WASM compilation fails` | Missing target | `rustup target add wasm32-unknown-unknown` |
| `npm install fails` | Node version too old | Upgrade to Node.js 18+ |
| `cargo test hangs` | Async runtime issue | Check for missing `#[tokio::test]` |
| `too many open files` | sled + many tests | `ulimit -n 4096` on macOS/Linux |

---

## Performance Profiling

### Benchmarks

```bash
cd benchmarks
cargo bench

# Specific benchmark
cargo bench -- louvain
cargo bench -- hybrid_search
```

### Flame graphs

```bash
# Install
cargo install flamegraph

# Generate
cargo flamegraph --bin neurograph -- --serve

# View
open flamegraph.svg
```

### Memory profiling

```bash
# macOS
cargo install cargo-instruments
cargo instruments -t Allocations

# Linux
valgrind --tool=massif ./target/release/neurograph
```

---

## Working with the Graph Driver

All storage operations go through the `GraphDriver` trait. To add a new backend:

1. Create a new file in `crates/neurograph-core/src/drivers/`
2. Implement `GraphDriver`:

```rust
#[async_trait]
impl GraphDriver for MyDriver {
    fn name(&self) -> &str { "my-driver" }

    async fn store_entity(&self, entity: &Entity) -> Result<(), DriverError> {
        // Store entity
    }

    async fn get_entity(&self, id: &EntityId) -> Result<Entity, DriverError> {
        // Retrieve entity
    }

    // ... implement all required methods
}
```

3. Add it to the `StorageBackend` enum in `config.rs`
4. Wire it up in `NeuroGraphBuilder::build()`
5. Add tests using the same patterns as `drivers::memory::tests`

---

## Adding a New Feature

1. **Branch:** Create a feature branch from `main`
2. **Design:** If it touches the public API, update `lib.rs` first
3. **Implement:** Write the feature with tests
4. **Lint:** `cargo clippy --workspace -- -D warnings`
5. **Format:** `cargo fmt --all`
6. **Test:** `cargo test --workspace`
7. **Document:** Add/update doc comments and `docs/*.md` if applicable
8. **PR:** Fill out the PR template with context and rationale

### Where to Put Things

| Type of Code | Location |
|-------------|----------|
| New storage backend | `crates/neurograph-core/src/drivers/` |
| New embedder | `crates/neurograph-core/src/embedders/` |
| New LLM provider | `crates/neurograph-core/src/llm/` |
| New retrieval strategy | `crates/neurograph-core/src/retrieval/` |
| New community algorithm | `crates/neurograph-core/src/community/` |
| New temporal operation | `crates/neurograph-core/src/temporal/` |
| Public API additions | `crates/neurograph-core/src/lib.rs` |
| Dashboard component | `dashboard/src/components/` |
| CLI command | `crates/neurograph-cli/src/` (Sprint 4) |
| Python binding | `crates/neurograph-python/src/` (Sprint 4) |

---

## Creating a Release

Releases are automated via GitHub Actions when tags are pushed:

```bash
# Ensure everything passes
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check

# Create and push tag
git tag v0.1.0
git push origin v0.1.0
```

The CI pipeline will:
1. Run full test suite
2. Build release binaries (Linux, macOS, Windows)
3. Build and push Docker images to `ghcr.io`
4. Publish to `crates.io`
5. Generate changelog via `git-cliff`
6. Create GitHub Release with artifacts

### Manual changelog generation

```bash
git cliff --tag v0.1.0 -o CHANGELOG.md
```

---

## IDE Setup

### VS Code (Recommended)

Install extensions:
- **rust-analyzer** — Rust language server
- **crates** — Dependency version management
- **Even Better TOML** — Cargo.toml editing
- **ESLint** — Dashboard linting
- **Prettier** — Dashboard formatting

Recommended `settings.json`:
```json
{
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.check.extraArgs": ["--workspace", "--all-targets"],
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

### JetBrains (RustRover / CLion)

Install the **Rust** plugin. Set clippy as the default check command in Settings → Languages → Rust → External Linters.

### Neovim

Use `rust-tools.nvim` or `rustaceanvim` for LSP integration with rust-analyzer.

---

*For the overall architecture, see [docs/architecture.md](docs/architecture.md). For contributing guidelines, see [CONTRIBUTING.md](CONTRIBUTING.md).*

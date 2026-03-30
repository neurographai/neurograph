# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

#### Core Engine
- **Bi-temporal knowledge graph engine** — Every fact has a `valid_from` / `valid_until` window with full history tracking. Supports point-in-time snapshots, temporal diffing, and timeline construction.
- **Hybrid retrieval pipeline** — Combines semantic search (cosine similarity), BM25 keyword search, and graph traversal using Reciprocal Rank Fusion (RRF) for high-recall, high-precision results.
- **Louvain community detection** — Pure Rust implementation of the Louvain algorithm with configurable resolution, hierarchical multi-level detection, and modularity scoring.
- **Leiden community detection** — Enhanced Louvain with refinement phase guaranteeing connected communities and higher-quality partitions.
- **Incremental community updates** — When new entities are ingested, only the k-hop neighborhood is recomputed instead of the full graph. Saves 70-90% compute.
- **Community summarization** — LLM-powered map-reduce summarization with diff-based re-summarization (~30% token cost saving). Falls back to rule-based summaries when no API key is set.
- **Intelligent forgetting engine** — Biologically-inspired memory decay using composite importance scoring: `time_decay × base_importance + access_boost + connectivity_boost`. Configurable TTL, daily decay rate, and prune thresholds.
- **Fact version chains** — Full chronological history of how facts evolve. When "Alice works at Google" is superseded by "Alice works at Anthropic", both versions are linked and queryable by timestamp.
- **Entity history tracking** — Tracks all modifications to entities (created, summary updated, merged, type changed, decay marked) with before/after snapshots.
- **Hybrid logical clock (HLC)** — Combines wall-clock time with a logical counter for strict event ordering in concurrent ingestion scenarios.
- **Cost-aware query routing** — `QueryRouter` classifies queries (local/global/temporal), estimates dollar cost per strategy, and selects the cheapest option that meets quality requirements.
- **Context assembly** — Token-budget-aware prompt building that selects the most relevant entities, relationships, and community summaries without exceeding the LLM context window.
- **Graph schema registry** — Tracks entity types and relationship types as they're ingested. Supports ontology constraints and type hierarchies.
- **Budget enforcement** — `CostTracker` maintains cumulative LLM spend and raises `BudgetExceeded` when the configured limit is reached.
- **Concurrency limiter** — Semaphore-bounded concurrent LLM calls (default: 4) to prevent rate limit errors.

#### Ingestion Pipeline
- **2-phase entity deduplication** — Phase 1: fast embedding cosine similarity + hash matching (>0.95 → auto-merge). Phase 2: LLM fallback for ambiguous cases (0.8-0.95 similarity).
- **Temporal conflict resolution** — When a new fact contradicts an existing one, the old fact's `valid_until` is set and linked via `superseded_by` to the new fact. No data is ever deleted.
- **Regex NER fallback** — Pattern-based named entity recognition that works without any API key. Detects persons, organizations, locations, dates, and simple relationships.
- **LLM structured extraction** — Uses OpenAI/Anthropic/Ollama for high-quality entity and relationship extraction with typed JSON output.
- **Post-extraction validators** — Check entity type consistency, relationship cardinality, and embedding dimensions after extraction.

#### Storage Backends
- **MemoryDriver** — In-memory graph storage using `petgraph` + `DashMap` for lock-free concurrent access. Ideal for testing and ephemeral sessions.
- **EmbeddedDriver (sled)** — Persistent embedded storage using sled. Zero-config, no external dependencies. Production-ready for single-node deployments.
- **GraphDriver trait** — Async trait abstracting all storage operations. Supports `store_entity`, `get_entity`, `search_by_vector`, `search_by_text`, `snapshot_at`, `traverse`, `store_community`, and more.

#### Embedding
- **HashEmbedder** — Deterministic hash-based embedder producing 128-dimensional vectors. Zero cost, zero latency, works offline.
- **OpenAiEmbedder** — OpenAI text-embedding-3-small/large integration for production-quality semantic search.
- **Embedder trait** — Async trait supporting `embed_one`, `embed_batch`, with model introspection (`model_name`, `dimensions`).

#### LLM Integration
- **OpenAI client** — Supports GPT-4o, GPT-4o-mini with structured JSON extraction, cost tracking per call, and configurable model selection.
- **LlmClient trait** — Async trait for pluggable LLM providers (OpenAI, Anthropic, Ollama, any OpenAI-compatible endpoint).

#### Dashboard
- **React 19 + TypeScript + Vite** — Modern frontend architecture with hot module replacement.
- **AntV G6 graph visualization** — WebGL/Canvas rendering with force-directed layouts, zoom, pan, node selection.
- **Native Rust WASM graph layouts** — Force-directed layout computation compiled from Rust to WASM for 5-10x speedup over JavaScript.
- **Dark/light mode** — Premium dark theme by default with animated sun/moon toggle. Persists via localStorage.
- **Temporal playback** — Timeline slider with density heatmap to scrub through knowledge history.
- **Community cluster visualization** — Color-coded G6 Combos for community grouping.
- **3-column layout rewrite** — Query panel (left), graph canvas + timeline (center), node detail inspector (right).
- **Zustand state management** — Global store for graph data, timeline, branches, query results, theme, and reasoning animation.
- **Query panel** — Natural-language query input with reasoning path visualization and cost tracking.
- **Branch diff viewer** — Side-by-side branch comparison with added/removed/modified node indicators.
- **Node detail panel** — Right sidebar inspector with metadata, connections, importance bar, and tier info.
- **Graph view switcher** — Filter edges by type: All / Semantic / Temporal / Causal / Entity with live counts.
- **Custom logo** — NeuroGraph hexagonal network logo in header.
- **Dashboard Dockerfile** — Nginx-based production container with SPA routing and API reverse proxy.

#### Intent Router & Multi-Strategy Graph (v0.2)
- **IntentRouter** — Classifies queries by intent type (semantic, temporal, causal, entity) with keyword-based TF scoring and confidence values.
- **Multi-hop planning** — `plan()` method decomposes complex queries into sequential `HopStep`s across sub-graph layers.
- **Sub-graph dispatch** — `route()` executes classified queries against specialized sub-graphs (entity, semantic, temporal, causal).
- **MultiStrategyGraph** — Orchestrates four specialized sub-graphs with cross-layer fusion for adaptive retrieval.
- **Entity sub-graph** — Named entities with relationship-strength edges.
- **Semantic sub-graph** — Embedding-similarity weighted edges for conceptual queries.
- **Temporal sub-graph** — Time-ordered edges with validity windows for before/after reasoning.
- **Causal sub-graph** — Cause-effect relationships with causal confidence scoring.
- **AdaptiveFusion** — Cross-layer result fusion with dynamically adjusted weights.

#### Tiered Memory System (v0.2)
- **4-tier memory architecture** — Working → Episodic → Semantic → Procedural, mirroring human cognitive layers.
- **Episode grouping** — Groups related facts into coherent episodes with temporal bounds and topic labels.
- **Learned rules (procedural)** — Stores and retrieves reusable patterns from past interactions with confidence scoring.
- **`get_context_for_query()`** — Assembles relevant context across all memory tiers with tier-specific decay functions.
- **Memory evolution** — Automatic promotion of frequently-accessed working memory items to semantic memory.

#### MCP Server (v0.2)
- **Dedicated `neurograph-mcp` crate** — Separated MCP server into its own workspace crate for cleaner architecture.
- **Dual transport** — Supports both stdio (for Claude Desktop) and SSE (for Cursor/web) transports.
- **MCP config files** — Pre-built `claude_desktop_config.json` and `cursor_config.json` for easy setup.

#### Infrastructure
- **Docker multi-stage build** — Optimized Dockerfile with builder and slim runtime stages. Non-root user, no embedded secrets, minimal attack surface.
- **Docker Compose** — Full-stack deployment with API server, dashboard, Neo4j, and observability stack.
- **GitHub Actions CI** — Automated testing, linting, formatting, security scanning, and release pipeline for Linux, macOS, and Windows.
- **OpenSSF Scorecard** — Weekly security posture assessment with public badge.
- **CodeQL analysis** — Semantic code analysis on every PR (Rust + TypeScript).
- **Dependabot** — Automated dependency monitoring for Rust and npm.
- **cargo-deny** — License compliance and vulnerability policy enforcement.
- **SBOM generation** — CycloneDX Software Bill of Materials for each release.
- **git-cliff changelog** — Automated changelog generation from Conventional Commits.

#### Documentation
- **Architecture documentation** — Full system design with Mermaid diagrams, data flow, and module descriptions.
- **Temporal Engine documentation** — Bi-temporal model, forgetting, branching, timeline visualization.
- **Community Detection documentation** — Louvain/Leiden algorithms, tuning, incremental updates, summarization.
- **Developer Guide** — Build, test, debug, profile, and IDE setup instructions.
- **Contributing Guide** — PR workflow, code standards, commit conventions, good first issues.
- **Security Policy** — Threat model, vulnerability reporting, trust boundaries, security roadmap.
- **Competitive Analysis** — Honest comparison with Graphiti, GraphRAG, Mem0.
- **RAI Transparency** — Responsible AI documentation covering data handling, bias considerations.
- **Code of Conduct** — Contributor Covenant.
- **CITATION.cff** — Academic citation format.
- **MAINTAINERS.md** — Maintainer list and responsibilities.
- **SUPPORT.md** — Support channels and resources.

#### Community Integration
- **GitHub Discussions** — Enabled for Q&A, ideas, and show-and-tell.
- **Issue templates** — Bug report and feature request with structured forms.
- **PR template** — Standardized pull request checklist.
- **CODEOWNERS** — Automated reviewer assignment for critical paths.

### Changed
- Upgraded minimum supported Rust version (MSRV) to 1.82.
- Moved from `NetworkX` (Python) to pure Rust for all graph algorithms.

### Fixed
- (No fixes yet — this is the initial release.)

### Security
- All CI actions pinned to SHA hashes (not tags).
- Docker images run as non-root user (UID 1000).
- No secrets embedded in container images.
- `cargo audit` and `cargo deny` enforced in CI.

---

## [0.1.0] — Unreleased (Target: Q2 2026)

This will be the first stable release. See the [Unreleased] section above for planned contents.

### Release Criteria
- [ ] All Stable features passing CI on Linux, macOS, Windows
- [ ] Benchmark suite automated in CI
- [ ] `crates.io` publish verified
- [ ] Docker image published to `ghcr.io`
- [ ] Documentation complete for all public APIs

---

<!-- Links -->
[Unreleased]: https://github.com/neurographai/neurograph/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/neurographai/neurograph/releases/tag/v0.1.0

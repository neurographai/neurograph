<p align="center">
  <a href="https://github.com/neurographai/neurograph/stargazers"><img src="https://img.shields.io/github/stars/neurographai/neurograph?style=for-the-badge&color=yellow" alt="GitHub stars"/></a>
  <a href="https://github.com/neurographai/neurograph/network/members"><img src="https://img.shields.io/github/forks/neurographai/neurograph?style=for-the-badge&color=blue" alt="GitHub forks"/></a>
  <a href="https://github.com/neurographai/neurograph/issues"><img src="https://img.shields.io/github/issues/neurographai/neurograph?style=for-the-badge&color=red" alt="GitHub issues"/></a>
  <a href="https://github.com/neurographai/neurograph/blob/main/LICENSE"><img src="https://img.shields.io/github/license/neurographai/neurograph?style=for-the-badge" alt="License"/></a>
  <a href="https://github.com/neurographai/neurograph/discussions"><img src="https://img.shields.io/github/discussions/neurographai/neurograph?style=for-the-badge&color=purple&logo=github&label=Discussions" alt="GitHub Discussions"/></a>
</p>

# NeuroGraph

> **The Operating System for AI Knowledge**
> Ingest anything · Remember everything · Forget intelligently · Reason visually · Branch reality

<br/>
<p align="center">
  <!-- TODO: Replace with actual architecture diagram -->
  <img src="https://raw.githubusercontent.com/neurographai/neurograph/main/docs/assets/architecture-placeholder.png" alt="NeuroGraph Architecture" style="max-width: 100%; height: auto;" />
</p>
<br/>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust"/>
  <img src="https://img.shields.io/badge/WebAssembly-654FF0?style=for-the-badge&logo=webassembly&logoColor=white" alt="WebAssembly"/>
  <img src="https://img.shields.io/badge/React-20232A?style=for-the-badge&logo=react&logoColor=61DAFB" alt="React"/>
  <img src="https://img.shields.io/badge/TypeScript-007ACC?style=for-the-badge&logo=typescript&logoColor=white" alt="TypeScript"/>
  <img src="https://img.shields.io/badge/Vite-B73BFE?style=for-the-badge&logo=vite&logoColor=FFD62E" alt="Vite"/>
  <img src="https://img.shields.io/badge/G6-1890FF?style=for-the-badge&logo=antdesign&logoColor=white" alt="AntV G6"/>
</p>

## Quick Start

```python
from neurograph import NeuroGraph
ng = NeuroGraph()
await ng.add("NeuroGraph is a temporal knowledge graph.")
result = await ng.query("What is NeuroGraph?")
```

## Features

| Capability | Status |
|---|---|
| **Temporal Knowledge Graph** - Bi-temporal facts with `valid_from` / `valid_until` | Completed |
| **Community Detection** - Hierarchical Louvain/Leiden in native Rust | Completed |
| **Interactive Dashboard** - Browser-based graph explorer powered by G6 + Rust WASM | Completed |
| **Sub-200ms Queries** - Rust-native hybrid retrieval (semantic + keyword + graph walk) | Completed |
| **Graph Version Control** - Branch, diff, and merge knowledge graphs like Git | Completed |
| **Think-While-You-Watch** - Watch AI reasoning animate on the graph in real-time | Completed |
| **Temporal Playback** - Scrub a timeline slider to see knowledge evolve | Completed |
| **Intelligent Forgetting** - Importance-based decay, compression, and archival | Completed |
| **Cost-Aware Routing** - Auto-selects cheapest query strategy within your budget | Completed |
| **Multi-Agent Graph Building** - 5 collaborative agents with visual debugging | Completed |
| **MCP Server** - Give Claude, Cursor, and other AI tools graph-based memory | Completed |
| **Zero Config** - `pip install neurograph` -> 3 lines -> done. No Docker. No API key. | Completed |

## What Makes NeuroGraph Different

| Feature | What It Does | Why It Matters |
|---------|-------------|----------------|
| **Think-While-You-Watch** | Ask a question -> watch the AI traverse the graph in real-time, nodes glowing and edges animating as it reasons | You can SEE how the AI arrived at its answer - full transparency |
| **Temporal Playback** | Drag a timeline slider -> the knowledge graph morphs to show what was true at any point in history | Track how knowledge evolves - like Git blame for facts |
| **Graph Branching** | `ng.branch("what-if")` -> add hypothetical facts -> `ng.diff()` to compare -> `ng.merge()` when verified | Explore hypothetical scenarios without corrupting your real knowledge |
| **Intelligent Forgetting** | Facts automatically decay based on importance (PageRank + access frequency + recency) | Graphs don't grow forever - NeuroGraph manages its own memory |
| **Cost-Aware Router** | Set a budget -> NeuroGraph auto-picks the cheapest strategy that meets quality | Never get a surprise LLM bill again |
| **Rust WASM Layouts** | Graph layout computed in Rust WebAssembly, not JavaScript | Render 100k-node graphs smoothly where JS would crash |
| **Zero API Key Mode** | Works completely offline: regex NER + local embeddings + embedded DB | Air-gapped environments, privacy-first, $0 cost |
| **Diff-Based Summaries** | When a community changes slightly, update the summary - don't regenerate it | ~70% token savings on community re-summarization |
| **Hybrid Search** | Semantic + BM25 + Graph Traversal fused with Reciprocal Rank Fusion | Better recall than any single search method alone |
| **Built-In Benchmarks** | `neurograph bench` runs accuracy, latency, and cost tests against standard datasets | Know exactly how good your graph is - with numbers |

## Why NeuroGraph?

Most graph-based AI memory systems store static snapshots of facts. When reality changes ("Alice moved to London"), they either append a conflicting fact or overwrite the old one, losing the timeline. They also rely on heavy Python abstractions that crash when visualizing massive graphs.

**NeuroGraph** solves this by:
1. **Bi-Temporal Tracking:** Every fact has a birth, a lifecycle, and a validation window. You can query the knowledge graph "as it was last Tuesday".
2. **Rust-Native Core:** Queries, community detection (Louvain/Leiden), and graph traversals are executed in native Rust, providing sub-200ms latency.
3. **WASM-Powered Visualization:** Layout algorithms are compiled to WebAssembly and run directly in the browser, allowing smooth visualization of massive graphs via AntV G6 natively.

## How NeuroGraph Compares

| Feature | NeuroGraph | GraphRAG (Microsoft) | Graphiti (Zep) | Mem0 |
|---------|-----------|----------------------|----------------|------|
| **Core Value**| **Bi-Temporal Knowledge & Extensible OS**| Batch document analysis | Temporal knowledge graph | Simple developer memory |
| **Query Speed** | **<200ms** (Rust) | ~5-10s | ~1s | ~1s |
| **Language** | Native Rust (Core), Py/TS wrappers | Python | Python | Python |
| **Temporal**| **Bi-temporal** | Static | Edge-based time | Recency only |
| **Dashboard** | **Interactive Browser** (G6 + WASM)| None / External Gephi | None / Neo4j Browser | Standard UI |
| **Self-Hosted** | **$0 Offline Mode** (Regex + FastEmbed)| High LLM cost for map stage | Free | Free |
| **Communities** | **Rust Louvain/Leiden** | Python NetworkX | No | No |
| **Decay**| **Importance-based Forgetting** | No | No | Optional API |

## Performance

Built on Rust. No GIL. No garbage collector. No excuses.

| Metric | NeuroGraph |
|--------|-----------|
| **Query Latency (P50)** | <150ms |
| **Query Latency (P99)** | <500ms |
| **Indexing Speed** | ~500 docs/min |
| **Graph Layout (100k nodes)** | <500ms (Rust WASM) |
| **Community Detection (50k edges)** | <1s (Rust native) |
| **Memory Usage** | ~50MB base |
| **Cold Start** | <2s |
| **Cost per 1k Queries** | ~$0.50 (with GPT-4o-mini) |
| **Cost per 1k Queries** | $0.00 (fully local / Ollama) |
| **Max Tested Graph Size** | 1M+ nodes |

<details>
<summary>Benchmark methodology</summary>

- Hardware: Single M2 MacBook Pro (16GB RAM)
- Dataset: 1,000 news articles (~500k tokens)
- LLM: GPT-4o-mini for extraction, FastEmbed local for search
- Database: Embedded sled (zero-config)
- Full methodology: [benchmarks/README.md](./benchmarks/README.md)

</details>

## API at a Glance

| Operation | Python | Rust |
|-----------|--------|------|
| **Create** | `ng = NeuroGraph()` | `let ng = NeuroGraph::builder().build().await?;` |
| **Ingest text** | `await ng.add("Alice joined Anthropic")` | `ng.add_text("Alice joined Anthropic").await?;` |
| **Ingest JSON** | `await ng.add({"name": "Alice"})` | `ng.add_json(json!({"name": "Alice"})).await?;` |
| **Query** | `result = await ng.query("Where does Alice work?")` | `let result = ng.query("Where does Alice work?").await?;` |
| **Time travel** | `past = await ng.at("2025-01-01")` | `let past = ng.at("2025-01-01").await?;` |
| **Entity history** | `history = await ng.history("alice")` | `let history = ng.history("alice").await?;` |
| **What changed** | `diff = await ng.what_changed("2025-01", "2026-01")` | `let diff = ng.what_changed("2025-01", "2026-01").await?;` |
| **Branch** | `await ng.branch("hypothesis")` | `ng.branch("hypothesis").await?;` |
| **Diff branches** | `diff = ng.diff_branches("main", "hypothesis")` | `let diff = ng.diff_branches("main", "hypothesis")?;` |
| **Communities** | `ng.detect_communities()` | `ng.detect_communities();` |
| **Dashboard** | `await ng.dashboard()` | `ng.serve(7777).await?;` |
| **Search** | `entities = await ng.search("Alice")` | `let entities = ng.search("Alice").await?;` |

## Integrations

### LLM Providers

| Provider | Models | Local/Cloud |
|----------|--------|-------------|
| OpenAI | GPT-4o, GPT-4o-mini, GPT-4-turbo | Cloud |
| Anthropic | Claude 4, Claude 3.5 Sonnet | Cloud |
| Google Gemini | Gemini 2.0 Flash, Gemini Pro | Cloud |
| Ollama | Llama 3, DeepSeek, Mistral, Phi | Local |
| Any OpenAI-compatible | LM Studio, vLLM, Together AI | Local/Cloud |
| **None (offline mode)** | **Regex NER + rule-based extraction** | **Local** |

### Graph Databases

| Backend | Type | Setup Required |
|---------|------|---------------|
| **Embedded (sled)** | **Embedded** | **None - default** |
| In-Memory (petgraph) | In-process | None |
| Kuzu | Embedded | None |
| Neo4j | Client-server | Docker or Neo4j Desktop |
| FalkorDB | Client-server | Docker |

### Embedding Providers

| Provider | Models | Local/Cloud |
|----------|--------|-------------|
| **FastEmbed (default)** | **bge-small-en-v1.5** | **Local** |
| OpenAI | text-embedding-3-small/large | Cloud |
| Sentence Transformers | Any HuggingFace model | Local |

### Agent Frameworks

| Framework | Integration Type |
|-----------|-----------------|
| LangGraph | Drop-in memory adapter |
| CrewAI | Shared memory across agents |
| AutoGen | Memory adapter |
| OpenAI Agents SDK | Compatible memory interface |
| MCP (Claude/Cursor) | Full MCP server |

### Observability

| Tool | What's Tracked |
|------|---------------|
| OpenTelemetry | Distributed traces for every operation |
| Prometheus | Metrics: latency, throughput, cache hits |
| Built-in Cost Tracker | Per-query: model, tokens, cost USD, latency ms |

## Architecture

NeuroGraph separates concerns between the heavy analysis runtime and a lightweight visualization frontend.

1. **NeuroGraph Core (Rust)**: Manages bi-temporal memory storage, incremental community detection (Louvain/Hierarchical Leiden), LLM-based ingestion, vector search, and observability.
2. **NeuroGraph Dashboard (React/TS)**: Integrates modern components alongside AntV G6 for highly complex graph renderings, temporal sliders for playback, and minimap analysis.
3. **NeuroGraph WASM**: Binds critical core compute logic directly to the dashboard, ensuring browser side processing operates seamlessly without server dependencies.

## Installation

### Via PIP (Recommended)
```bash
pip install neurograph
```

### Via Docker
Ensure you have Docker and Docker Compose installed.
```bash
docker-compose up --build
```
- REST API / Core Engine: `http://localhost:8000`
- Visualization Dashboard: `http://localhost:3000`

### Developer Setup (Source)
Prerequisites: Rust (cargo 1.80+), Node.js (v18+)

```bash
# Build Engine
cd crates/neurograph-core
cargo build --release

# Build Dashboard
cd ../../dashboard
npm install
npm run dev
```

## Complete Feature Matrix

<details>
<summary>Click to expand all 87 features</summary>

### Reasoning and Knowledge

| Feature | Details |
|---------|---------|
| Entity extraction (LLM) | Structured JSON output via OpenAI / Anthropic / Gemini / Ollama |
| Entity extraction (offline) | Regex-based NER fallback - works without any API key |
| Relationship extraction | Automatic from text + manual from structured JSON |
| Multi-hop reasoning | Graph walk + LLM reasoning across connected entities |
| Community detection (Louvain) | Native Rust implementation on petgraph - O(n log n) |
| Community detection (Leiden) | Hierarchical with resolution parameter and level control |
| Incremental community updates | k-hop delta recomputation - only affected neighborhoods |
| Community summarization | LLM map-reduce with hierarchical rollup |
| Diff-based re-summarization | Update summaries incrementally at ~30% token cost |
| Hierarchical community levels | Multi-resolution from macro themes to micro topics |
| Cost-aware query routing | Classifies query -> estimates cost per strategy -> selects optimal |
| Local queries | Direct entity/subgraph retrieval (fastest, cheapest) |
| Global queries | Community summary map-reduce (comprehensive) |
| DRIFT search | Dynamic local-global fusion |
| Temporal queries | Time-filtered retrieval respecting fact validity windows |
| Multi-hop queries | Graph traversal + LLM chain reasoning |

### Retrieval and Search

| Feature | Details |
|---------|---------|
| Semantic vector search | Cosine similarity on embeddings (OpenAI / FastEmbed / any provider) |
| BM25 keyword search | Full-text search via tantivy |
| Graph traversal search | Scored BFS/DFS from seed entities |
| Hybrid retrieval | Reciprocal Rank Fusion (RRF) combining all three methods |
| Cross-encoder reranking | LLM-based passage relevance scoring |
| Pre-built search recipes | `find_entity`, `find_connections`, `find_community`, `temporal_search` |
| Context assembly | Token-budget-aware graph -> LLM prompt with citations + confidence |

### Temporal and Data Management

| Feature | Details |
|---------|---------|
| Bi-temporal model | Every fact has `valid_from` and `valid_until` timestamps |
| Automatic fact invalidation | New contradicting facts invalidate old ones (not delete) |
| Point-in-time queries | `ng.at("2026-03-15")` returns graph state at that moment |
| Entity history | `ng.history("alice")` returns full chronological fact chain |
| Temporal diff | `ng.what_changed("2026-01", "2026-06")` shows additions/removals |
| Graph branching | `ng.branch("hypothesis")` creates copy-on-write branch |
| Graph diff | `ng.diff_branches("main", "hypothesis")` shows differences |
| Graph merge | 4 strategies: SourceWins, TargetWins, VerifiedOnly, TemporalMerge |
| Named snapshots | Immutable point-in-time snapshots with labels |
| Intelligent forgetting | Importance scoring via PageRank + access frequency + recency |
| Configurable decay | Exponential, linear, step-function, or no decay |
| Compression | Merge low-importance similar entities into summaries |
| Archival | Move old facts to cold storage after configurable TTL |
| Episode/provenance tracking | Every fact traces back to source data (text, JSON, file) |
| Prescribed ontology | Define entity/edge types upfront via typed schemas |
| Learned ontology | Automatically discover entity types from data |
| 2-phase deduplication | Phase 1: embedding similarity + hash. Phase 2: LLM fallback |
| Contradiction resolution | Temporal invalidation with full history preserved |

### Visualization and Dashboard

| Feature | Details |
|---------|---------|
| Built-in interactive dashboard | `await ng.dashboard()` opens browser at localhost:7777 |
| WebGL/Canvas rendering | G6 engine with multi-layer canvas (background, main, label) |
| Force-directed layout | Rust WASM - 10-50x faster than JavaScript equivalent |
| Hierarchical layout | Dagre-style for tree/DAG structures |
| Radial layout | Ego-centric view centered on selected entity |
| Circular layout | For small, dense subgraphs |
| Temporal layout | X-axis = time, Y-axis = entity groups |
| Think-While-You-Watch | Live animation of AI reasoning path on the graph |
| Temporal playback slider | Scrub through time - nodes appear/disappear as facts change |
| Semantic zoom | Zoom in = entity details. Zoom out = community summaries |
| Community clusters | G6 Combos with color-coded boundaries |
| Natural language search | Type a question -> results highlight paths on graph |
| Entity detail panel | Click any node -> see summary, relationships, full history |
| Relationship detail panel | Click any edge -> see fact, validity window, provenance |
| Community detail panel | Click any cluster -> see summary, members, sub-communities |
| Graph statistics panel | Node count, edge count, community count, cost tracker |
| Dark mode | Default premium dark theme with glassmorphism |
| Light mode | Clean light theme for presentations |
| Minimap | Bird's-eye view navigation |
| Tooltips | Hover to preview entity/relationship info |
| Context menu | Right-click for actions (expand, hide, explore) |
| Cost meter | Real-time query cost gauge |
| React embeddable components | `@neurograph/react` for embedding in any app |
| Playback controls | Play, pause, speed (1x/2x/5x/10x) for temporal animation |

### Agent and Integration Support

| Feature | Details |
|---------|---------|
| Agent memory interface | `ng.as_memory()` returns framework-compatible memory |
| Builder agent | Extracts entities and relationships from data |
| Validator agent | Verifies extracted facts against sources |
| Conflict resolver agent | Resolves contradictions between facts |
| Schema aligner agent | Ensures new entities match prescribed ontology |
| Summarizer agent | Generates and updates community summaries |
| Multi-agent visual debugging | Watch agents negotiate on the graph in real-time |
| LangGraph integration | Drop-in memory adapter for LangGraph agents |
| CrewAI integration | Shared knowledge graph across crew members |
| AutoGen integration | Memory adapter for AutoGen agents |
| OpenAI Agents SDK | Compatible memory interface |
| MCP server | Full Model Context Protocol server for Claude/Cursor |

### Infrastructure and Developer Experience

| Feature | Details |
|---------|---------|
| Zero-config embedded mode | `pip install neurograph` -> 3 lines -> works |
| Embedded database (sled) | Persistent storage, no Docker required |
| In-memory mode | Pure petgraph backend for testing and prototyping |
| Neo4j driver | Connect to existing Neo4j instances |
| FalkorDB driver | Connect to FalkorDB for redis-speed graph queries |
| Kuzu driver | Embedded analytical graph database |
| OpenAI LLM | GPT-4o, GPT-4o-mini, GPT-4-turbo |
| Anthropic LLM | Claude 4, Claude 3.5 Sonnet |
| Google Gemini | Gemini 2.0 Flash, Gemini Pro |
| Ollama (local) | DeepSeek, Llama, Mistral - fully offline |
| Generic OpenAI-compatible | Any API following OpenAI spec (LM Studio, vLLM, etc.) |
| No API key mode | Regex NER + local FastEmbed - zero cost, zero internet |
| REST API | Axum-based, async, production-ready |
| WebSocket | Real-time graph updates + reasoning traces |
| Python SDK | Native PyO3 bindings - Rust speed with Python API |
| TypeScript SDK | `@neurograph/client` for Node.js + browser |
| Rust SDK | Native, zero-overhead Rust API |
| CLI | `neurograph serve`, `neurograph ingest`, `neurograph query` |
| Docker | Multi-stage build, slim runtime image |
| Docker Compose | Full stack (server + dashboard) or embedded-only |
| OpenTelemetry | Distributed tracing + Prometheus metrics |
| Per-operation cost tracking | Every LLM call logs: model, tokens, cost, latency |
| Cost dashboard | Visual cost breakdown in the browser |
| Built-in benchmark suite | Compare your graph quality against standard datasets |
| Plugin system | Trait-based hooks: `on_ingest`, `on_query`, `on_extract` |
| Biomedical plugin (example) | Gene/protein/disease extraction |
| Codebase plugin (example) | Source code analysis and dependency graphing |

</details>

## Documentation

Full documentation is available across the repository.
Detailed guides:
- [Architecture Details](docs/architecture.md)
- [Temporal Engine Guide](docs/temporal.md)
- [Community Detection](docs/community.md)

## Roadmap

- Upcoming features involve expanding the distributed computing models for the graph and enriching the Python SDK hooks.
- See the main issue tracker for feature voting.

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change. 

## License

Apache-2.0 License.

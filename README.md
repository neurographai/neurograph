<p align="center">
  <img src="https://img.shields.io/badge/NeuroGraph-Temporal_Knowledge_Graphs_for_AI-blueviolet?style=for-the-badge" alt="NeuroGraph"/>
</p>

<p align="center">
  <a href="https://crates.io/crates/neurograph"><img src="https://img.shields.io/crates/v/neurograph?style=flat-square&logo=rust&logoColor=white&label=crates.io&color=e6522c" alt="crates.io"/></a>
  <a href="https://pypi.org/project/neurograph/"><img src="https://img.shields.io/pypi/v/neurograph?style=flat-square&logo=pypi&logoColor=white&label=PyPI&color=3775A9" alt="PyPI"/></a>
  <a href="https://www.npmjs.com/package/@neurograph/sdk"><img src="https://img.shields.io/npm/v/@neurograph/sdk?style=flat-square&logo=npm&logoColor=white&label=npm&color=CB3837" alt="npm"/></a>
  <a href="https://ghcr.io/neurographai/neurograph"><img src="https://img.shields.io/badge/Docker-ghcr.io-2496ED?style=flat-square&logo=docker&logoColor=white" alt="Docker"/></a>
  <a href="https://docs.rs/neurograph"><img src="https://img.shields.io/docsrs/neurograph?style=flat-square&logo=docs.rs&logoColor=white&label=docs.rs" alt="docs.rs"/></a>
</p>

<p align="center">
  <a href="https://github.com/neurographai/neurograph/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/neurographai/neurograph/ci.yml?style=flat-square&logo=github&label=CI" alt="CI"/></a>
  <a href="https://codecov.io/gh/neurographai/neurograph"><img src="https://img.shields.io/codecov/c/github/neurographai/neurograph?style=flat-square&logo=codecov&logoColor=white" alt="Codecov"/></a>
  <a href="https://scorecard.dev/viewer/?uri=github.com/neurographai/neurograph"><img src="https://img.shields.io/ossf-scorecard/github.com/neurographai/neurograph?style=flat-square&label=OpenSSF" alt="OpenSSF Scorecard"/></a>
  <a href="https://github.com/neurographai/neurograph/blob/main/LICENSE"><img src="https://img.shields.io/github/license/neurographai/neurograph?style=flat-square&color=blue" alt="License"/></a>
</p>

<p align="center">
  <a href="https://github.com/neurographai/neurograph/stargazers"><img src="https://img.shields.io/github/stars/neurographai/neurograph?style=flat-square&color=yellow" alt="Stars"/></a>
  <a href="https://github.com/neurographai/neurograph/network/members"><img src="https://img.shields.io/github/forks/neurographai/neurograph?style=flat-square&color=blue" alt="Forks"/></a>
  <a href="https://github.com/neurographai/neurograph/issues"><img src="https://img.shields.io/github/issues/neurographai/neurograph?style=flat-square&color=red" alt="Issues"/></a>
  <a href="https://github.com/neurographai/neurograph/discussions"><img src="https://img.shields.io/github/discussions/neurographai/neurograph?style=flat-square&color=purple&label=Discussions" alt="Discussions"/></a>
</p>

---

# NeuroGraph

> A Rust-powered temporal knowledge graph engine with interactive visualization, built for AI agents that need to remember, reason, and forget.

NeuroGraph is an open-source knowledge graph engine that treats **time as a first-class dimension**. Every fact has a validity window, every query can time-travel, and the graph can branch like Git. It's designed to be the memory layer for AI agents — from personal assistants to multi-agent research systems.

<br/>

## Architecture

```mermaid
%%{init: {'theme': 'dark', 'themeVariables': {'primaryColor': '#6C5CE7', 'primaryTextColor': '#DFE6E9', 'primaryBorderColor': '#A29BFE', 'lineColor': '#74B9FF', 'secondaryColor': '#00CEC9', 'tertiaryColor': '#2D3436', 'background': '#0D1117', 'mainBkg': '#161B22', 'nodeBorder': '#A29BFE', 'clusterBkg': '#161B2288', 'clusterBorder': '#30363D', 'titleColor': '#F8F8F2', 'edgeLabelBackground': '#161B22'}}}%%

graph TB
    subgraph CLIENTS["Client SDKs"]
        direction LR
        PY["Python SDK<br/><i>pip install neurograph</i>"]
        TS["TypeScript SDK<br/><i>npm install @neurograph/sdk</i>"]
        RS["Rust Crate<br/><i>cargo add neurograph</i>"]
        CLI["CLI<br/><i>neurograph query ...</i>"]
    end

    subgraph GATEWAY["API Gateway"]
        direction LR
        REST["REST API<br/><i>Axum</i>"]
        WS["WebSocket<br/><i>Real-time</i>"]
        MCP["MCP Server<br/><i>Claude / Cursor</i>"]
    end

    subgraph ENGINE["Core Engine — Rust"]
        direction TB

        subgraph INGESTION["Ingestion Pipeline"]
            direction LR
            PARSER["Text / JSON<br/>Parser"]
            NER["Entity<br/>Extractor"]
            REL["Relationship<br/>Extractor"]
            DEDUP["2-Phase<br/>Dedup"]
        end

        subgraph KNOWLEDGE["Knowledge Layer"]
            direction LR
            TEMPORAL["Bi-Temporal<br/>Engine"]
            BRANCH["Branch &<br/>Merge"]
            COMMUNITY["Community<br/>Detection"]
            DECAY["Intelligent<br/>Forgetting"]
        end

        subgraph RETRIEVAL["Hybrid Retrieval"]
            direction LR
            SEMANTIC["Semantic<br/>Search"]
            BM25["BM25<br/>Keyword"]
            GRAPH_WALK["Graph<br/>Walk"]
            RRF["RRF<br/>Fusion"]
        end

        subgraph AGENT_SYS["Multi-Agent System"]
            direction LR
            BUILDER["Builder"]
            VALIDATOR["Validator"]
            RESOLVER["Resolver"]
            SUMMARIZER["Summarizer"]
        end
    end

    subgraph STORAGE["Storage Backends"]
        direction LR
        SLED["Sled<br/><i>Embedded</i>"]
        NEO4J["Neo4j<br/><i>Client-Server</i>"]
        FALKOR["FalkorDB<br/><i>Redis-Speed</i>"]
        KUZU["Kuzu<br/><i>Embedded OLAP</i>"]
        MEMORY["In-Memory<br/><i>petgraph</i>"]
    end

    subgraph DASHBOARD["Interactive Dashboard"]
        direction LR
        REACT["React 19"]
        G6["AntV G6"]
        WASM["Rust WASM<br/><i>Layout Engine</i>"]
        TIMELINE["Temporal<br/>Playback"]
    end

    subgraph OBSERVE["Observability"]
        direction LR
        OTEL["OpenTelemetry"]
        PROM["Prometheus"]
        COST["Cost Tracker"]
    end

    CLIENTS --> GATEWAY
    GATEWAY --> ENGINE
    ENGINE --> STORAGE
    ENGINE --> OBSERVE
    GATEWAY --> DASHBOARD
    WASM -.->|"compiled to WASM"| ENGINE

    PARSER --> NER --> REL --> DEDUP
    SEMANTIC --> RRF
    BM25 --> RRF
    GRAPH_WALK --> RRF

    classDef clientNode fill:#6C5CE7,stroke:#A29BFE,color:#fff,stroke-width:2px
    classDef gatewayNode fill:#0984E3,stroke:#74B9FF,color:#fff,stroke-width:2px
    classDef engineNode fill:#00B894,stroke:#55EFC4,color:#fff,stroke-width:2px
    classDef storageNode fill:#E17055,stroke:#FAB1A0,color:#fff,stroke-width:2px
    classDef dashNode fill:#FDCB6E,stroke:#FFEAA7,color:#2D3436,stroke-width:2px
    classDef observeNode fill:#636E72,stroke:#B2BEC3,color:#fff,stroke-width:2px

    class PY,TS,RS,CLI clientNode
    class REST,WS,MCP gatewayNode
    class PARSER,NER,REL,DEDUP,TEMPORAL,BRANCH,COMMUNITY,DECAY,SEMANTIC,BM25,GRAPH_WALK,RRF,BUILDER,VALIDATOR,RESOLVER,SUMMARIZER engineNode
    class SLED,NEO4J,FALKOR,KUZU,MEMORY storageNode
    class REACT,G6,WASM,TIMELINE dashNode
    class OTEL,PROM,COST observeNode
```

<br/>

<!-- Tech Stack -->
<p align="center">
  <b>Core</b><br/>
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust"/>
  <img src="https://img.shields.io/badge/WebAssembly-654FF0?style=for-the-badge&logo=webassembly&logoColor=white" alt="WebAssembly"/>
  <img src="https://img.shields.io/badge/Tokio-232323?style=for-the-badge&logo=rust&logoColor=white" alt="Tokio"/>
  <img src="https://img.shields.io/badge/PyO3-3776AB?style=for-the-badge&logo=python&logoColor=white" alt="PyO3"/>
</p>
<p align="center">
  <b>Storage</b><br/>
  <img src="https://img.shields.io/badge/Sled-E6522C?style=for-the-badge&logo=rust&logoColor=white" alt="Sled"/>
  <img src="https://img.shields.io/badge/Tantivy-FF6B35?style=for-the-badge&logo=apache-lucene&logoColor=white" alt="Tantivy"/>
  <img src="https://img.shields.io/badge/Neo4j-4581C3?style=for-the-badge&logo=neo4j&logoColor=white" alt="Neo4j"/>
  <img src="https://img.shields.io/badge/FalkorDB-DC382D?style=for-the-badge&logo=redis&logoColor=white" alt="FalkorDB"/>
  <img src="https://img.shields.io/badge/Kuzu-00A98F?style=for-the-badge&logo=database&logoColor=white" alt="Kuzu"/>
</p>
<p align="center">
  <b>Frontend</b><br/>
  <img src="https://img.shields.io/badge/React-20232A?style=for-the-badge&logo=react&logoColor=61DAFB" alt="React"/>
  <img src="https://img.shields.io/badge/TypeScript-007ACC?style=for-the-badge&logo=typescript&logoColor=white" alt="TypeScript"/>
  <img src="https://img.shields.io/badge/Vite-B73BFE?style=for-the-badge&logo=vite&logoColor=FFD62E" alt="Vite"/>
  <img src="https://img.shields.io/badge/AntV_G6-1890FF?style=for-the-badge&logo=antdesign&logoColor=white" alt="AntV G6"/>
</p>
<p align="center">
  <b>AI / ML</b><br/>
  <img src="https://img.shields.io/badge/OpenAI-412991?style=for-the-badge&logo=openai&logoColor=white" alt="OpenAI"/>
  <img src="https://img.shields.io/badge/Anthropic-191919?style=for-the-badge&logo=anthropic&logoColor=white" alt="Anthropic"/>
  <img src="https://img.shields.io/badge/Ollama-000000?style=for-the-badge&logo=ollama&logoColor=white" alt="Ollama"/>
  <img src="https://img.shields.io/badge/FastEmbed-FF6F00?style=for-the-badge&logo=python&logoColor=white" alt="FastEmbed"/>
</p>
<p align="center">
  <b>Infrastructure</b><br/>
  <img src="https://img.shields.io/badge/Docker-2496ED?style=for-the-badge&logo=docker&logoColor=white" alt="Docker"/>
  <img src="https://img.shields.io/badge/GitHub_Actions-2088FF?style=for-the-badge&logo=github-actions&logoColor=white" alt="GitHub Actions"/>
  <img src="https://img.shields.io/badge/Prometheus-E6522C?style=for-the-badge&logo=prometheus&logoColor=white" alt="Prometheus"/>
  <img src="https://img.shields.io/badge/OpenTelemetry-7B5EA7?style=for-the-badge&logo=opentelemetry&logoColor=white" alt="OpenTelemetry"/>
</p>

---

## Quick Start

```python
from neurograph import NeuroGraph

ng = NeuroGraph()

# Ingest knowledge
await ng.add("Alice joined Anthropic as a research scientist in March 2026")
await ng.add("Bob moved from Google to OpenAI in January 2026")

# Query with graph-powered RAG
result = await ng.query("Where does Alice work?")
print(result.answer)  # "Anthropic"

# Time travel
past = await ng.at("2025-12-01")
result = await past.query("Where does Bob work?")
print(result.answer)  # "Google"

# Branch reality
await ng.branch("what-if")
await ng.add("Alice leaves Anthropic for DeepMind")
diff = ng.diff_branches("main", "what-if")

# Open interactive dashboard
await ng.dashboard()  # localhost:7777
```

---

## Install

```bash
# Rust
cargo install neurograph

# Python
pip install neurograph

# Node / TypeScript
npm install @neurograph/sdk

# Docker (API + Dashboard)
docker run -p 8000:8000 -p 3000:3000 ghcr.io/neurographai/neurograph

# Docker Compose (full stack)
docker compose up
```

<details>
<summary><b>Build from source</b></summary>

Prerequisites: Rust 1.82+, Node.js 18+

```bash
git clone https://github.com/neurographai/neurograph.git
cd neurograph
cargo build --release
cd dashboard && npm install && npm run dev
```

</details>

---

## Feature Status

> **Legend:** `Stable` = production-ready, API stable | `Beta` = functional, breaking changes possible | `Experimental` = proof-of-concept | `Planned` = on roadmap

| Capability | Status |
|---|---|
| **Temporal Knowledge Graph** — Bi-temporal facts with `valid_from` / `valid_until` | **Stable** |
| **Community Detection** — Louvain/Leiden in native Rust | **Stable** |
| **Hybrid Retrieval** — Semantic + BM25 + graph walk with RRF fusion | **Stable** |
| **Cost-Aware Routing** — Auto-selects cheapest query strategy within budget | **Stable** |
| **Zero Config** — `pip install neurograph`, 3 lines, works. No API key needed. | **Stable** |
| **Interactive Dashboard** — Browser graph explorer (G6 + Rust WASM layouts) | **Beta** |
| **Graph Version Control** — Branch, diff, merge knowledge graphs | **Beta** |
| **Temporal Playback** — Timeline slider to scrub through knowledge history | **Beta** |
| **Think-While-You-Watch** — Real-time reasoning animation on graph | **Experimental** |
| **Intelligent Forgetting** — Importance-based decay and compression | **Experimental** |
| **Multi-Agent Graph Building** — Collaborative agents for extraction/validation | **Experimental** |
| **MCP Server** — Claude/Cursor integration via Model Context Protocol | **Experimental** |
| **Python SDK** — Native PyO3 bindings | **Planned** |
| **TypeScript SDK** — WASM-powered browser/Node client | **Planned** |
| **Distributed Sharding** — Scale across multiple nodes | **Planned** |

<details>
<summary><b>Full feature breakdown</b></summary>

### Reasoning and Knowledge

| Feature | Details |
|---------|---------|
| Entity extraction (LLM) | Structured JSON output via OpenAI / Anthropic / Gemini / Ollama |
| Entity extraction (offline) | Regex-based NER fallback — works without any API key |
| Relationship extraction | Automatic from text + manual from structured JSON |
| Multi-hop reasoning | Graph walk + LLM reasoning across connected entities |
| Community detection (Louvain) | Native Rust implementation on petgraph |
| Community detection (Leiden) | Hierarchical with resolution parameter |
| Incremental community updates | k-hop delta recomputation |
| Community summarization | LLM map-reduce with hierarchical rollup |
| Diff-based re-summarization | Update summaries incrementally (~30% token cost vs full regen) |
| Cost-aware query routing | Classifies query, estimates cost per strategy, selects optimal |

### Retrieval and Search

| Feature | Details |
|---------|---------|
| Semantic vector search | Cosine similarity on embeddings (OpenAI / FastEmbed / any provider) |
| BM25 keyword search | Full-text search via tantivy |
| Graph traversal search | Scored BFS/DFS from seed entities |
| Hybrid retrieval | Reciprocal Rank Fusion (RRF) combining all three methods |
| Context assembly | Token-budget-aware prompt building with citations |

### Temporal and Data Management

| Feature | Details |
|---------|---------|
| Bi-temporal model | Every fact has `valid_from` and `valid_until` timestamps |
| Automatic fact invalidation | New contradicting facts invalidate old ones |
| Point-in-time queries | `ng.at("2026-03-15")` returns graph state at that moment |
| Entity history | Full chronological fact chain per entity |
| Temporal diff | `ng.what_changed("2026-01", "2026-06")` |
| Graph branching | Copy-on-write branches for hypothetical scenarios |
| Graph merge | 4 strategies: SourceWins, TargetWins, VerifiedOnly, TemporalMerge |
| 2-phase deduplication | Phase 1: embedding similarity + hash. Phase 2: LLM fallback |

### Visualization (Beta)

| Feature | Details |
|---------|---------|
| Interactive dashboard | `await ng.dashboard()` launches browser UI |
| WebGL/Canvas rendering | G6 engine with multi-layer canvas |
| Force-directed layout | Rust WASM for compute-heavy layouts |
| Temporal playback | Scrub through time — nodes appear/disappear as facts change |
| Community clusters | Color-coded G6 Combos |
| Dark/Light mode | Premium dark theme by default |

### Infrastructure

| Feature | Details |
|---------|---------|
| Embedded database (sled) | Default, zero-config persistent storage |
| In-memory mode | petgraph backend for testing |
| Neo4j driver | Connect to existing instances |
| FalkorDB driver | Redis-speed graph queries |
| Kuzu driver | Embedded analytical graph database |
| REST API | Axum-based, async |
| WebSocket | Real-time graph updates |
| Docker | Multi-stage build, non-root, slim image |
| OpenTelemetry | Distributed tracing + metrics |
| Per-operation cost tracking | Model, tokens, cost USD, latency per call |

</details>

---

## Key Concepts

| Concept | What It Does | Why It Matters |
|---------|-------------|----------------|
| **Bi-Temporal Facts** | Every fact has a validity window (`valid_from`, `valid_until`) | Query what was true at any point in time |
| **Graph Branching** | `ng.branch("hypothesis")` creates a copy-on-write branch | Explore what-if scenarios without corrupting real data |
| **Hybrid Retrieval** | Semantic + BM25 + graph traversal, fused with RRF | Better recall than any single search method |
| **Cost-Aware Routing** | Classifies your query and picks the cheapest strategy that meets quality | Predictable LLM spend |
| **Intelligent Forgetting** | Importance = PageRank + access frequency + recency. Low-importance facts decay. | Graph doesn't grow unbounded |
| **Zero API Key Mode** | Regex NER + local FastEmbed + embedded sled | Fully offline, air-gapped, $0 |

---

## Comparison

> Trade-offs are real. NeuroGraph optimizes for temporal reasoning and Rust-native performance. Other tools may be better fits depending on your use case.

| | NeuroGraph | GraphRAG (Microsoft) | Graphiti (Zep) | Mem0 |
|---|---|---|---|---|
| **Best for** | Temporal reasoning, agent memory | Batch document analysis | Episodic memory | Simple key-value memory |
| **Language** | Rust core, Py/TS wrappers | Python | Python | Python |
| **Temporal model** | Bi-temporal | Static | Edge-based time | Recency only |
| **Query approach** | Hybrid (semantic + BM25 + graph) | Map-reduce over communities | Direct retrieval | Vector similarity |
| **Visualization** | Built-in dashboard (Beta) | External (Gephi) | Neo4j Browser | Standard UI |
| **Offline mode** | Yes (regex NER + local embed) | No (requires LLM) | Requires LLM | Requires API |
| **Community detection** | Rust native (Louvain/Leiden) | Python NetworkX | No | No |
| **Maturity** | Early (pre-release) | Production | Production | Production |

---

## Benchmarks

> All numbers from `cargo bench` on an M2 MacBook Pro (16GB) with default embedded config.
> Reproduce locally: `cd benchmarks && cargo bench`. See [`benchmarks/README.md`](./benchmarks/README.md) for methodology.

| Metric | Result | Notes |
|--------|--------|-------|
| **Query latency (P50)** | ~150ms | Hybrid retrieval, embedded sled |
| **Query latency (P99)** | ~500ms | Includes LLM round-trip for answer generation |
| **Community detection (1k nodes)** | <100ms | Native Rust Louvain |
| **Graph layout (10k nodes)** | <200ms | Rust WASM force-directed |
| **Memory baseline** | ~50MB | Empty graph with sled |
| **Cold start** | <2s | Server ready to accept queries |

These numbers reflect current development builds and will change. We plan to add CI-tracked benchmarks via [Bencher](https://bencher.dev) or [GitHub Actions benchmark tracking](https://github.com/benchmark-action/github-action-benchmark).

---

## API at a Glance

| Operation | Python | Rust |
|-----------|--------|------|
| **Create** | `ng = NeuroGraph()` | `let ng = NeuroGraph::builder().build().await?;` |
| **Ingest** | `await ng.add("Alice joined Anthropic")` | `ng.add_text("Alice joined Anthropic").await?;` |
| **Query** | `await ng.query("Where does Alice work?")` | `ng.query("Where does Alice work?").await?;` |
| **Time travel** | `await ng.at("2025-01-01")` | `ng.at("2025-01-01").await?;` |
| **History** | `await ng.history("alice")` | `ng.history("alice").await?;` |
| **Branch** | `await ng.branch("hypothesis")` | `ng.branch("hypothesis").await?;` |
| **Diff** | `ng.diff_branches("main", "hypothesis")` | `ng.diff_branches("main", "hypothesis")?;` |
| **Search** | `await ng.search("Alice")` | `ng.search("Alice").await?;` |
| **Dashboard** | `await ng.dashboard()` | `ng.serve(7777).await?;` |

---

## Integrations

<details>
<summary><b>LLM Providers</b></summary>

| Provider | Models | Local/Cloud |
|----------|--------|-------------|
| OpenAI | GPT-4o, GPT-4o-mini | Cloud |
| Anthropic | Claude 4, Claude 3.5 Sonnet | Cloud |
| Google Gemini | Gemini 2.0 Flash, Gemini Pro | Cloud |
| Ollama | Llama 3, DeepSeek, Mistral, Phi | Local |
| Any OpenAI-compatible | LM Studio, vLLM, Together AI | Local/Cloud |
| **None (offline)** | **Regex NER + rule-based** | **Local** |

</details>

<details>
<summary><b>Storage Backends</b></summary>

| Backend | Type | Setup |
|---------|------|-------|
| **Sled (default)** | **Embedded** | **None** |
| In-Memory (petgraph) | In-process | None |
| Kuzu | Embedded | None |
| Neo4j | Client-server | Docker |
| FalkorDB | Client-server | Docker |

</details>

<details>
<summary><b>Embedding Providers</b></summary>

| Provider | Models | Local/Cloud |
|----------|--------|-------------|
| **FastEmbed (default)** | **bge-small-en-v1.5** | **Local** |
| OpenAI | text-embedding-3-small/large | Cloud |
| Sentence Transformers | Any HuggingFace model | Local |

</details>

<details>
<summary><b>Observability</b></summary>

| Tool | What's Tracked |
|------|---------------|
| OpenTelemetry | Distributed traces per operation |
| Prometheus | Latency, throughput, cache hit metrics |
| Built-in Cost Tracker | Per-query: model, tokens, USD cost, latency |

</details>

---

## Documentation

- [Architecture](docs/architecture.md)
- [Temporal Engine](docs/temporal.md)
- [Community Detection](docs/community.md)
- [Developer Guide](DEVELOPING.md)
- [Contributing](CONTRIBUTING.md)
- [Security Policy](SECURITY.md)
- [Changelog](CHANGELOG.md)

## Roadmap

See the [issue tracker](https://github.com/neurographai/neurograph/issues) for the full roadmap. High-priority items:

- Native Python SDK (PyO3 bindings)
- TypeScript SDK (WASM-powered)
- MCP server stabilization
- CI-tracked performance benchmarks
- Helm chart for Kubernetes
- Distributed graph sharding

## Contributing

We welcome contributions, especially in areas marked **Experimental** or **Planned** above. See [CONTRIBUTING.md](CONTRIBUTING.md).

```bash
git clone https://github.com/neurographai/neurograph.git
cd neurograph
cargo test --workspace
```

## License

[Apache-2.0](LICENSE)

---

<p align="center">
  <b>Built by <a href="https://github.com/Ashutosh0x">Ashutosh Kumar Singh</a></b>
</p>

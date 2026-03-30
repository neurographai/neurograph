# Community Detection

> How NeuroGraph discovers structure in knowledge graphs using Louvain and Leiden algorithms.

---

## Table of Contents

- [Overview](#overview)
- [Why Community Detection Matters for RAG](#why-community-detection-matters-for-rag)
- [Algorithms](#algorithms)
  - [Louvain Algorithm](#louvain-algorithm)
  - [Leiden Algorithm](#leiden-algorithm)
  - [Algorithm Comparison](#algorithm-comparison)
- [Implementation Details](#implementation-details)
  - [LouvainDetector](#louvaindetector)
  - [LeidenDetector](#leidendetector)
  - [IncrementalCommunityUpdater](#incrementalcommunityupdater)
  - [CommunitySummarizer](#communitysummarizer)
- [API Usage](#api-usage)
- [Configuration & Tuning](#configuration--tuning)
- [Community-Powered Queries](#community-powered-queries)
- [Visualization](#visualization)
- [Design Influences](#design-influences)

---

## Overview

Community detection finds clusters of densely connected entities in the knowledge graph. In NeuroGraph, communities serve three critical purposes:

1. **Global query answering** — "What are the main themes in this dataset?" can be answered by summarizing community descriptions rather than scanning every entity.

2. **Context window efficiency** — Instead of stuffing an LLM's context with every entity, we send the relevant community summary + the specific entities within it. This is both cheaper and more effective.

3. **Visual organization** — In the dashboard, communities are rendered as grouped clusters (G6 Combos), making large graphs navigable.

---

## Why Community Detection Matters for RAG

Traditional RAG retrieves document chunks by similarity. This works for local questions ("Where does Alice work?") but fails for global questions ("What are the organizational dynamics in this dataset?").

Community detection solves this by:

```
Knowledge Graph → Community Detection → Community Summaries
                                            ↓
                        Query → Map-Reduce over Community Summaries → Answer
```

**Microsoft's GraphRAG** pioneered this pattern. NeuroGraph extends it with:

- **Pure Rust implementation** (10–50x faster than Python/NetworkX)
- **Incremental updates** (new nodes don't require full recomputation)
- **Diff-based re-summarization** (~30% token cost vs full regeneration)

### Concrete Example

Consider a knowledge graph about AI research companies:

```
[Alice] ──WORKS_AT──► [Anthropic] ◄──DEVELOPS── [Claude]
[Bob]   ──WORKS_AT──► [Anthropic]
[Charlie]──WORKS_AT──► [OpenAI]  ◄──DEVELOPS── [GPT-4]
[David] ──WORKS_AT──► [DeepMind] ◄──DEVELOPS── [Gemini]
```

Community detection groups these into:

| Community | Members | Summary |
|-----------|---------|---------|
| Community 0 | Alice, Bob, Anthropic, Claude | Anthropic is an AI safety company where Alice and Bob work. They develop Claude. |
| Community 1 | Charlie, OpenAI, GPT-4 | OpenAI is an AI research lab where Charlie works. They develop GPT-4. |
| Community 2 | David, DeepMind, Gemini | Google DeepMind is a research division where David works. They develop Gemini. |

Now the query "Compare the main AI research organizations" can be answered using just the 3 community summaries (a few hundred tokens) instead of scanning all 9 entities individually.

---

## Algorithms

### Louvain Algorithm

The Louvain method maximizes **modularity** — a measure of how densely connected nodes within communities are compared to a random graph.

**Two-phase iteration:**

```
Phase 1: Local Moving
├── For each node i:
│   ├── Compute modularity gain for moving to each neighbor's community
│   ├── Move to the community with highest positive gain
│   └── Repeat until no node improves
│
Phase 2: Aggregation (optional)
├── Collapse each community into a super-node
├── Sum edge weights between communities
└── Repeat Phase 1 on the coarsened graph
```

**Modularity gain formula:**

The gain from moving node i to community C:

```
ΔQ = [Σ_in + k_i,in] / 2m  -  [(Σ_tot + k_i) / 2m]²
   - [Σ_in / 2m  -  (Σ_tot / 2m)²  -  (k_i / 2m)²]
```

Where:
- `Σ_in` = sum of weights inside community C
- `Σ_tot` = sum of all weights incident to C
- `k_i` = weighted degree of node i
- `k_i,in` = sum of weights from i to nodes in C
- `m` = total weight of all edges

**Time complexity:** O(n log n) in practice (despite worst-case O(n²))

### Leiden Algorithm

The Leiden algorithm improves upon Louvain by adding a **refinement phase** that ensures all communities are well-connected (no disconnected sub-communities).

```
Phase 1: Local Moving (same as Louvain)
Phase 2: Refinement
├── For each community from Phase 1:
│   ├── Start with each node in its own sub-community
│   ├── For each node, move to sub-community maximizing a modified quality function
│   └── The refined partition guarantees connected communities
Phase 3: Aggregation
├── Collapse refined communities into super-nodes
└── Repeat from Phase 1
```

**Key improvement:** Louvain can produce disconnected communities (nodes assigned to a community they're not connected to). Leiden's refinement phase eliminates this.

**Additional features in NeuroGraph's Leiden:**

| Feature | Description |
|---------|-------------|
| **Resolution parameter** | γ > 1.0 finds more, smaller communities; γ < 1.0 finds fewer, larger ones |
| **Hierarchical detection** | Multiple levels of community structure |
| **Weighted edges** | Uses relationship weights for community assignment |

### Algorithm Comparison

| Property | Louvain | Leiden |
|----------|---------|-------|
| **Quality** | Good | Better (guaranteed connected) |
| **Speed** | Fast | Slightly slower (refinement phase) |
| **Hierarchy** | Multi-level | Multi-level |
| **Disconnected communities?** | Possible | Never |
| **Resolution parameter** | ✅ | ✅ |
| **Use when** | Speed is priority | Quality is priority |

---

## Implementation Details

### LouvainDetector

**Location:** `crates/neurograph-core/src/community/louvain.rs`

```rust
pub struct LouvainDetector {
    config: LouvainConfig,
}

pub struct LouvainConfig {
    pub resolution: f64,          // default: 1.0
    pub min_modularity_gain: f64, // default: 0.0001
    pub max_iterations: usize,    // default: 100
    pub hierarchical: bool,       // default: true
    pub max_levels: u32,          // default: 3
}
```

The detector operates in four steps:

1. **Load graph** — Fetches all entities and relationships from the `GraphDriver`
2. **Build adjacency** — Constructs an internal edge list with weights
3. **Run Louvain** — Iterative local moving until convergence
4. **Store communities** — Creates `Community` objects and stores them via the driver

**Key internal structures:**

```rust
/// Working edge for the Louvain algorithm
struct LouvainEdge {
    source: usize,    // Node index
    target: usize,    // Node index
    weight: f64,      // Relationship weight
}
```

The algorithm maintains:
- `community[i]` — Community assignment for each node
- `community_total[c]` — Total weighted degree of community c
- `strength[i]` — Weighted degree of node i
- `adj[i]` — Adjacency list for node i

**Convergence:** Iteration stops when no node move improves modularity by more than `min_modularity_gain` (default: 0.0001).

### LeidenDetector

**Location:** `crates/neurograph-core/src/community/leiden.rs`

```rust
pub struct LeidenDetector {
    config: LeidenConfig,
}

pub struct LeidenConfig {
    pub resolution: f64,
    pub max_iterations: usize,
    pub max_levels: u32,
    pub refinement_iterations: usize,  // Extra iterations for refinement
}
```

The Leiden implementation extends Louvain with:

1. **Refinement phase** — After local moving, each community is individually refined
2. **Connectivity guarantee** — Communities are checked for internal connectivity
3. **Higher-quality partitions** — Generally produces better modularity scores

### IncrementalCommunityUpdater

**Location:** `crates/neurograph-core/src/community/incremental.rs`

When new entities are ingested, recomputing communities from scratch is wasteful. The incremental updater uses **k-hop delta recomputation**:

```
New entity → Find k-hop neighborhood → Re-run Louvain on subgraph → Update affected communities
```

```rust
pub struct IncrementalCommunityUpdater {
    driver: Arc<dyn GraphDriver>,
}

impl IncrementalCommunityUpdater {
    /// Update communities after new entities are added
    pub async fn update_after_ingestion(
        &self,
        entity_ids: &[EntityId],  // newly added entities
        group_id: Option<&str>,
    ) -> Result<IncrementalUpdateResult, IncrementalError>;
}
```

The result:

```rust
pub struct IncrementalUpdateResult {
    pub entities_evaluated: usize,
    pub communities_created: usize,
    pub communities_updated: usize,
    pub communities_merged: usize,
}
```

**k-hop neighborhood:** Default k=2, meaning we recompute communities for all entities within 2 hops of the newly added entities.

### CommunitySummarizer

**Location:** `crates/neurograph-core/src/community/summarizer.rs`

Generates human-readable summaries for each community:

```rust
pub struct CommunitySummarizer {
    driver: Arc<dyn GraphDriver>,
    llm: Option<Arc<dyn LlmClient>>,
}
```

**Two modes:**

| Mode | When Used | How It Works |
|------|-----------|-------------|
| **LLM summarization** | `OPENAI_API_KEY` set | Sends entity names + relationships to LLM → natural language summary |
| **Rule-based fallback** | No API key | Concatenates entity names and relationship types into a template |

**Map-reduce pattern for large communities:**

```
Community with 100 entities
    ↓
Split into 10 groups of 10
    ↓
Map: Summarize each group (10 LLM calls)
    ↓
Reduce: Summarize the 10 group summaries (1 LLM call)
    ↓
Final community summary
```

**Diff-based re-summarization:**

When a community changes (entity added/removed), the summarizer computes a diff and only re-summarizes the changed portions. This saves ~30% token cost compared to full regeneration.

---

## API Usage

### Python

```python
from neurograph import NeuroGraph, LouvainConfig

ng = NeuroGraph()

# Add data
await ng.add("Alice works at Anthropic")
await ng.add("Bob works at Anthropic")
await ng.add("Anthropic develops Claude")
await ng.add("Charlie works at OpenAI")
await ng.add("OpenAI develops GPT-4")

# Detect communities (default Louvain)
result = await ng.detect_communities()
print(f"Found {len(result.communities)} communities")
print(f"Modularity: {result.modularity:.4f}")

for community in result.communities:
    print(f"  {community.name}: {len(community.member_ids)} members")

# Custom configuration
config = LouvainConfig(
    resolution=1.5,       # More, smaller communities
    max_iterations=200,
    hierarchical=True,
    max_levels=4,
)
result = await ng.detect_communities_with(config)

# Summarize communities (uses LLM if available)
summaries = await ng.summarize_communities()
for summary in summaries:
    print(f"Community: {summary.community_name}")
    print(f"Summary: {summary.summary}")

# Incremental update after new data
episode = await ng.add("Eve works at Anthropic")
await ng.update_communities(episode.entity_ids)
```

### Rust

```rust
use neurograph_core::{NeuroGraph, LouvainConfig};

let ng = NeuroGraph::builder().build().await?;

// Detect communities
let result = ng.detect_communities().await?;
println!("Communities: {}", result.communities.len());
println!("Modularity: {:.4}", result.modularity);

// Custom config
let config = LouvainConfig {
    resolution: 1.5,
    max_iterations: 200,
    hierarchical: true,
    max_levels: 4,
    ..Default::default()
};
let result = ng.detect_communities_with(config).await?;

// Summarize
let summaries = ng.summarize_communities().await?;
for s in &summaries {
    println!("{}: {}", s.community_name, s.summary);
}
```

---

## Configuration & Tuning

### Resolution Parameter

The resolution parameter γ controls the granularity of communities:

| γ Value | Effect | Use Case |
|---------|--------|----------|
| 0.5 | Large, coarse communities | High-level theme detection |
| 1.0 (default) | Standard modularity | General-purpose |
| 1.5 | Smaller, fine-grained communities | Detailed relationship mapping |
| 2.0+ | Very small communities | Clique detection |

**Rule of thumb:**
- Start with γ = 1.0
- If communities are too broad (100+ members), increase γ
- If communities are too small (1-3 members), decrease γ

### Iteration Limits

| Parameter | Default | Impact |
|-----------|---------|--------|
| `max_iterations` | 100 | Higher = better quality, slower |
| `min_modularity_gain` | 0.0001 | Lower = more iterations, marginally better |
| `max_levels` | 3 | More levels = deeper hierarchy |

### Performance Benchmarks

On an M2 MacBook Pro (16GB):

| Graph Size | Louvain Time | Leiden Time | Communities Found |
|------------|-------------|-------------|-------------------|
| 100 nodes | <5ms | <10ms | ~5-10 |
| 1,000 nodes | <100ms | <200ms | ~20-50 |
| 10,000 nodes | ~1s | ~2s | ~100-500 |
| 100,000 nodes | ~15s | ~30s | ~500-2000 |

These numbers are for the pure Rust implementation. For comparison, Python NetworkX on the same graphs is typically 10–50x slower.

---

## Community-Powered Queries

When a query is classified as "global" by the `QueryRouter`, the execution flow changes:

```
Global Query: "What are the main research themes?"
    ↓
1. Load all community summaries
    ↓
2. Map: Score each summary for relevance to the query
    ↓
3. Reduce: Combine top-k relevant summaries
    ↓
4. LLM: Generate answer from combined summaries
    ↓
Answer: "The main themes are: AI safety (Community 0: Anthropic, Claude),
         large language models (Community 1: OpenAI, GPT-4), and
         multimodal AI (Community 2: DeepMind, Gemini)."
```

This is dramatically more efficient than scanning every entity:

| Approach | Tokens | Cost | Quality |
|----------|--------|------|---------|
| Scan all entities | ~10,000 | ~$0.01 | Medium (no structure) |
| Community summaries | ~500 | ~$0.001 | High (structured themes) |

---

## Visualization

In the dashboard, communities are visualized using **G6 Combos**:

- Each community becomes a Combo (bounding group)
- Member entities are positioned inside the Combo
- Combo colors are automatically assigned per community
- Cross-community edges are drawn between Combos
- Clicking a Combo expands/collapses its members

The visualization data is generated by combining:
1. Community assignments from the detector
2. Entity positions from the force-directed layout
3. Community summaries from the summarizer

```json
{
  "combos": [
    {
      "id": "community_0",
      "label": "Anthropic Cluster",
      "style": { "fill": "#6C5CE7", "opacity": 0.3 }
    }
  ],
  "nodes": [
    {
      "id": "alice",
      "label": "Alice",
      "comboId": "community_0"
    }
  ]
}
```

---

## Design Influences

| Concept | Source | NeuroGraph Enhancement |
|---------|--------|----------------------|
| Hierarchical Leiden | GraphRAG (`hierarchical_leiden.py`) | Pure Rust implementation (10-50x speedup) |
| Community summarization | GraphRAG (map-reduce) | Diff-based re-summarization (~30% cheaper) |
| Label propagation | Graphiti (Zep) | Replaced with Louvain/Leiden for higher quality |
| Incremental updates | Original | k-hop delta recomputation |
| Community visualization | Original | G6 Combos integration |

### Why Not NetworkX?

GraphRAG originally used Python's NetworkX for Leiden. This has several drawbacks:
1. **Speed** — NetworkX is pure Python, 10-50x slower than native Rust
2. **Memory** — Python objects have significant overhead per node
3. **Integration** — Requires Python runtime even for Rust deployments
4. **Incremental** — NetworkX doesn't support incremental updates natively

NeuroGraph's pure Rust implementation addresses all four issues.

---

*For the temporal engine, see [Temporal Engine](temporal.md). For the full architecture, see [Architecture](architecture.md).*

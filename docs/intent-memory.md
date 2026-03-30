# Intent Router & Tiered Memory

> Deep-dive into NeuroGraph's intent-aware query routing and multi-tier memory system.

---

## Table of Contents

- [Intent Router](#intent-router)
  - [Architecture](#architecture)
  - [Query Classification](#query-classification)
  - [Multi-Hop Planning](#multi-hop-planning)
  - [Sub-Graph Dispatch](#sub-graph-dispatch)
- [Tiered Memory](#tiered-memory)
  - [Memory Layers](#memory-layers)
  - [Episode Grouping](#episode-grouping)
  - [Learned Rules (Procedural)](#learned-rules-procedural)
  - [Context Assembly](#context-assembly)
- [Multi-Strategy Graph Engine](#multi-strategy-graph-engine)

---

## Intent Router

**Location:** `crates/neurograph-core/src/multigraph/intent.rs`

The `IntentRouter` classifies incoming queries by intent type and routes them to the most appropriate sub-graph layer for optimal retrieval.

### Architecture

```
Query → IntentRouter.classify() → IntentType
                ↓
        IntentRouter.plan() → Vec<HopStep>
                ↓
        IntentRouter.route() → SubGraphResult
```

### Query Classification

The router classifies queries into four intent types:

| Intent Type | Signal Keywords | Example Query |
|-------------|----------------|---------------|
| **Semantic** | "what", "describe", "explain", "summarize" | "What does Alice work on?" |
| **Temporal** | "when", "before", "after", "during", "timeline" | "When did Bob join OpenAI?" |
| **Causal** | "why", "because", "caused", "led to", "result" | "Why did Alice leave Google?" |
| **Entity** | "who", "where", names, organizations | "Who is the CEO of Anthropic?" |

Classification uses keyword-based scoring with TF weighting. Each intent type has a confidence score (0.0–1.0), and the highest-confidence type is selected.

```rust
let intent = router.classify("When did Alice join Anthropic?");
// IntentClassification {
//     intent_type: IntentType::Temporal,
//     confidence: 0.85,
//     query: "When did Alice join Anthropic?"
// }
```

### Multi-Hop Planning

For complex queries that require traversing multiple sub-graphs, the `plan()` method decomposes the query into sequential hop steps:

```rust
let plan = router.plan("Why did Bob leave Google before joining OpenAI?");
// vec![
//     HopStep { layer: Entity, query: "Bob", confidence: 0.9 },
//     HopStep { layer: Temporal, query: "leave Google", confidence: 0.8 },
//     HopStep { layer: Causal, query: "why ... before joining OpenAI", confidence: 0.75 },
// ]
```

Each `HopStep` contains:
- `layer` — which sub-graph to query
- `query` — the sub-query text
- `confidence` — expected relevance (0.0–1.0)

### Sub-Graph Dispatch

The `route()` method executes the classified query against the `MultiStrategyGraph`:

```rust
let result = router.route(
    &classification,
    &multi_graph,
    Some(10),  // top-k results
);
```

This delegates to the appropriate sub-graph layer (entity, semantic, temporal, or causal) and returns scored results.

---

## Tiered Memory

**Location:** `crates/neurograph-core/src/memory/tiered.rs`

NeuroGraph implements a biologically-inspired 4-tier memory system that mirrors human cognitive architecture.

### Memory Layers

```
┌──────────────────────────────────────┐
│  L1 — Working Memory (short-term)    │  ← Active context, fast decay
├──────────────────────────────────────┤
│  L2 — Episodic Memory               │  ← Grouped episodes, temporal bounds
├──────────────────────────────────────┤
│  L3 — Semantic Memory                │  ← Long-term facts, slow decay
├──────────────────────────────────────┤
│  L4 — Procedural Memory             │  ← Learned rules, persistent
└──────────────────────────────────────┘
```

| Tier | Purpose | Decay Rate | Example Content |
|------|---------|------------|-----------------|
| **L1 Working** | Active conversation context | Fast (minutes–hours) | Current query entities, recent mentions |
| **L2 Episodic** | Grouped interaction episodes | Medium (hours–days) | "User asked about Alice's career on March 15" |
| **L3 Semantic** | Long-term factual knowledge | Slow (days–weeks) | "Alice works at Anthropic" |
| **L4 Procedural** | Learned rules and patterns | None (permanent) | "When user asks about careers, check employment history first" |

### Episode Grouping

Episodes group related facts with temporal bounds:

```rust
let episode = Episode {
    id: "ep-001".to_string(),
    name: "Alice career discussion".to_string(),
    facts: vec!["Alice joined Anthropic", "Alice left Google"],
    start_time: Utc::now() - Duration::hours(1),
    end_time: Utc::now(),
};

memory.add_episode(episode);
```

Episodes are automatically created during ingestion and can be queried by time range or topic.

### Learned Rules (Procedural)

The procedural tier stores reusable patterns discovered during interaction:

```rust
let rule = LearnedRule {
    id: "rule-001".to_string(),
    pattern: "employment_query".to_string(),
    action: "Check entity relationships with type WORKS_AT".to_string(),
    confidence: 0.92,
    source_episodes: vec!["ep-001".to_string()],
};

memory.add_learned_rule(rule);
```

Learned rules are consulted during query planning to improve routing accuracy over time.

### Context Assembly

The `get_context_for_query()` method assembles relevant context across all memory tiers:

```rust
let context = memory.get_context_for_query("Where does Alice work?", 5);
// MemoryContext {
//     working: [...],     // Recent mentions of Alice
//     episodic: [...],    // Past episodes about Alice
//     semantic: [...],    // Long-term facts about Alice's employment
//     procedural: [...],  // Rules about answering employment queries
// }
```

The method:
1. Searches all four tiers in parallel
2. Scores results by relevance and recency
3. Applies tier-specific decay functions
4. Returns a unified `MemoryContext` with results from each tier

---

## Multi-Strategy Graph Engine

**Location:** `crates/neurograph-core/src/multigraph/`

The `MultiStrategyGraph` maintains multiple specialized sub-graphs that work together:

```
multigraph/
├── mod.rs          # MultiStrategyGraph orchestrator
├── entity.rs       # Entity sub-graph (nodes = entities, edges = relationships)
├── semantic.rs     # Semantic sub-graph (edges weighted by embedding similarity)
├── temporal.rs     # Temporal sub-graph (time-ordered edges with validity windows)
├── causal.rs       # Causal sub-graph (cause-effect relationships)
├── fusion.rs       # Cross-layer result fusion
└── intent.rs       # IntentRouter (query classification + dispatch)
```

### Sub-Graph Types

| Sub-Graph | Node Type | Edge Weight | Best For |
|-----------|-----------|-------------|----------|
| **Entity** | Named entities | Relationship strength | "Who/What" queries |
| **Semantic** | Concepts + entities | Embedding similarity | "Explain/Describe" queries |
| **Temporal** | Time-stamped facts | Temporal proximity | "When/Before/After" queries |
| **Causal** | Events + causes | Causal confidence | "Why/Because" queries |

### Cross-Layer Fusion

The `AdaptiveFusion` engine combines results from multiple sub-graphs using weighted scoring:

```rust
let fused = fusion.fuse(vec![
    (entity_results, 0.4),
    (semantic_results, 0.3),
    (temporal_results, 0.2),
    (causal_results, 0.1),
]);
```

Weights are dynamically adjusted based on:
- Query intent classification confidence
- Sub-graph density (sparse graphs get lower weight)
- Historical retrieval accuracy per tier

---

*For core architecture, see [Architecture](architecture.md). For temporal details, see [Temporal Engine](temporal.md).*

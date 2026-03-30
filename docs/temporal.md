# Temporal Engine

> How NeuroGraph treats time as a first-class dimension in the knowledge graph.

---

## Table of Contents

- [Overview](#overview)
- [Bi-Temporal Model](#bi-temporal-model)
  - [Valid Time](#valid-time)
  - [Transaction Time](#transaction-time)
  - [Four-Timestamp Pattern](#four-timestamp-pattern)
- [Core Components](#core-components)
  - [TemporalManager](#temporalmanager)
  - [FactVersionChain](#factversionchain)
  - [EntityHistory](#entityhistory)
  - [LogicalClock](#logicalclock)
  - [ForgettingEngine](#forgettingengine)
- [Temporal Operations](#temporal-operations)
  - [Point-in-Time Snapshots](#point-in-time-snapshots)
  - [Temporal Diffing](#temporal-diffing)
  - [Timeline Construction](#timeline-construction)
  - [Date Parsing](#date-parsing)
- [Time Travel API](#time-travel-api)
- [Intelligent Forgetting](#intelligent-forgetting)
  - [Importance Scoring](#importance-scoring)
  - [Decay Algorithm](#decay-algorithm)
  - [TTL-Based Expiration](#ttl-based-expiration)
  - [Prune Candidates](#prune-candidates)
- [Temporal Conflict Resolution](#temporal-conflict-resolution)
- [Graph Branching](#graph-branching)
- [Visualization Integration](#visualization-integration)
- [Design Influences](#design-influences)

---

## Overview

Most knowledge graph systems treat facts as static — once stored, they're either present or absent. NeuroGraph rejects this. In reality, facts change over time:

- People change jobs
- Companies get acquired
- Research results supersede older findings
- Relationships form and dissolve

NeuroGraph's temporal engine ensures that **every fact carries its own timeline**. You can:

1. **Query the present** — Get the most current state of the graph
2. **Time travel** — See what was true on any past date
3. **Track history** — Get the full chronological chain for any entity
4. **Diff time windows** — See exactly what changed between two dates
5. **Forget intelligently** — Let unimportant facts fade naturally

---

## Bi-Temporal Model

NeuroGraph implements a **bi-temporal** data model, meaning every fact is tagged with two independent time dimensions.

### Valid Time

**What was true in reality?**

Every relationship has `valid_from` and `valid_until`:

```
Relationship: "Alice works at Google"
├── valid_from:  2021-03-15   (she started at Google)
└── valid_until: 2025-06-01   (she left Google)

Relationship: "Alice works at Anthropic"
├── valid_from:  2025-06-01   (she joined Anthropic)
└── valid_until: None          (she still works there)
```

Valid time answers questions like: *"Where did Alice work in 2023?"* → Google.

### Transaction Time

**When did we learn this?**

Every entity and relationship has `created_at` and `expired_at`:

```
Relationship: "Alice works at Google"
├── created_at:  2025-01-01   (we recorded this on Jan 1, 2025)
└── expired_at:  2025-07-15   (we learned it was outdated on Jul 15)
```

Transaction time answers questions like: *"What did our knowledge base contain on Jan 10?"* — It would include "Alice works at Google" since we hadn't yet learned she'd left.

### Four-Timestamp Pattern

Each relationship carries four timestamps:

| Timestamp | Dimension | Meaning |
|-----------|-----------|---------|
| `valid_from` | Valid time | When the fact became true in reality |
| `valid_until` | Valid time | When the fact stopped being true (None = still valid) |
| `created_at` | Transaction time | When we recorded this fact |
| `expired_at` | Transaction time | When we invalidated this record (None = current) |

This four-timestamp pattern is the foundation of the bi-temporal model, inspired by Graphiti's approach but enhanced with additional temporal operations.

---

## Core Components

### TemporalManager

**Location:** `crates/neurograph-core/src/temporal/manager.rs`

The central orchestrator for all temporal operations. It wraps a `GraphDriver` and provides:

```rust
pub struct TemporalManager {
    driver: Arc<dyn GraphDriver>,
}

impl TemporalManager {
    /// Point-in-time snapshot
    pub async fn snapshot_at(&self, timestamp: DateTime<Utc>, group_id: Option<&str>)
        -> Result<TemporalSnapshot, TemporalError>;

    /// What changed between two times
    pub async fn what_changed(&self, from: DateTime<Utc>, to: DateTime<Utc>, ...)
        -> Result<TemporalDiff, TemporalError>;

    /// Build visualization timeline
    pub async fn build_timeline(&self, group_id: Option<&str>)
        -> Result<Vec<TimelineEvent>, TemporalError>;

    /// Parse date strings (supports multiple formats)
    pub fn parse_date(date_str: &str) -> Result<DateTime<Utc>, TemporalError>;
}
```

**`TemporalSnapshot`** contains the full state of the graph at a given moment:

```rust
pub struct TemporalSnapshot {
    pub timestamp: DateTime<Utc>,
    pub entities: Vec<Entity>,         // Entities that existed at this time
    pub relationships: Vec<Relationship>, // Relationships valid at this time
    pub entity_count: usize,
    pub relationship_count: usize,
}
```

### FactVersionChain

**Location:** `crates/neurograph-core/src/temporal/versioning.rs`

Tracks how a specific fact evolves over time. When "Alice works at Google" is superseded by "Alice works at Anthropic", both versions are linked:

```rust
pub struct FactVersionChain {
    pub source_entity_id: EntityId,
    pub target_entity_id: EntityId,
    pub relationship_type: String,
    pub versions: Vec<FactVersion>,  // Ordered chronologically
}
```

Each `FactVersion` contains:

```rust
pub struct FactVersion {
    pub relationship_id: RelationshipId,
    pub version: u32,              // 1 = original, 2 = first update, etc.
    pub fact: String,
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub superseded_at: Option<DateTime<Utc>>,
    pub superseded_by: Option<RelationshipId>,
    pub confidence: f64,
}
```

**Key operations:**

| Method | Purpose |
|--------|---------|
| `current()` | Get the latest valid version |
| `version_at(timestamp)` | Get the version that was true at a specific time |
| `version_count()` | Total number of versions |
| `add_version(v)` | Append a new version (auto-sorts chronologically) |

**Example:**

```
FactVersionChain: Alice → WORKS_AT
├── v1: "Alice works at MIT"        (2018-09-01 → 2021-03-15)
├── v2: "Alice works at Google"     (2021-03-15 → 2025-06-01)
└── v3: "Alice works at Anthropic"  (2025-06-01 → None) ← current
```

Calling `chain.version_at("2023-01-01")` returns v2 ("Alice works at Google").

### EntityHistory

Tracks all modifications to an entity over time:

```rust
pub struct EntityHistory {
    pub entity_id: EntityId,
    pub entries: Vec<EntityHistoryEntry>,
}

pub struct EntityHistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub change_type: EntityChangeType,
    pub description: String,
    pub old_summary: Option<String>,
    pub new_summary: Option<String>,
}
```

Change types:

| Change Type | Meaning |
|-------------|---------|
| `Created` | Entity was first added |
| `SummaryUpdated` | Entity's summary text changed |
| `Merged` | Entity was merged with a duplicate (dedup) |
| `TypeChanged` | Entity type was reclassified |
| `AttributesUpdated` | Entity attributes were modified |
| `DecayMarked` | Entity was flagged by the forgetting engine |

### LogicalClock

**Location:** `crates/neurograph-core/src/temporal/clock.rs`

A hybrid logical clock (HLC) that combines wall-clock time with a logical counter for strict event ordering:

```rust
pub struct LogicalClock {
    pub wall: DateTime<Utc>,  // Physical time
    pub logical: u64,         // Monotonically increasing counter
}
```

The HLC ensures that:
- Events on the same node are strictly ordered
- Events across nodes respect causality  
- No two events have the same clock value

This is critical for distributed scenarios where multiple agents might ingest facts simultaneously.

### ForgettingEngine

**Location:** `crates/neurograph-core/src/temporal/forgetting.rs`

Manages knowledge decay and graph pruning. See [Intelligent Forgetting](#intelligent-forgetting) below.

---

## Temporal Operations

### Point-in-Time Snapshots

The most fundamental temporal operation: *"What did the graph look like at timestamp T?"*

```rust
let temporal_mgr = TemporalManager::new(driver.clone());
let snapshot = temporal_mgr.snapshot_at(timestamp, None).await?;

// snapshot.entities = entities that existed at `timestamp`
// snapshot.relationships = relationships valid at `timestamp`
```

The snapshot uses the bi-temporal model to filter:
- Entity: `created_at <= timestamp`
- Relationship: `valid_from <= timestamp < valid_until` AND `expired_at IS NULL OR expired_at > timestamp`

### Temporal Diffing

*"What changed between January and June?"*

```rust
let diff = temporal_mgr.what_changed(from_ts, to_ts, None).await?;

// diff.added_entities = entities created in [from, to]
// diff.modified_entities = entities updated in [from, to]
// diff.invalidated_relationships = relationships that expired in [from, to]
```

The `TemporalDiff` struct:

```rust
pub struct TemporalDiff {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub added_entities: Vec<Entity>,
    pub modified_entities: Vec<Entity>,
    pub invalidated_relationships: Vec<Relationship>,
}
```

### Timeline Construction

Generate a sequence of events for the G6 Timebar visualization:

```rust
let timeline = temporal_mgr.build_timeline(None).await?;
// Returns: Vec<TimelineEvent>
```

Each event:

```rust
pub struct TimelineEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: TimelineEventType, // Ingestion | Update | Contradiction | Snapshot
    pub description: String,            // "Added 5 entities: Alice, Bob, ..."
    pub entity_count: usize,
    pub relationship_count: usize,
}
```

Events are grouped by date and sorted chronologically. The dashboard renders them on a horizontal timeline slider.

### Date Parsing

The `parse_date` function accepts multiple formats for developer convenience:

| Input | Parsed As | Format |
|-------|-----------|--------|
| `"2025-01-15"` | 2025-01-15 00:00:00 UTC | ISO date |
| `"2025-01-15T10:30:00Z"` | 2025-01-15 10:30:00 UTC | ISO datetime |
| `"2025"` | 2025-01-01 00:00:00 UTC | Year only |
| `"January 15, 2025"` | 2025-01-15 00:00:00 UTC | Long month |
| `"Jan 15, 2025"` | 2025-01-15 00:00:00 UTC | Short month |
| `"2025/01/15"` | 2025-01-15 00:00:00 UTC | Slash format |
| `"01/15/2025"` | 2025-01-15 00:00:00 UTC | US format |

---

## Time Travel API

The public API exposes temporal features through a clean interface:

```python
# Python SDK
ng = NeuroGraph()

# Add facts with explicit timestamps
await ng.add_text_at("Alice works at Google", "2023-01-15")
await ng.add_text_at("Alice works at Anthropic", "2025-06-15")

# Time travel — returns a TemporalView
past = await ng.at("2024-01-01")
result = await past.query("Where does Alice work?")
# → "Google"

# Entity history — full chronological chain
history = await ng.entity_history("Alice")
for rel in history:
    print(f"{rel.fact} ({rel.valid_from} → {rel.valid_until})")
# Alice works at Google (2023-01-15 → 2025-06-15)
# Alice works at Anthropic (2025-06-15 → None)

# What changed
diff = await ng.what_changed("2025-01", "2025-07")
print(f"Added: {len(diff.added_entities)} entities")
print(f"Invalidated: {len(diff.invalidated_relationships)} relationships")
```

```rust
// Rust API
let ng = NeuroGraph::builder().build().await?;

// Time travel
let past = ng.at("2024-01-01").await?;
println!("Entities at that time: {}", past.entity_count());

// History
let rels = ng.entity_history("Alice").await?;
for rel in &rels {
    println!("{} (valid: {} → {:?})", rel.fact, rel.valid_from, rel.valid_until);
}

// Timeline for visualization
let timeline = ng.build_timeline().await?;
```

---

## Intelligent Forgetting

Knowledge graphs grow unbounded unless managed. NeuroGraph's forgetting engine implements biologically-inspired memory decay.

### Importance Scoring

Each entity's importance is a composite score:

```
importance = (base_importance × time_decay) + access_boost + connectivity_boost
```

| Component | Formula | Intuition |
|-----------|---------|-----------|
| **Time decay** | `e^(-decay_rate × days_since_update)` | Exponential decay from last access |
| **Access boost** | `ln(access_count + 1) / 10` | Frequently accessed facts survive |
| **Connectivity boost** | `ln(relationship_count + 1) / 10` | Well-connected facts survive |

### Decay Algorithm

```rust
pub fn calculate_importance(&self, entity: &Entity, relationship_count: usize) -> f64 {
    let days_since_update = (now - entity.updated_at).num_days().max(0) as f64;

    // Exponential decay based on time since last access
    let time_decay = (-self.config.daily_decay_rate * days_since_update).exp();

    // Access frequency boost (logarithmic scale)
    let access_boost = (entity.access_count as f64 + 1.0).ln() / 10.0;

    // Connectivity boost
    let connectivity_boost = (relationship_count as f64 + 1.0).ln() / 10.0;

    // Combine and clamp to [0.0, 1.0]
    let raw_score = entity.importance_score * time_decay + access_boost + connectivity_boost;
    raw_score.clamp(0.0, 1.0)
}
```

### Decay Configuration

```rust
pub struct ForgettingConfig {
    pub enabled: bool,              // default: false
    pub default_ttl: Option<Duration>, // Hard TTL (None = never expire)
    pub importance_threshold: f64,  // Prune below this (default: 0.1)
    pub min_access_count: u64,      // Keep if accessed ≥ N times (default: 5)
    pub daily_decay_rate: f64,      // Decay rate per day (default: 0.01)
    pub max_prune_batch: usize,     // Max entities to prune per pass (default: 100)
}
```

### TTL-Based Expiration

For time-sensitive facts (e.g., event schedules), a hard TTL can be set:

```rust
let config = ForgettingConfig {
    enabled: true,
    default_ttl: Some(Duration::days(90)), // Facts expire after 90 days
    ..Default::default()
};
```

The `is_expired()` check:

```rust
pub fn is_expired(&self, entity: &Entity) -> bool {
    if let Some(ttl) = self.config.default_ttl {
        let age = Utc::now() - entity.created_at;
        age > ttl
    } else {
        false
    }
}
```

### Prune Candidates

The decay pass identifies candidates for pruning:

```rust
let result = engine.decay_pass(auto_prune: true, group_id: None).await?;
// ForgettingResult {
//     scores_updated: 150,    // Entities whose importance was recalculated
//     prune_candidates: 12,   // Entities below threshold
//     pruned: 12,             // Actually deleted (if auto_prune=true)
//     total_evaluated: 500,   // Total entities scanned
// }
```

An entity is a prune candidate when:
1. `importance_score < importance_threshold` (default: 0.1)
2. `access_count < min_access_count` (default: 5)

Both conditions must be true — a frequently accessed low-importance entity is kept, as is a high-importance rarely-accessed entity.

---

## Temporal Conflict Resolution

When a new fact contradicts an existing one:

```
Existing: "Alice works at Google"  (valid_from: 2021-03-15, valid_until: None)
New:      "Alice works at Anthropic" (valid_from: 2025-06-01)
```

The conflict resolver:

1. **Detects the contradiction** — Same source entity, same relationship type, different target
2. **Closes the old fact** — Sets `valid_until = 2025-06-01` on the Google relationship
3. **Creates the new fact** — Stores the Anthropic relationship with `valid_from = 2025-06-01`
4. **Links the chain** — Sets `superseded_by` on the old fact pointing to the new one

After resolution:

```
"Alice works at Google"    (valid_from: 2021-03-15, valid_until: 2025-06-01) — superseded
"Alice works at Anthropic" (valid_from: 2025-06-01, valid_until: None)      — current
```

No data is ever deleted — it's marked as superseded, preserving the complete history.

---

## Graph Branching

NeuroGraph supports **copy-on-write branching** for hypothetical scenarios:

```python
# Create a branch
await ng.branch("what-if-acquisition")

# Make changes in the branch
await ng.add("Google acquires Anthropic")
await ng.add("Alice becomes a Google employee")

# Compare
diff = ng.diff_branches("main", "what-if-acquisition")
# Shows which entities/relationships differ

# Merge if desired (4 strategies)
await ng.merge("what-if-acquisition", strategy="SourceWins")
```

Merge strategies:

| Strategy | Behavior |
|----------|----------|
| `SourceWins` | Branch changes overwrite main |
| `TargetWins` | Main changes take priority, branch changes ignored |
| `VerifiedOnly` | Only merge branch changes with confidence > threshold |
| `TemporalMerge` | Merge based on valid_from timestamps (latest wins) |

---

## Visualization Integration

The temporal engine provides data formatted for the **G6 Timebar** visualization component:

```
Timeline events → JSON → WebSocket → Dashboard → G6 Timebar
```

The temporal playback feature works by:

1. Building the timeline: `build_timeline()` → `Vec<TimelineEvent>`
2. Serializing to JSON (all `TimelineEvent` fields are `Serialize`)
3. Sending to the dashboard via WebSocket
4. The dashboard's timeline slider calls `snapshot_at(t)` for each position
5. G6 dynamically adds/removes nodes and edges based on the snapshot

This creates an animation effect where entities appear and disappear as you scrub through time.

---

## Design Influences

| Concept | Source | NeuroGraph Enhancement |
|---------|--------|----------------------|
| Bi-temporal model | Graphiti (Zep) | Enhanced with `FactVersionChain` for full history navigation |
| Memory consolidation | Mem0 | Extended with graph-aware scoring (connectivity + PageRank) |
| Fact invalidation | Graphiti | Added `superseded_by` linking for version chain traversal |
| Temporal playback | Original | G6 Timebar integration for visual time scrubbing |
| Intelligent forgetting | Original | Composite scoring: time decay × access frequency × connectivity |
| HLC clock | CockroachDB | Adapted for knowledge graph event ordering |

---

*For the full architecture overview, see [Architecture](architecture.md). For community detection, see [Community Detection](community.md).*

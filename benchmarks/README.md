# NeuroGraph Benchmark Suite

> Proving every claim with numbers.

## Overview

NeuroGraph ships with a comprehensive benchmarking system covering **5 dimensions**:

| Layer | What It Measures | Tool | Status |
|-------|-----------------|------|--------|
| **Correctness** | Do operations produce right results? | `cargo test` | ✅ Active |
| **Performance** | How fast is each operation? | Criterion | ✅ Active |
| **Temporal** | Does time-travel actually work? | Custom suite | ✅ Active |
| **Graph Engine** | How does the storage layer scale? | Criterion | ✅ Active |
| **Cost** | How much does each operation cost? | Built-in tracker | ✅ Active |

## Running Benchmarks

### Quick: All Tests
```bash
cargo test --workspace
```

### Performance Benchmarks (Criterion)
```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench -- engine_benchmarks
cargo bench -- search_benchmarks
cargo bench -- community_benchmarks
cargo bench -- temporal_benchmarks

# Generate HTML reports (opens in browser)
cargo bench -- --output-format bencher
# Reports saved to: target/criterion/
```

### Run a Specific Test Suite
```bash
cargo test test_temporal_scenarios
cargo test test_community_correctness
cargo test test_search_correctness
cargo test test_cost_tracking
```

## Benchmark Groups

### Engine Benchmarks (`engine_benchmarks.rs`)
| Benchmark | What It Measures |
|-----------|-----------------|
| `entity_storage/store_entity/{N}` | Raw entity storage throughput at 10-5K entities |
| `relationship_storage/store_relationship/{N}` | Relationship storage throughput |
| `ingestion/add_text/{N}` | Full pipeline throughput (extract → dedup → store) |
| `query_latency/simple_query/{N}` | Query latency at different graph sizes |
| `initialization/builder_memory` | NeuroGraph initialization time |

### Search Benchmarks (`search_benchmarks.rs`)
| Benchmark | What It Measures |
|-----------|-----------------|
| `vector_search/cosine_top10/{N}` | Cosine similarity search at 100-5K entities |
| `text_search/keyword_top10/{N}` | BM25-style keyword search |
| `graph_traversal/bfs_depth{D}/{N}` | BFS traversal at depth 1-3 on 100-1K nodes |

### Community Benchmarks (`community_benchmarks.rs`)
| Benchmark | What It Measures |
|-----------|-----------------|
| `louvain_detection/detect/{N}n_{C}c` | Louvain on synthetic clique graphs (50-1000 nodes) |
| `louvain_resolution/resolution/{R}` | Effect of resolution parameter on 200-node graph |

### Temporal Benchmarks (`temporal_benchmarks.rs`)
| Benchmark | What It Measures |
|-----------|-----------------|
| `temporal_snapshot/snapshot_at_midpoint/{N}` | Point-in-time snapshot reconstruction |
| `temporal_snapshot/snapshot_at_recent/{N}` | Recent snapshot (best-case) |
| `temporal_diff/what_changed/{N}` | Diff computation over time ranges |
| `temporal_timeline/build_timeline/{N}` | Timeline event generation |
| `date_parsing/parse/{format}` | Date string parsing throughput |

## Methodology

### Hardware
All benchmarks should be run on consistent hardware. Recommended baseline:
- Apple M-series or equivalent x86-64
- 16GB+ RAM
- SSD storage

### Measurement
- **Criterion** uses statistical analysis: reports mean, median, standard deviation
- Each benchmark runs enough iterations for statistical significance
- Results include confidence intervals
- Regression detection: CI fails if performance degrades >20%

### Synthetic Data
- **Clique graphs**: Known community structure for validating detection accuracy
- **Temporal graphs**: Entities spread across time for snapshot benchmarks
- **Embedding vectors**: Deterministic vectors for reproducible search results

## CI Integration

Performance benchmarks run on every PR via `.github/workflows/bench.yml`:
- Results compared against `main` branch baseline
- PRs blocked if performance regresses by >20%
- Historical results tracked in `benchmarks/results/`

## Test Coverage

| Test Suite | Tests | What It Covers |
|-----------|-------|---------------|
| `test_neurograph` | 12 | Public API integration |
| `test_drivers` | ~20 | Memory + sled driver CRUD |
| `test_ingestion` | ~21 | Pipeline extraction + dedup |
| `test_retrieval` | ~13 | Search and query strategies |
| `test_temporal_scenarios` | 30+ | Snapshots, diffs, parsing, forgetting |
| `test_community_correctness` | 12+ | Louvain accuracy and edge cases |
| `test_search_correctness` | 15+ | Vector, text, hybrid, traversal |
| `test_cost_tracking` | 7 | Cost accumulation and budget |
| Inline module tests | ~60 | Per-module unit tests |
| **Total** | **250+** | |

## Adding New Benchmarks

1. Create a new benchmark file in `crates/neurograph-core/benches/`
2. Add a `[[bench]]` entry to `Cargo.toml`
3. Use `criterion_group!` and `criterion_main!` macros
4. Use `b.to_async(&rt)` for async benchmarks with Tokio
5. Use `black_box()` to prevent compiler optimization

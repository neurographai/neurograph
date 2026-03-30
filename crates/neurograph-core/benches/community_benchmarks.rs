// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Community detection benchmarks: Louvain on synthetic graphs at various scales.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use neurograph_core::community::louvain::{LouvainConfig, LouvainDetector};
use neurograph_core::drivers::memory::MemoryDriver;
use neurograph_core::drivers::traits::GraphDriver;
use neurograph_core::graph::{Entity, Relationship};
use tokio::runtime::Runtime;

/// Generate a graph with `num_cliques` cliques of `clique_size` nodes each,
/// connected by sparse inter-clique edges (to give Louvain something to detect).
async fn create_clique_graph(
    driver: &MemoryDriver,
    num_cliques: usize,
    clique_size: usize,
) -> usize {
    let mut entities = Vec::new();
    let mut total_edges = 0;

    // Create cliques
    for c in 0..num_cliques {
        let mut clique_entities = Vec::new();
        for i in 0..clique_size {
            let name = format!("C{}_Node_{}", c, i);
            let entity = Entity::new(&name, "Node");
            driver.store_entity(&entity).await.unwrap();
            clique_entities.push(entity);
        }

        // Fully connect within clique
        for i in 0..clique_size {
            for j in (i + 1)..clique_size {
                let rel = Relationship::new(
                    clique_entities[i].id.clone(),
                    clique_entities[j].id.clone(),
                    "INTRA_CLIQUE",
                    &format!("C{}: {} to {}", c, i, j),
                );
                driver.store_relationship(&rel).await.unwrap();
                total_edges += 1;
            }
        }

        entities.extend(clique_entities);
    }

    // Add sparse inter-clique edges (1 edge between adjacent cliques)
    for c in 0..(num_cliques - 1) {
        let src_idx = c * clique_size;
        let tgt_idx = (c + 1) * clique_size;
        let rel = Relationship::new(
            entities[src_idx].id.clone(),
            entities[tgt_idx].id.clone(),
            "INTER_CLIQUE",
            &format!("Bridge clique {} to {}", c, c + 1),
        )
        .with_weight(0.1); // Weak bridge
        driver.store_relationship(&rel).await.unwrap();
        total_edges += 1;
    }

    total_edges
}

fn bench_louvain_detection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("louvain_detection");
    group.sample_size(10);

    // (num_cliques, clique_size) → total nodes
    let configs = [
        (5, 10),  // 50 nodes
        (10, 10), // 100 nodes
        (20, 10), // 200 nodes
        (10, 50), // 500 nodes
        (20, 50), // 1000 nodes
    ];

    for (num_cliques, clique_size) in configs {
        let total_nodes = num_cliques * clique_size;
        let label = format!("{}n_{}c", total_nodes, num_cliques);

        let driver = rt.block_on(async {
            let d = MemoryDriver::new();
            create_clique_graph(&d, num_cliques, clique_size).await;
            d
        });

        group.bench_with_input(BenchmarkId::new("detect", &label), &label, |b, _| {
            b.to_async(&rt).iter(|| {
                let d = &driver;
                async move {
                    let detector = LouvainDetector::new();
                    black_box(detector.detect(d, None).await.unwrap());
                }
            });
        });
    }
    group.finish();
}

fn bench_louvain_resolution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("louvain_resolution");
    group.sample_size(10);

    // Same graph, different resolution parameters
    let driver = rt.block_on(async {
        let d = MemoryDriver::new();
        create_clique_graph(&d, 10, 20).await; // 200 nodes
        d
    });

    for resolution in [0.5, 1.0, 1.5, 2.0] {
        group.bench_with_input(
            BenchmarkId::new("resolution", format!("{:.1}", resolution)),
            &resolution,
            |b, &res| {
                b.to_async(&rt).iter(|| {
                    let d = &driver;
                    async move {
                        let config = LouvainConfig {
                            resolution: res,
                            ..Default::default()
                        };
                        let detector = LouvainDetector::with_config(config);
                        black_box(detector.detect(d, None).await.unwrap());
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_louvain_detection, bench_louvain_resolution,);
criterion_main!(benches);

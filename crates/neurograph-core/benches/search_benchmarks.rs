// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Search benchmarks: vector search, text search, hybrid, and graph traversal.

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use neurograph_core::drivers::memory::MemoryDriver;
use neurograph_core::drivers::traits::GraphDriver;
use neurograph_core::graph::{Entity, Relationship};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Create a graph with `n` entities, each with a 3-dim embedding.
async fn create_searchable_graph(driver: &MemoryDriver, n: usize) {
    for i in 0..n {
        let angle = (i as f32) * std::f32::consts::TAU / (n as f32);
        let entity = Entity::new(&format!("Entity_{}", i), "Concept")
            .with_summary(&format!("Entity number {} in the graph for testing search performance", i))
            .with_embedding(vec![angle.cos(), angle.sin(), (i as f32) / (n as f32)]);
        driver.store_entity(&entity).await.unwrap();
    }
}

/// Create a connected graph for traversal benchmarks.
async fn create_traversal_graph(driver: &MemoryDriver, n: usize) -> Vec<Entity> {
    let mut entities = Vec::with_capacity(n);
    for i in 0..n {
        let entity = Entity::new(&format!("Node_{}", i), "Node");
        driver.store_entity(&entity).await.unwrap();
        entities.push(entity);
    }
    // Create chain + random connections
    for i in 0..(n - 1) {
        let rel = Relationship::new(
            entities[i].id.clone(),
            entities[i + 1].id.clone(),
            "LINKS_TO",
            &format!("Node_{} links to Node_{}", i, i + 1),
        );
        driver.store_relationship(&rel).await.unwrap();
    }
    // Add some cross-links for graph density
    for i in (0..n).step_by(3) {
        let target = (i + n / 3) % n;
        if i != target {
            let rel = Relationship::new(
                entities[i].id.clone(),
                entities[target].id.clone(),
                "CROSS_LINK",
                &format!("Cross link {} to {}", i, target),
            );
            driver.store_relationship(&rel).await.unwrap();
        }
    }
    entities
}

fn bench_vector_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("vector_search");

    for n in [100, 1_000, 5_000] {
        let driver = rt.block_on(async {
            let d = MemoryDriver::new();
            create_searchable_graph(&d, n).await;
            d
        });

        group.bench_with_input(
            BenchmarkId::new("cosine_top10", n),
            &n,
            |b, _| {
                let query_vec = vec![1.0_f32, 0.0, 0.5];
                b.to_async(&rt).iter(|| {
                    let d = &driver;
                    let q = &query_vec;
                    async move {
                        black_box(
                            d.search_entities_by_vector(q, 10, None).await.unwrap()
                        );
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_text_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("text_search");

    for n in [100, 1_000, 5_000] {
        let driver = rt.block_on(async {
            let d = MemoryDriver::new();
            create_searchable_graph(&d, n).await;
            d
        });

        group.bench_with_input(
            BenchmarkId::new("keyword_top10", n),
            &n,
            |b, _| {
                b.to_async(&rt).iter(|| {
                    let d = &driver;
                    async move {
                        black_box(
                            d.search_entities_by_text("Entity number 42", 10, None)
                                .await
                                .unwrap()
                        );
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_graph_traversal(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("graph_traversal");

    for n in [100, 500, 1_000] {
        let (driver, entities) = rt.block_on(async {
            let d = MemoryDriver::new();
            let e = create_traversal_graph(&d, n).await;
            (d, e)
        });

        for max_depth in [1, 2, 3] {
            group.bench_with_input(
                BenchmarkId::new(
                    format!("bfs_depth{}", max_depth),
                    n,
                ),
                &n,
                |b, _| {
                    let start_id = &entities[0].id;
                    b.to_async(&rt).iter(|| {
                        let d = &driver;
                        async move {
                            black_box(
                                d.traverse(start_id, max_depth, None).await.unwrap()
                            );
                        }
                    });
                },
            );
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_vector_search,
    bench_text_search,
    bench_graph_traversal,
);
criterion_main!(benches);

// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Engine benchmarks: ingestion throughput, query latency, entity storage.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use neurograph_core::drivers::memory::MemoryDriver;
use neurograph_core::graph::{Entity, Relationship};
use neurograph_core::NeuroGraph;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn bench_entity_storage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("entity_storage");

    for count in [10, 100, 1_000, 5_000] {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(BenchmarkId::new("store_entity", count), &count, |b, &n| {
            b.to_async(&rt).iter(|| async move {
                let driver = MemoryDriver::new();
                for i in 0..n {
                    let entity = Entity::new(&format!("Entity_{}", i), "Concept");
                    driver.store_entity(&entity).await.unwrap();
                }
                black_box(driver.stats().await.unwrap());
            });
        });
    }
    group.finish();
}

fn bench_relationship_storage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("relationship_storage");

    for count in [10, 100, 1_000] {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("store_relationship", count),
            &count,
            |b, &n| {
                b.to_async(&rt).iter(|| async move {
                    let driver = MemoryDriver::new();
                    // Pre-create entities
                    let mut entities = Vec::new();
                    for i in 0..n {
                        let entity = Entity::new(&format!("E_{}", i), "Node");
                        driver.store_entity(&entity).await.unwrap();
                        entities.push(entity);
                    }
                    // Store relationships (chain: 0→1→2→...→n)
                    for i in 0..(n - 1) {
                        let rel = Relationship::new(
                            entities[i].id.clone(),
                            entities[i + 1].id.clone(),
                            "CONNECTS_TO",
                            &format!("E_{} connects to E_{}", i, i + 1),
                        );
                        driver.store_relationship(&rel).await.unwrap();
                    }
                    black_box(driver.stats().await.unwrap());
                });
            },
        );
    }
    group.finish();
}

fn bench_neurograph_add_text(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("ingestion");
    group.sample_size(10); // Ingestion is slower, use fewer samples

    for count in [1, 5, 10, 50] {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(BenchmarkId::new("add_text", count), &count, |b, &n| {
            b.to_async(&rt).iter(|| async move {
                let ng = NeuroGraph::builder().memory().build().await.unwrap();

                for i in 0..n {
                    ng.add_text(&format!("Person_{i} works at Company_{i} in City_{i}"))
                        .await
                        .unwrap();
                }
                black_box(ng.stats().await.unwrap());
            });
        });
    }
    group.finish();
}

fn bench_query_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("query_latency");
    group.sample_size(10);

    // Pre-populate, then measure query
    for graph_size in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("simple_query", graph_size),
            &graph_size,
            |b, &size| {
                let ng = rt.block_on(async {
                    let ng = NeuroGraph::builder().memory().build().await.unwrap();
                    for i in 0..size {
                        let entity = Entity::new(&format!("Person_{}", i), "Person")
                            .with_summary(&format!("Person {} works at Company_{}", i, i));
                        ng.store_entity(&entity).await.unwrap();
                    }
                    ng
                });

                b.to_async(&rt).iter(|| {
                    let ng_ref = &ng;
                    async move {
                        black_box(ng_ref.query("Where does Person_5 work?").await.unwrap());
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_builder(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("initialization");

    group.bench_function("builder_memory", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(NeuroGraph::builder().memory().build().await.unwrap());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_entity_storage,
    bench_relationship_storage,
    bench_neurograph_add_text,
    bench_query_latency,
    bench_builder,
);
criterion_main!(benches);

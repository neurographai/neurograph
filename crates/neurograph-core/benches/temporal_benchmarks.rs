// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Temporal benchmarks: snapshot reconstruction, diff computation, timeline building.

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion,
};
use chrono::{Duration, Utc};
use neurograph_core::drivers::memory::MemoryDriver;
use neurograph_core::drivers::traits::GraphDriver;
use neurograph_core::graph::{Entity, EntityId, Relationship};
use neurograph_core::temporal::manager::TemporalManager;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Create a graph with entities spread across time.
async fn create_temporal_graph(driver: &MemoryDriver, n: usize) {
    let now = Utc::now();
    for i in 0..n {
        let mut entity = Entity::new(&format!("TemporalEntity_{}", i), "Event");
        entity.created_at = now - Duration::days((n - i) as i64);
        driver.store_entity(&entity).await.unwrap();

        // Create some relationships with temporal validity
        if i > 0 {
            let prev_entities = driver.list_entities(None, i).await.unwrap();
            if let Some(prev) = prev_entities.last() {
                let rel = Relationship::new(
                    prev.id.clone(),
                    entity.id.clone(),
                    "FOLLOWS",
                    &format!("Event {} follows event {}", i, i - 1),
                )
                .with_valid_from(now - Duration::days((n - i) as i64));
                driver.store_relationship(&rel).await.unwrap();
            }
        }
    }
}

fn bench_snapshot_at(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("temporal_snapshot");

    for n in [50, 100, 500, 1_000] {
        let (driver, _) = rt.block_on(async {
            let d = Arc::new(MemoryDriver::new());
            create_temporal_graph(&d, n).await;
            (d, ())
        });

        group.bench_with_input(
            BenchmarkId::new("snapshot_at_midpoint", n),
            &n,
            |b, &size| {
                let midpoint = Utc::now() - Duration::days((size / 2) as i64);
                b.to_async(&rt).iter(|| {
                    let d = driver.clone();
                    let ts = midpoint;
                    async move {
                        black_box(
                            d.snapshot_at(&ts, None).await.unwrap()
                        );
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("snapshot_at_recent", n),
            &n,
            |b, _| {
                let recent = Utc::now() - Duration::hours(1);
                b.to_async(&rt).iter(|| {
                    let d = driver.clone();
                    let ts = recent;
                    async move {
                        black_box(
                            d.snapshot_at(&ts, None).await.unwrap()
                        );
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_what_changed(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("temporal_diff");

    for n in [50, 100, 500, 1_000] {
        let driver = rt.block_on(async {
            let d = Arc::new(MemoryDriver::new());
            create_temporal_graph(&d, n).await;
            d
        });

        group.bench_with_input(
            BenchmarkId::new("what_changed", n),
            &n,
            |b, &size| {
                let from = Utc::now() - Duration::days(size as i64);
                let to = Utc::now() - Duration::days((size / 2) as i64);
                b.to_async(&rt).iter(|| {
                    let d = driver.clone();
                    async move {
                        let mgr = TemporalManager::new(d);
                        black_box(
                            mgr.what_changed(from, to, None).await.unwrap()
                        );
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_build_timeline(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("temporal_timeline");

    for n in [50, 100, 500, 1_000] {
        let driver = rt.block_on(async {
            let d = Arc::new(MemoryDriver::new());
            create_temporal_graph(&d, n).await;
            d
        });

        group.bench_with_input(
            BenchmarkId::new("build_timeline", n),
            &n,
            |b, _| {
                b.to_async(&rt).iter(|| {
                    let d = driver.clone();
                    async move {
                        let mgr = TemporalManager::new(d);
                        black_box(
                            mgr.build_timeline(None).await.unwrap()
                        );
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_date_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("date_parsing");

    let formats = [
        ("iso_date", "2025-06-15"),
        ("iso_datetime", "2025-06-15T10:30:00Z"),
        ("year_only", "2025"),
        ("slash_date", "2025/06/15"),
    ];

    for (name, date_str) in formats {
        group.bench_with_input(
            BenchmarkId::new("parse", name),
            &date_str,
            |b, &s| {
                b.iter(|| {
                    black_box(
                        TemporalManager::parse_date(s).unwrap()
                    );
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_snapshot_at,
    bench_what_changed,
    bench_build_timeline,
    bench_date_parsing,
);
criterion_main!(benches);

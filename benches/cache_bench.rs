// benches/cache_bench.rs
use criterion::{Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;
use uuid::Uuid;
use vrc_log::cache::Cache;

fn bench_cache_store_and_check(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Prepare the cache outside of the measured iterations
    let cache = rt.block_on(async { Cache::new_in_memory().await.unwrap() });

    // Pre-generate avatars to avoid measuring UUID generation
    let n = 10_000;
    #[allow(clippy::cast_possible_truncation)]
    let avatars: Vec<(String, u32)> = (0..n)
        .map(|i| (format!("avtr_{}", Uuid::new_v4()), i as u32))
        .collect();

    c.bench_function("store 10k avatars", |b| {
        b.iter(|| {
            rt.block_on(async {
                cache
                    .store_avatar_ids_with_providers(
                        avatars.iter().map(|(id, p)| (id.as_str(), *p)),
                    )
                    .await
                    .unwrap();
            });
        });
    });

    c.bench_function("check 10k avatars", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = cache
                    .check_all_ids(avatars.iter().map(|(id, _)| id.clone()))
                    .await
                    .unwrap();

                // Sanity check
                assert_eq!(result.len(), n);
            });
        });
    });
}

criterion_group!(cache_benches, bench_cache_store_and_check);
criterion_main!(cache_benches);

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use haqumei::{Haqumei, HaqumeiOptions, ParallelJTalk};
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;

fn bench_g2p(c: &mut Criterion) {
    let mut group = c.benchmark_group("G2P Performance");

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let waganeko = fs::read_to_string(manifest_dir.join("../resources/waganeko.txt")).unwrap();
    let lines: Vec<&str> = waganeko.lines().filter(|l| !l.is_empty()).collect();

    let total_chars: u64 = lines.iter().map(|l| l.chars().count() as u64).sum();
    group.throughput(Throughput::Elements(total_chars));

    group.sample_size(10);

    let mut hq_default = Haqumei::new().unwrap();
    group.bench_function("MultiThread (Default)", |b| {
        b.iter(|| black_box(hq_default.g2p_batch(black_box(&lines))))
    });

    let mut hq_heavy = Haqumei::with_options(HaqumeiOptions {
        modify_kanji_yomi: true,
        ..Default::default()
    })
    .unwrap();
    group.bench_function("SingleThread (Heavy Options)", |b| {
        b.iter(|| black_box(hq_heavy.g2p_batch(black_box(&lines))))
    });

    let pojt = ParallelJTalk::new().unwrap();
    group.bench_function("ParallelJTalk (Batch)", |b| {
        b.iter(|| {
            black_box(pojt.g2p(black_box(&lines)).unwrap());
        })
    });

    group.bench_function("G2P Mapping Detailed", |b| {
        b.iter(|| black_box(hq_default.g2p_mapping_detailed_batch(black_box(&lines))))
    });

    group.finish();
}

criterion_group!(benches, bench_g2p);
criterion_main!(benches);

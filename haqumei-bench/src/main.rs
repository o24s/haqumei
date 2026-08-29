use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use haqumei::{Haqumei, HaqumeiOptions, OpenJTalk};
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

    let mut haqumei = Haqumei::new().unwrap();
    group.bench_function("MultiThread (Default)", |b| {
        b.iter(|| black_box(haqumei.g2p_batch(black_box(&lines))))
    });

    let mut ojt = OpenJTalk::new().unwrap();
    group.bench_function("OpenJTalk (Batch)", |b| {
        b.iter(|| {
            black_box(ojt.g2p_batch(black_box(&lines)).unwrap());
        })
    });

    group.bench_function("G2P Mapping", |b| {
        b.iter(|| black_box(haqumei.g2p_mapping_batch(black_box(&lines))))
    });

    group.finish();
}

/// 文脈読みの補正を入れたときと外したときで測る。
fn bench_context_reading(c: &mut Criterion) {
    let mut group = c.benchmark_group("Context Reading");

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let waganeko = fs::read_to_string(manifest_dir.join("../resources/waganeko.txt")).unwrap();
    let lines: Vec<&str> = waganeko.lines().filter(|l| !l.is_empty()).collect();

    let total_chars: u64 = lines.iter().map(|l| l.chars().count() as u64).sum();
    group.throughput(Throughput::Elements(total_chars));
    group.sample_size(10);

    for (label, on) in [("On", true), ("Off", false)] {
        let mut haqumei = Haqumei::with_options(HaqumeiOptions {
            modify_context_reading: on,
            ..Default::default()
        })
        .unwrap();
        group.bench_function(label, |b| {
            b.iter(|| black_box(haqumei.g2p_batch(black_box(&lines))))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_g2p, bench_context_reading);
criterion_main!(benches);

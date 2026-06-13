use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use haqumei_kanalizer::{ConvertOptions, Kanalizer, MaxLength, Strategy, StrategyTopP};

fn bench_convert(c: &mut Criterion) {
    let mut group = c.benchmark_group("kanalizer_convert");

    let cases = [
        ("short", "hello"),
        ("medium", "international"),
        ("long", "internationalization"),
    ];

    group.sample_size(30);
    group.measurement_time(std::time::Duration::from_secs(10));
    let mut kanalizer = Kanalizer::new().unwrap();

    for (name, input) in cases {
        group.bench_with_input(BenchmarkId::new("greedy", name), input, |b, input| {
            b.iter(|| {
                black_box(kanalizer.convert(black_box(input)).unwrap());
            });
        });

        group.bench_with_input(BenchmarkId::new("topp", name), input, |b, input| {
            b.iter(|| {
                black_box(
                    kanalizer
                        .convert_with_options(
                            black_box(input),
                            &ConvertOptions {
                                strategy: Strategy::TopP(StrategyTopP {
                                    top_p: 0.9,
                                    temperature: 1.0,
                                }),
                                max_length: MaxLength::Auto,
                                ..Default::default()
                            },
                        )
                        .unwrap(),
                );
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_convert);
criterion_main!(benches);

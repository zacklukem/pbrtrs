use criterion::{criterion_group, Criterion};

pub fn bench_random_vec(c: &mut Criterion) {
    c.bench_function("random_vec", |b| {
        b.iter(|| {
            pbrtrs_core::util::random_vec();
        });
    });
}

criterion_group!(benches, bench_random_vec);

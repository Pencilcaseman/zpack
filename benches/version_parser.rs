use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use zpack::package::version::Version;

fn criterion_benchmark(c: &mut Criterion) {
    let bench_suite = [
        "1.9.0",
        "v1.10.0",
        "v1.11.0",
        "1.0.0-alpha",
        "v1.0.0-alpha.1",
        "v1.0.0-0.3.7",
        "1.0.0-x.7.z.92",
        "v1.0.0-x-y-z.--",
        "1.0.0-alpha+001",
        "v1.0.0+20130313144700",
        "1.0.0-beta+exp.sha.5114f85",
        "v1.0.0+21AF26D3----117B344092BD",
        "v123456789.123456789.123456789-123456789+0123456789",
    ];

    for input in bench_suite.into_iter() {
        c.bench_function(&format!("semver.org '{}'", input), |b| {
            b.iter(|| black_box(Version::new(input)))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

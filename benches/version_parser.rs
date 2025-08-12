use criterion::{Criterion, criterion_group, criterion_main};

use zpack::package::version::Version;

use std::hint::black_box;

fn parse_version() -> Version {
    let test_version = "v123456789.123456789.123456789-123456789";
    Version::new(black_box(test_version)).expect("Failed to parse version")
}

fn criterion_benchmark(c: &mut Criterion) {
    assert_eq!(
        parse_version(),
        Version::SemVer {
            major: 123456789,
            minor: Some(123456789),
            patch: Some(123456789),
            rc: Some("123456789".into())
        }
    );

    c.bench_function("parse_long_version", |b| {
        b.iter(|| black_box(parse_version()))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

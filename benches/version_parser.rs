use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use zpack::package::version::Version;

fn parse_short_version() -> Version {
    let test_version = "123.456";
    Version::new(black_box(test_version)).expect("Failed to parse version")
}

fn parse_long_version() -> Version {
    let test_version = "v123456789.123456789.123456789-123456789";
    Version::new(black_box(test_version)).expect("Failed to parse version")
}

fn custom_parser() -> (u64, u64) {
    use zpack::util::parse::*;

    fn small_version_parser(version: &str) -> Result<(u64, u64), ()> {
        let dot = MatchConsumer::new(".");
        let num = IntegerConsumer::new();

        // Parses <num>.<num>
        let semver = num
            .map(|v| Ok(u64::try_from(v)?))
            .then_ignore(dot)
            .then(num.map(|v| Ok(u64::try_from(v)?)))
            .map(|(major, minor)| Ok((major, minor)));

        let cur = Cursor::new(version);

        match semver.consume(cur) {
            Ok((res, _)) => Ok(res),
            Err(_) => Err(()),
        }
    }

    let test_version = "123.456";
    small_version_parser(black_box(test_version))
        .expect("Failed to parse version")
}

fn criterion_benchmark(c: &mut Criterion) {
    assert_eq!(
        parse_short_version(),
        Version::SemVer {
            major: 123,
            minor: Some(456),
            patch: None,
            rc: None,
            meta: None
        }
    );

    assert_eq!(
        parse_long_version(),
        Version::SemVer {
            major: 123456789,
            minor: Some(123456789),
            patch: Some(123456789),
            rc: Some(vec!["123456789".into()]),
            meta: None
        }
    );

    assert_eq!(custom_parser(), (123, 456));

    c.bench_function("parse_short_version", |b| {
        b.iter(|| black_box(parse_short_version()))
    });

    c.bench_function("parse_long_version", |b| {
        b.iter(|| black_box(parse_long_version()))
    });

    c.bench_function("custom_parser", |b| {
        b.iter(|| black_box(custom_parser()))
    });

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
    ];

    for input in bench_suite.into_iter() {
        c.bench_function(&format!("semver.org '{}'", input), |b| {
            b.iter(|| black_box(Version::new(input)))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

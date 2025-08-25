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
        Version::SemVer { major: 123, minor: Some(456), patch: None, rc: None }
    );

    assert_eq!(
        parse_long_version(),
        Version::SemVer {
            major: 123456789,
            minor: Some(123456789),
            patch: Some(123456789),
            rc: Some("123456789".into())
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
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

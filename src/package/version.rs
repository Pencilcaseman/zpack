use anyhow::{Context, Result, anyhow};

use crate::util::parse::*;

/// A type representing a concrete version.
///
/// Multiple version types are supported, including:
///  - Semantic Versioning: [`Version::SemVer`]
///  - Other: [`Version::Other`]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Version {
    SemVer {
        major: u64,
        minor: Option<u64>,
        patch: Option<u64>,
        rc: Option<String>,
    },
    Other(String),
}

/// Parse a version string and return a concrete Version option, if possible. If
/// a concrete version cannot be parsed, return a string representation of the
/// version.
fn parse_version(version: &str) -> Result<Version> {
    let v = MatchConsumer::new("v");
    let dot = MatchConsumer::new(".");
    let dash = MatchConsumer::new("-");
    let num = IntegerConsumer::new();

    let opt_v = OptionalConsumer::new(v);

    // Parses [v]<num>[.<num>][.<num>][-<something>]
    let semver = opt_v
        .ignore_then(num.map(|v| Ok(u64::try_from(v)?)))
        .maybe(dot.ignore_then(num.map(|v| Ok(u64::try_from(v)?))))
        .maybe(dot.ignore_then(num.map(|v| Ok(u64::try_from(v)?))))
        .maybe(dash.ignore_then(RawConsumer::new(|cursor| {
            match cursor.take_while(|c| !c.is_ascii_whitespace()) {
                Some((rc, cur)) if !rc.is_empty() => Ok((rc.to_owned(), cur)),
                _ => Err(anyhow!("Did not find additional version info")),
            }
        })))
        .map(|(((major, minor), patch), rc)| -> Result<_> {
            Ok(Version::SemVer { major, minor, patch, rc })
        });

    let cur = Cursor::new(version);

    match semver.consume(cur) {
        Ok((res, _)) => Ok(res),
        Err(_) => Err(anyhow!("Parsing version failed")),
    }
}

impl Version {
    pub fn new(ver: &str) -> Result<Self> {
        parse_version(ver)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_version() -> Result<()> {
        let v1 = Version::new("v1.23.4")?;
        assert_eq!(
            v1,
            Version::SemVer {
                major: 1,
                minor: Some(23),
                patch: Some(4),
                rc: None
            }
        );

        let v2 = Version::new("123.456-dev")?;

        assert_eq!(
            v2,
            Version::SemVer {
                major: 123,
                minor: Some(456),
                patch: None,
                rc: Some("dev".into())
            }
        );

        Ok(())
    }
}

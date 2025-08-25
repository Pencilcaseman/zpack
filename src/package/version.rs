use std::cmp;

use anyhow::{Result, anyhow};

use crate::util::parse::*;

/// A type representing a concrete version.
///
/// Multiple version types are supported, including:
///  - Semantic Versioning: [`Version::SemVer`]
///  - Other: [`Version::Other`]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Version {
    /// Semantic Versioning
    ///
    /// See [https://semver.org](https://semver.org) for more information
    SemVer {
        major: u64,
        minor: Option<u64>,
        patch: Option<u64>,
        rc: Option<Vec<String>>,
        meta: Option<Vec<String>>,
    },

    /// Any other arbitrary version specifier
    Other(Vec<String>),
}

/// Parse a version string and return a concrete Version option, if possible. If
/// a concrete version cannot be parsed, return a string representation of the
/// version.
fn parse_version(version: &str) -> Result<Version> {
    let v = MatchConsumer::new("v");
    let dot = MatchConsumer::new(".");
    let dash = MatchConsumer::new("-");
    let plus = MatchConsumer::new("+");
    let num = IntegerConsumer::new();

    let opt_v = OptionalConsumer::new(v);

    let dot_separated = BoundedConsumer::new(
        None,
        None,
        RawConsumer::new(|cursor| {
            cursor
                .take_while_non_zero(|c| c.is_alphanumeric() || *c == '-')
                .map(|(s, c)| (s.to_string(), c))
                .ok_or(anyhow!("Expected alphanumeric or hyphen"))
        })
        .maybe_ignore(dot),
    );

    let semver = opt_v
        .ignore_then(num.map(|v| Ok(u64::try_from(v)?)))
        .maybe(dot.ignore_then(num.map(|v| Ok(u64::try_from(v)?))))
        .maybe(dot.ignore_then(num.map(|v| Ok(u64::try_from(v)?))))
        .maybe(dash.ignore_then(dot_separated.clone()))
        .maybe(plus.ignore_then(dot_separated))
        .map(|((((major, minor), patch), rc), meta)| -> Result<_> {
            Ok(Version::SemVer { major, minor, patch, rc, meta })
        });

    let cur = Cursor::new(version);

    match semver.consume(cur) {
        Ok((res, _)) => Ok(res),
        Err(e) => Err(anyhow!("Parsing version failed: {e:?}")),
    }
}

impl Version {
    pub fn new(ver: &str) -> Result<Self> {
        parse_version(ver)
    }
}

impl std::cmp::PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Versions can only be compared with another version of the same type
        if std::mem::discriminant(self) != std::mem::discriminant(other) {
            return None;
        }

        use std::cmp::Ordering;

        use Version::*;

        match (self, other) {
            (
                SemVer {
                    major: major1,
                    minor: minor1,
                    patch: patch1,
                    rc: rc1,
                    meta: _,
                },
                SemVer {
                    major: major2,
                    minor: minor2,
                    patch: patch2,
                    rc: rc2,
                    meta: _,
                },
            ) => {
                let version_cmp = (major1, minor1, patch1)
                    .partial_cmp(&(major2, minor2, patch2));

                if version_cmp.is_none() {
                    return version_cmp;
                }

                if !matches!(version_cmp, Some(Ordering::Equal)) {
                    version_cmp
                } else {
                    // Compare release candidates
                    match (rc1, rc2) {
                        (None, None) => None,
                        (None, Some(_)) => Some(Ordering::Less),
                        (Some(_), None) => Some(Ordering::Greater),
                        (Some(s1), Some(s2)) => {
                            let rc_ordering = ["alpha", "dev"];

                            todo!()
                        }
                    }
                }
            }
            (Other(string1), Other(string2)) => string1.partial_cmp(string2),
            _ => None,
        }
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
                rc: None,
                meta: None
            }
        );

        let v2 = Version::new("123.456-dev")?;

        assert_eq!(
            v2,
            Version::SemVer {
                major: 123,
                minor: Some(456),
                patch: None,
                rc: Some(vec!["dev".to_string()]),
                meta: None
            }
        );

        let v3 = Version::new("1.0.0+21AF26D3----117B344092BD.DEVEL")?;

        assert_eq!(
            v3,
            Version::SemVer {
                major: 1,
                minor: Some(0),
                patch: Some(0),
                rc: None,
                meta: Some(vec![
                    "21AF26D3----117B344092BD".to_string(),
                    "DEVEL".to_string()
                ])
            }
        );

        let test_suite = [
            (
                "1.9.0",
                Version::SemVer {
                    major: 1,
                    minor: Some(9),
                    patch: Some(0),
                    rc: None,
                    meta: None,
                },
            ),
            (
                "1.10.0",
                Version::SemVer {
                    major: 1,
                    minor: Some(10),
                    patch: Some(0),
                    rc: None,
                    meta: None,
                },
            ),
            (
                "1.11.0",
                Version::SemVer {
                    major: 1,
                    minor: Some(11),
                    patch: Some(0),
                    rc: None,
                    meta: None,
                },
            ),
            (
                "1.0.0-alpha",
                Version::SemVer {
                    major: 1,
                    minor: Some(0),
                    patch: Some(0),
                    rc: Some(vec!["alpha".into()]),
                    meta: None,
                },
            ),
            (
                "1.0.0-alpha.1",
                Version::SemVer {
                    major: 1,
                    minor: Some(0),
                    patch: Some(0),
                    rc: Some(vec!["alpha".into(), "1".into()]),
                    meta: None,
                },
            ),
            (
                "1.0.0-0.3.7",
                Version::SemVer {
                    major: 1,
                    minor: Some(0),
                    patch: Some(0),
                    rc: Some(vec!["0".into(), "3".into(), "7".into()]),
                    meta: None,
                },
            ),
            (
                "1.0.0-x.7.z.92",
                Version::SemVer {
                    major: 1,
                    minor: Some(0),
                    patch: Some(0),
                    rc: Some(vec![
                        "x".into(),
                        "7".into(),
                        "z".into(),
                        "92".into(),
                    ]),
                    meta: None,
                },
            ),
            (
                "1.0.0-x-y-z.--",
                Version::SemVer {
                    major: 1,
                    minor: Some(0),
                    patch: Some(0),
                    rc: Some(vec!["x-y-z".into(), "--".into()]),
                    meta: None,
                },
            ),
            (
                "1.0.0-alpha+001",
                Version::SemVer {
                    major: 1,
                    minor: Some(0),
                    patch: Some(0),
                    rc: Some(vec!["alpha".into()]),
                    meta: Some(vec!["001".into()]),
                },
            ),
            (
                "1.0.0+20130313144700",
                Version::SemVer {
                    major: 1,
                    minor: Some(0),
                    patch: Some(0),
                    rc: None,
                    meta: Some(vec!["20130313144700".into()]),
                },
            ),
            (
                "1.0.0-beta+exp.sha.5114f85",
                Version::SemVer {
                    major: 1,
                    minor: Some(0),
                    patch: Some(0),
                    rc: Some(vec!["beta".into()]),
                    meta: Some(vec![
                        "exp".into(),
                        "sha".into(),
                        "5114f85".into(),
                    ]),
                },
            ),
            (
                "1.0.0+21AF26D3----117B344092BD",
                Version::SemVer {
                    major: 1,
                    minor: Some(0),
                    patch: Some(0),
                    rc: None,
                    meta: Some(vec!["21AF26D3----117B344092BD".into()]),
                },
            ),
        ];

        for (string, version) in test_suite.into_iter() {
            assert_eq!(Version::new(string)?, version);
        }

        Ok(())
    }
}

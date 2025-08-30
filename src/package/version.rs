use chumsky::prelude::*;
use pyo3::prelude::*;

use crate::util::error::{ParserErrorType, ParserErrorWrapper};

/// A type representing a concrete version.
///
/// Multiple version types are supported, including:
///  - Semantic Versioning: [`Version::SemVer`]
///  - Other: [`Version::Other`]
#[pyclass(str)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Version {
    /// Semantic Versioning
    ///
    /// For example: 8.4.7-alpha+5d41402a
    ///
    /// See [https://semver.org](https://semver.org) for more information
    SemVer {
        /// Major version
        major: u32,

        /// Minor version
        minor: u32,

        /// Patch version
        patch: u32,

        // Pre-release
        rc: Option<Vec<String>>,

        /// Metadata
        meta: Option<Vec<String>>,
    },

    /// Arbitrary dot-separated version
    ///
    /// For example: 2025.06.alpha.3
    DotSeparated { parts: Vec<String> },

    /// Any other arbitrary version specifier
    Other { value: String },
}

impl Version {
    /// Create a new [`Version`] instance from a string. See [`Version`] for
    /// more information regarding valid version types
    ///
    /// * `ver`: Version string
    pub fn new<'a>(
        ver: &'a str,
    ) -> Result<Self, ParserErrorWrapper<'a, ariadne::Source<&'a str>>> {
        let parser = Self::parser();
        parser.parse(ver).into_result().map_err(|errs| {
            ParserErrorWrapper::new("Version", ariadne::Source::from(ver), errs)
        })
    }

    /// Create a new [`Version`] from a string and a version parser. See
    /// [`Version::new()`] also.
    ///
    /// * `ver`:
    /// * `parser`:
    pub fn new_with_parser<'a>(
        ver: &'a str,
        parser: impl Parser<'a, &'a str, Version, extra::Err<ParserErrorType<'a>>>,
    ) -> Result<Self, ParserErrorWrapper<'a, ariadne::Source<&'a str>>> {
        parser.parse(ver).into_result().map_err(|errs| {
            ParserErrorWrapper::new("Version", ariadne::Source::from(ver), errs)
        })
    }

    pub fn new_semver<'a>(
        ver: &'a str,
    ) -> Result<Self, ParserErrorWrapper<'a, ariadne::Source<&'a str>>> {
        Self::new_with_parser(ver, Self::semver_parser())
    }

    pub fn new_dot_separated<'a>(
        ver: &'a str,
    ) -> Result<Self, ParserErrorWrapper<'a, ariadne::Source<&'a str>>> {
        Self::new_with_parser(ver, Self::dot_separated_parser())
    }

    pub fn parser<'a>()
    -> impl Parser<'a, &'a str, Self, extra::Err<ParserErrorType<'a>>> {
        choice((
            Self::semver_parser().then_ignore(end()),
            Self::dot_separated_parser().then_ignore(end()),
        ))
    }

    /// Parse a version string and return a concrete Version option, if
    /// possible. If a concrete version cannot be parsed, return a string
    /// representation of the version.
    pub fn semver_parser<'a>()
    -> impl Parser<'a, &'a str, Self, extra::Err<ParserErrorType<'a>>> {
        let core = int()
            .separated_by(just('.'))
            .collect_exactly::<[_; 3]>()
            .recover_with(via_parser(
                none_of("-+").repeated().map(|_| [0, 0, 0]),
            ));

        let pre_release = just('-').ignore_then(dot_sep_idents().recover_with(
            skip_then_retry_until(any().ignored(), one_of(".+").ignored()),
        ));

        let metadata = just('+').ignore_then(dot_sep_idents().recover_with(
            skip_then_retry_until(
                any().ignored(),
                one_of(".+").ignored().or(end().ignored()),
            ),
        ));

        just('v')
            .or_not()
            .ignore_then(core)
            .then(pre_release.or_not())
            .then(metadata.or_not())
            .map(|((version, rc), meta)| Version::SemVer {
                major: version[0],
                minor: version[1],
                patch: version[2],
                rc,
                meta,
            })
    }

    pub fn dot_separated_parser<'a>()
    -> impl Parser<'a, &'a str, Self, extra::Err<ParserErrorType<'a>>> {
        dot_sep_idents()
            .recover_with(skip_then_retry_until(
                any().ignored(),
                just('.').ignored(),
            ))
            .map(|parts| Self::DotSeparated { parts })
    }
}

fn py_new_helper<'a, F>(ver: &'a str, constructor: F) -> PyResult<Version>
where
    F: Fn(
        &'a str,
    )
        -> Result<Version, ParserErrorWrapper<'a, ariadne::Source<&'a str>>>,
{
    use pyo3::exceptions::PyValueError;

    (constructor)(ver).map_err(|err| match err.build() {
        Some(built) => {
            PyValueError::new_err(built.to_string().unwrap_or_else(|e| e))
        }
        None => PyValueError::new_err(""),
    })
}

#[pymethods]
impl Version {
    #[new]
    pub fn py_new(ver: &str) -> PyResult<Self> {
        py_new_helper(ver, Version::new)
    }

    #[pyo3(name = "new_semver")]
    #[staticmethod]
    pub fn py_new_semver(ver: &str) -> PyResult<Self> {
        py_new_helper(ver, Version::new_semver)
    }

    pub fn __repr__(&self) -> String {
        match self {
            Self::SemVer { major, minor, patch, rc, meta } => {
                format!(
                    "Version.SemVer(major={major}, minor={minor}, patch={patch}, rc={rc:?}, meta={meta:?})"
                )
            }
            Self::DotSeparated { parts } => {
                format!("Version.DotSeparated(parts={parts:?})")
            }
            Self::Other { value } => format!("Version.Other(value={value})"),
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SemVer { major, minor, patch, rc, meta } => {
                write!(f, "v")?;
                write!(f, "{}.", major)?;
                write!(f, "{}.", minor)?;
                write!(f, "{}", patch)?;

                if let Some(rc) = rc {
                    write!(f, "-{}", rc.join("."))?;
                }

                if let Some(meta) = meta {
                    write!(f, "+{}", meta.join("."))?;
                }

                Ok(())
            }
            _ => write!(f, "todo"),
        }
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
                let version_cmp =
                    (major1, minor1, patch1).cmp(&(major2, minor2, patch2));

                if !matches!(version_cmp, Ordering::Equal) {
                    Some(version_cmp)
                } else {
                    // Compare pre-releases.
                    // 1.2.3-alpha is considered a lower version than 1.2.3
                    //
                    // If both pre-releases exist, compare lexicographically
                    match (rc1, rc2) {
                        (None, None) => None,
                        (None, Some(_)) => Some(Ordering::Greater),
                        (Some(_), None) => Some(Ordering::Less),
                        (Some(s1), Some(s2)) => s1.partial_cmp(s2),
                    }
                }
            }
            (Other { value: value1 }, Other { value: value2 }) => {
                value1.partial_cmp(value2)
            }
            _ => None,
        }
    }
}

fn ident<'a>() -> impl Parser<'a, &'a str, char, extra::Err<ParserErrorType<'a>>>
{
    one_of(
        ('0'..='9')
            .chain('a'..='z')
            .chain('A'..='Z')
            .chain(['-'])
            .collect::<String>(),
    )
    .labelled("alphanumeric or '-'")
}

fn dot_sep_idents<'a>()
-> impl Parser<'a, &'a str, Vec<String>, extra::Err<ParserErrorType<'a>>> {
    ident()
        .repeated()
        .at_least(1)
        .collect::<String>()
        .separated_by(just('.').recover_with(skip_then_retry_until(
            any().ignored(),
            one_of(".-+").ignored(),
        )))
        .collect::<Vec<_>>()
        .labelled("dot-separated list")
}

fn int<'a>() -> impl Parser<'a, &'a str, u32, extra::Err<ParserErrorType<'a>>> {
    one_of('0'..='9')
        .labelled("digit")
        .recover_with(skip_then_retry_until(
            any().ignored(),
            one_of(".-+").ignored(),
        ))
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(|s| s.parse::<u32>().unwrap_or(0))
        .labelled("integer")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_version() {
        let test_suite = [
            (
                "1.9.0",
                Version::SemVer {
                    major: 1,
                    minor: 9,
                    patch: 0,
                    rc: None,
                    meta: None,
                },
            ),
            (
                "1.10.0",
                Version::SemVer {
                    major: 1,
                    minor: 10,
                    patch: 0,
                    rc: None,
                    meta: None,
                },
            ),
            (
                "1.11.0",
                Version::SemVer {
                    major: 1,
                    minor: 11,
                    patch: 0,
                    rc: None,
                    meta: None,
                },
            ),
            (
                "1.0.0-alpha",
                Version::SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    rc: Some(vec!["alpha".into()]),
                    meta: None,
                },
            ),
            (
                "1.0.0-alpha.1",
                Version::SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    rc: Some(vec!["alpha".into(), "1".into()]),
                    meta: None,
                },
            ),
            (
                "1.0.0-0.3.7",
                Version::SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    rc: Some(vec!["0".into(), "3".into(), "7".into()]),
                    meta: None,
                },
            ),
            (
                "1.0.0-x.7.z.92",
                Version::SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
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
                    minor: 0,
                    patch: 0,
                    rc: Some(vec!["x-y-z".into(), "--".into()]),
                    meta: None,
                },
            ),
            (
                "1.0.0-alpha+001",
                Version::SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    rc: Some(vec!["alpha".into()]),
                    meta: Some(vec!["001".into()]),
                },
            ),
            (
                "1.0.0+20130313144700",
                Version::SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    rc: None,
                    meta: Some(vec!["20130313144700".into()]),
                },
            ),
            (
                "1.0.0-beta+exp.sha.5114f85",
                Version::SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
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
                    minor: 0,
                    patch: 0,
                    rc: None,
                    meta: Some(vec!["21AF26D3----117B344092BD".into()]),
                },
            ),
        ];

        for (string, version) in test_suite.into_iter() {
            match Version::new(string) {
                Ok(v) => assert_eq!(v, version),
                Err(errs) => todo!(),
            }
        }
    }
}

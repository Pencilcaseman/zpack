use anyhow::{Result, anyhow};
use chumsky::prelude::*;
use pyo3::{exceptions::PyTypeError, prelude::*};

use crate::util::error::{ParserErrorType, ParserErrorWrapper};

/// Semantic Versioning
///
/// For example: 8.4.7-alpha+5d41402a
///
/// See [https://semver.org](https://semver.org) for more information
#[pyclass(str, eq)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SemVer {
    /// Major version
    #[pyo3(get, set)]
    major: u32,

    /// Minor version
    #[pyo3(get, set)]
    minor: u32,

    /// Patch version
    #[pyo3(get, set)]
    patch: u32,

    // Pre-release
    #[pyo3(get, set)]
    rc: Option<Vec<String>>,

    /// Metadata
    #[pyo3(get, set)]
    meta: Option<Vec<String>>,
}

/// Arbitrary dot-separated version
///
/// For example: 2025.06.alpha.3
#[pyclass(str, eq)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DotSeparated {
    #[pyo3(get, set)]
    parts: Vec<String>,
}

/// Any other arbitrary version specifier
///
/// For example: beta+3.4/abc
#[pyclass(str, eq)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Other {
    #[pyo3(get, set)]
    value: String,
}

impl SemVer {
    pub fn new(version: &str) -> Result<Self> {
        Self::parser().parse(version).into_result().map_err(|errs| {
            anyhow!(
                ParserErrorWrapper::new(
                    std::any::type_name::<Self>(),
                    ariadne::Source::from(version),
                    errs,
                )
                .build()
                .unwrap()
                .to_string()
                .unwrap_or_else(|v| v)
            )
        })
    }

    fn parser<'a>()
    -> impl Parser<'a, &'a str, Self, extra::Err<ParserErrorType<'a>>> {
        let core = int().separated_by(just('.')).collect_exactly::<[_; 3]>();
        let pre_release = just('-').ignore_then(dot_sep_idents());
        let metadata = just('+').ignore_then(dot_sep_idents());

        just('v')
            .or_not()
            .ignore_then(core)
            .then(pre_release.or_not())
            .then(metadata.or_not())
            .map(|((version, rc), meta)| Self {
                major: version[0],
                minor: version[1],
                patch: version[2],
                rc,
                meta,
            })
            .then_ignore(end())
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

        if let Some(rc) = &self.rc {
            write!(f, "-{}", rc.join("."))?;
        }

        if let Some(meta) = &self.meta {
            write!(f, "+{}", meta.join("."))?;
        }

        Ok(())
    }
}

impl std::cmp::PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;

        let version_cmp = (self.major, self.minor, self.patch).cmp(&(
            other.major,
            other.minor,
            other.patch,
        ));

        if !matches!(version_cmp, Ordering::Equal) {
            Some(version_cmp)
        } else {
            // Compare pre-releases.
            // 1.2.3-alpha is considered a lower version than 1.2.3
            //
            // If both pre-releases exist, compare lexicographically
            match (&self.rc, &other.rc) {
                (None, None) => Some(Ordering::Equal),
                (None, Some(_)) => Some(Ordering::Greater),
                (Some(_), None) => Some(Ordering::Less),
                (Some(s1), Some(s2)) => s1.partial_cmp(s2),
            }
        }
    }
}

#[pymethods]
impl SemVer {
    #[new]
    pub fn py_new(ver: &str) -> PyResult<Self> {
        Ok(Self::new(ver)?)
    }

    #[staticmethod]
    pub fn from_version(ver: Version) -> PyResult<Self> {
        Self::try_from(ver).map_err(PyTypeError::new_err)
    }

    pub fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

impl DotSeparated {
    fn new(version: &str) -> Result<Self> {
        Self::parser().parse(version).into_result().map_err(|errs| {
            anyhow!(
                ParserErrorWrapper::new(
                    std::any::type_name::<Self>(),
                    ariadne::Source::from(version),
                    errs,
                )
                .build()
                .unwrap()
                .to_string()
                .unwrap_or_else(|v| v)
            )
        })
    }

    fn parser<'a>()
    -> impl Parser<'a, &'a str, Self, extra::Err<ParserErrorType<'a>>> {
        dot_sep_idents().map(|parts| Self { parts }).then_ignore(end())
    }
}

impl std::fmt::Display for DotSeparated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.parts.join("."))
    }
}

impl std::cmp::PartialOrd for DotSeparated {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Compare lexicographically
        self.parts.partial_cmp(&other.parts)
    }
}

#[pymethods]
impl DotSeparated {
    #[new]
    fn py_new(ver: &str) -> PyResult<Self> {
        Ok(Self::new(ver)?)
    }

    pub fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

impl Other {
    fn new(version: &str) -> Result<Self> {
        Self::parser().parse(version).into_result().map_err(|errs| {
            anyhow!(
                ParserErrorWrapper::new(
                    std::any::type_name::<Self>(),
                    ariadne::Source::from(version),
                    errs,
                )
                .build()
                .unwrap()
                .to_string()
                .unwrap_or_else(|v| v)
            )
        })
    }

    fn parser<'a>()
    -> impl Parser<'a, &'a str, Self, extra::Err<ParserErrorType<'a>>> {
        text::ident()
            .map(|value: &str| Self { value: value.to_string() })
            .then_ignore(end())
    }
}

impl std::fmt::Display for Other {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl std::cmp::PartialOrd for Other {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Compare lexicographically
        self.value.partial_cmp(&other.value)
    }
}

#[pymethods]
impl Other {
    #[new]
    fn py_new(ver: &str) -> PyResult<Self> {
        Ok(Self::new(ver)?)
    }

    pub fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[pyclass(str, eq)]
#[derive(Clone, Eq, PartialEq)]
pub enum Version {
    SemVer(SemVer),
    DotSeparated(DotSeparated),
    Other(Other),
}

impl std::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SemVer(v) => write!(f, "Version::{v:?}"),
            Self::DotSeparated(v) => write!(f, "Version::{v:?}"),
            Self::Other(v) => write!(f, "Version::{v:?}"),
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SemVer(v) => write!(f, "{v}"),
            Self::DotSeparated(v) => write!(f, "{v}"),
            Self::Other(v) => write!(f, "{v}"),
        }
    }
}

impl Version {
    pub fn new(version: &str) -> Result<Self> {
        Self::parser().parse(version).into_result().map_err(|errs| {
            anyhow!(
                ParserErrorWrapper::new(
                    std::any::type_name::<Self>(),
                    ariadne::Source::from(version),
                    errs,
                )
                .build()
                .unwrap()
                .to_string()
                .unwrap_or_else(|v| v)
            )
        })
    }

    fn parser<'a>()
    -> impl Parser<'a, &'a str, Self, extra::Err<ParserErrorType<'a>>> {
        {
            choice((
                SemVer::parser().map(Self::SemVer),
                DotSeparated::parser().map(Self::DotSeparated),
                Other::parser().map(Self::Other),
            ))
        }
    }
}

impl From<SemVer> for Version {
    fn from(semver: SemVer) -> Self {
        Self::SemVer(semver)
    }
}

impl From<DotSeparated> for Version {
    fn from(dotsep: DotSeparated) -> Self {
        Self::DotSeparated(dotsep)
    }
}

impl From<Other> for Version {
    fn from(other: Other) -> Self {
        Self::Other(other)
    }
}

impl TryFrom<Version> for SemVer {
    type Error = &'static str;

    fn try_from(value: Version) -> std::result::Result<Self, Self::Error> {
        match value {
            Version::SemVer(v) => Ok(v),
            _ => Err("Cannot convert non-SemVer type to SemVer"),
        }
    }
}

impl TryFrom<Version> for DotSeparated {
    type Error = &'static str;

    fn try_from(value: Version) -> std::result::Result<Self, Self::Error> {
        match value {
            Version::DotSeparated(v) => Ok(v),
            _ => Err("Cannot convert non-DotSeparated type to DotSeparated"),
        }
    }
}

impl TryFrom<Version> for Other {
    type Error = &'static str;

    fn try_from(value: Version) -> std::result::Result<Self, Self::Error> {
        match value {
            Version::Other(v) => Ok(v),
            _ => Err("Cannot convert non-Other type to Other"),
        }
    }
}

#[pymethods]
impl Version {
    #[new]
    pub fn py_new(version: &str) -> PyResult<Self> {
        Ok(Self::new(version)?)
    }

    pub fn __repr__(&self) -> String {
        format!("{self:?}")
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
        .separated_by(just('.'))
        .collect::<Vec<_>>()
        .labelled("dot-separated list")
}

fn int<'a>() -> impl Parser<'a, &'a str, u32, extra::Err<ParserErrorType<'a>>> {
    one_of('0'..='9')
        .labelled("digit")
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
                SemVer { major: 1, minor: 9, patch: 0, rc: None, meta: None },
            ),
            (
                "1.10.0",
                SemVer { major: 1, minor: 10, patch: 0, rc: None, meta: None },
            ),
            (
                "1.11.0",
                SemVer { major: 1, minor: 11, patch: 0, rc: None, meta: None },
            ),
            (
                "1.0.0-alpha",
                SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    rc: Some(vec!["alpha".into()]),
                    meta: None,
                },
            ),
            (
                "1.0.0-alpha.1",
                SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    rc: Some(vec!["alpha".into(), "1".into()]),
                    meta: None,
                },
            ),
            (
                "1.0.0-0.3.7",
                SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    rc: Some(vec!["0".into(), "3".into(), "7".into()]),
                    meta: None,
                },
            ),
            (
                "1.0.0-x.7.z.92",
                SemVer {
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
                SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    rc: Some(vec!["x-y-z".into(), "--".into()]),
                    meta: None,
                },
            ),
            (
                "1.0.0-alpha+001",
                SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    rc: Some(vec!["alpha".into()]),
                    meta: Some(vec!["001".into()]),
                },
            ),
            (
                "1.0.0+20130313144700",
                SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    rc: None,
                    meta: Some(vec!["20130313144700".into()]),
                },
            ),
            (
                "1.0.0-beta+exp.sha.5114f85",
                SemVer {
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
                SemVer {
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
                Ok(v) => assert_eq!(v, version.into()),
                Err(_) => todo!(),
            }
        }
    }
}

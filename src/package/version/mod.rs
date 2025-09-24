pub mod dot_separated;
pub mod other;
pub mod parsers;
pub mod semver;

use anyhow::{Result, anyhow};
use chumsky::prelude::*;
use dot_separated::*;
use other::*;
use pyo3::{
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
    types::{PyString, PyTuple},
};
use semver::*;

use crate::util::error::{ParserErrorType, ParserErrorWrapper};

/// Wrapper around many version types to support arbitrary version usage.
///
/// See [`SemVer`], [`DotSeparated`] and [`Other`] for more information.
#[derive(Clone, PartialEq, PartialOrd)]
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
    /// Construct a new [`Version`] instance
    ///
    /// See other version types for more information on valid input forms.
    ///
    /// * `version`: Version string
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

    /// Create a version parser
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

/// Python wrapper around a [`Version`]
#[pyclass(name = "Version", eq, ord)]
#[derive(Clone, PartialEq, PartialOrd)]
pub struct PyVersion {
    pub inner: Version,
}

#[pymethods]
impl PyVersion {
    /// Construct a new [`PyVersion`] wrapper
    ///
    /// Valid inputs are:
    /// * String version
    /// * [`SemVer`]
    /// * [`DotSeparated`]
    /// * [`Other`]
    /// * Version::SemVer
    /// * Version::DotSeparated
    /// * Version::Other
    #[new]
    #[pyo3(signature = (*args))]
    fn py_new(args: &Bound<'_, PyTuple>) -> PyResult<Self> {
        if args.len() != 1 {
            return Err(PyValueError::new_err("Other() requires arguments"));
        }

        let arg0 = args.get_item(0)?;

        if let Ok(s) = arg0.downcast::<PyString>() {
            Ok(Self { inner: Version::new(s.to_str()?)? })
        } else if let Ok(semver) = arg0.extract::<PyRef<PySemVer>>() {
            Ok(Self { inner: Version::SemVer((*semver).inner.clone()) })
        } else if let Ok(dotsep) = arg0.extract::<PyRef<PyDotSeparated>>() {
            Ok(Self { inner: Version::DotSeparated((*dotsep).inner.clone()) })
        } else if let Ok(other) = arg0.extract::<PyRef<PyOther>>() {
            Ok(Self { inner: Version::Other((*other).inner.clone()) })
        } else if let Ok(version) = arg0.extract::<PyRef<PyVersion>>() {
            Ok(Self { inner: (*version).inner.clone() })
        } else {
            Err(PyTypeError::new_err(format!(
                "Cannot construct Other from type '{}'",
                arg0.get_type().name()?
            )))
        }
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    pub fn __str__(&self) -> String {
        format!("{}", self.inner)
    }
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

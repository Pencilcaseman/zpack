use std::collections::HashSet;

use anyhow::{Result, anyhow};
use chumsky::prelude::*;
use pyo3::{
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
    types::{PyDict, PyString, PyTuple},
};

use super::parsers::*;
use crate::{
    package::version::{PyVersion, Version},
    util::error::{ParserErrorType, ParserErrorWrapper},
};

/// Semantic Versioning
///
/// For example: 8.4.7-alpha+5d41402a
///
/// See [https://semver.org](https://semver.org) for more information
#[derive(Debug, Clone)]
pub struct SemVer {
    /// Major version
    pub major: u32,

    /// Minor version
    pub minor: u32,

    /// Patch version
    pub patch: u32,

    // Pre-release
    pub rc: Option<Vec<String>>,

    /// Metadata
    pub meta: Option<Vec<String>>,
}

impl SemVer {
    /// Construct a new [`SemVer`] instance from a version string
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

    /// Build a [`SemVer`] parser
    pub fn parser<'a>()
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

impl std::cmp::PartialEq for SemVer {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major
            && self.minor == other.minor
            && self.patch == other.patch
            && self.rc == other.rc
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

/// Python wrapper type for [`SemVer`]
#[pyclass(name = "SemVer", eq, ord)]
#[derive(Clone, PartialEq, PartialOrd)]
pub struct PySemVer {
    pub inner: SemVer,
}

#[pymethods]
impl PySemVer {
    /// Construct a new [`PySemVer`] wrapper
    ///
    /// Valid inputs are:
    /// * String version
    /// * SemVer
    /// * Version::SemVer(...)
    /// * major=..., minor=..., patch=..., [rc=...,] [meta=...]
    #[new]
    #[pyo3(signature = (*args, **kwargs))]
    fn py_new(
        args: &Bound<'_, PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        // *args
        if !args.is_empty() {
            if kwargs.is_some() {
                return Err(PyTypeError::new_err(
                    "Constructor expects *args or **kwargs, but not both",
                ));
            }

            match args.len() {
                1 => {
                    // String, SemVer or Version
                    let arg0 = args.get_item(0)?;

                    if let Ok(s) = arg0.downcast::<PyString>() {
                        // String
                        return Ok(Self { inner: SemVer::new(s.to_str()?)? });
                    } else if let Ok(other) = arg0.extract::<PyRef<Self>>() {
                        // PySemVer copy
                        return Ok(Self { inner: (*other).inner.clone() });
                    } else if let Ok(version) =
                        arg0.extract::<PyRef<PyVersion>>()
                    {
                        // PyVersion
                        if let Version::SemVer(semver) = &(*version).inner {
                            return Ok(Self { inner: semver.clone() });
                        }

                        return Err(PyTypeError::new_err(format!(
                            "Expected SemVer; found {}",
                            version.__repr__()
                        )));
                    } else {
                        return Err(PyTypeError::new_err(format!(
                            "Cannot construct SemVer from type '{}'",
                            arg0.get_type().name()?
                        )));
                    }
                }
                _ => todo!(),
            }
        }

        // **kwargs
        if let Some(kwargs) = kwargs {
            let major: u32 = kwargs
                .get_item("major")?
                .ok_or_else(|| {
                    PyTypeError::new_err(
                        "Missing required keyword argument 'major'",
                    )
                })?
                .extract()?;

            let minor: u32 = kwargs
                .get_item("minor")?
                .ok_or_else(|| {
                    PyTypeError::new_err(
                        "Missing required keyword argument 'minor'",
                    )
                })?
                .extract()?;

            let patch: u32 = kwargs
                .get_item("patch")?
                .ok_or_else(|| {
                    PyTypeError::new_err(
                        "Missing required keyword argument 'patch'",
                    )
                })?
                .extract()?;

            let rc: Option<Vec<String>> = match kwargs.get_item("rc")? {
                Some(item) => item.extract()?,
                None => None,
            };

            let meta: Option<Vec<String>> = match kwargs.get_item("meta")? {
                Some(item) => item.extract()?,
                None => None,
            };

            // Ensure no extra keys
            let expected_keys: HashSet<&str> =
                ["major", "minor", "patch", "rc", "meta"]
                    .iter()
                    .cloned()
                    .collect();

            for key_obj in kwargs.keys() {
                let key_str: &str = key_obj.extract()?;

                if !expected_keys.contains(key_str) {
                    return Err(PyValueError::new_err(format!(
                        "'{}' is an invalid keyword argument for SemVer()",
                        key_str
                    )));
                }
            }

            return Ok(Self {
                inner: SemVer { major, minor, patch, rc, meta },
            });
        }

        Err(PyValueError::new_err("SemVer() requires arguments"))
    }

    #[getter]
    pub fn get_major(&self) -> u32 {
        self.inner.major
    }

    #[setter]
    pub fn set_major(&mut self, new_major: u32) {
        self.inner.major = new_major;
    }

    #[getter]
    pub fn get_minor(&self) -> u32 {
        self.inner.minor
    }

    #[setter]
    pub fn set_minor(&mut self, new_minor: u32) {
        self.inner.minor = new_minor;
    }

    #[getter]
    pub fn get_patch(&self) -> u32 {
        self.inner.patch
    }

    #[setter]
    pub fn set_patch(&mut self, new_patch: u32) {
        self.inner.patch = new_patch;
    }

    #[getter]
    pub fn get_rc(&self) -> &Option<Vec<String>> {
        &self.inner.rc
    }

    #[setter]
    pub fn set_rc(&mut self, new_rc: Option<Vec<String>>) {
        self.inner.rc = new_rc;
    }

    #[getter]
    pub fn get_meta(&self) -> &Option<Vec<String>> {
        &self.inner.meta
    }

    #[setter]
    pub fn set_meta(&mut self, new_meta: Option<Vec<String>>) {
        self.inner.meta = new_meta;
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    pub fn __str__(&self) -> String {
        format!("{}", self.inner)
    }
}

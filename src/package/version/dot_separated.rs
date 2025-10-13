use anyhow::{Result, anyhow};
use chumsky::prelude::*;
use pyo3::{
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
    types::{PyDict, PyString, PyTuple},
};

use super::parsers::*;
use crate::{
    package::version::Version,
    util::error::{ParserErrorType, ParserErrorWrapper},
};

/// Arbitrary dot-separated version
///
/// For example: 2025.06.alpha.3
#[pyclass]
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DotSeparated {
    /// Components of the version separated by '.'
    #[pyo3(get, set)]
    pub parts: Vec<String>,
}

impl DotSeparated {
    /// Create a new [`DotSeparated`] instance
    ///
    /// For example: `1.2.3.alpha.2025`
    ///
    /// * `version`: Versions string
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

    /// Parser for a DotSeparated version
    pub fn parser<'a>()
    -> impl Parser<'a, &'a str, Self, extra::Err<ParserErrorType<'a>>> {
        dot_sep_idents().map(|parts| Self { parts }).then_ignore(end())
    }
}

impl std::fmt::Display for DotSeparated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.parts.join("."))
    }
}

// /// Python wrapper around [`DotSeparated`]
// #[pyclass(name = "DotSeparated", eq, ord)]
// #[derive(Clone, PartialEq, PartialOrd)]
// pub struct PyDotSeparated {
//     pub inner: DotSeparated,
// }

#[pymethods]
impl DotSeparated {
    /// Construct a new [`PyDotSeparated`] wrapper
    ///
    /// Valid inputs are:
    /// * String version
    /// * DotSeparated
    /// * Version::DotSeparated(...)
    /// * parts=...
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
                    // String, DotSeparated or Version
                    let arg0 = args.get_item(0)?;

                    if let Ok(s) = arg0.downcast::<PyString>() {
                        // String
                        return Ok(Self::new(s.to_str()?)?);
                    } else if let Ok(other) = arg0.extract::<PyRef<Self>>() {
                        // DotSeparated copy
                        return Ok((*other).clone());
                    } else if let Ok(version) = arg0.extract::<PyRef<Version>>()
                    {
                        // PyVersion
                        if let Version::DotSeparated(dotsep) = &(*version) {
                            return Ok(dotsep.clone());
                        }

                        return Err(PyTypeError::new_err(format!(
                            "Expected DotSeparated; found {}",
                            version.__repr__()
                        )));
                    } else {
                        return Err(PyTypeError::new_err(format!(
                            "Cannot construct DotSeparated from type '{}'",
                            arg0.get_type().name()?
                        )));
                    }
                }
                _ => todo!(),
            }
        }

        // **kwargs
        if let Some(kwargs) = kwargs {
            let parts: Vec<String> = match kwargs.get_item("parts")? {
                Some(item) => item.extract()?,
                None => {
                    return Err(PyValueError::new_err(
                        "invalid input to argument 'parts'",
                    ));
                }
            };

            for key_obj in kwargs.keys() {
                let key_str: &str = key_obj.extract()?;

                if key_str != "parts" {
                    return Err(PyValueError::new_err(format!(
                        "'{}' is an invalid keyword argument for DotSeparated()",
                        key_str
                    )));
                }
            }

            return Ok(Self { parts });
        }

        Err(PyValueError::new_err("DotSeparated() requires arguments"))
    }

    #[getter]
    pub fn get_parts(&self) -> &[String] {
        &self.parts
    }

    #[setter]
    pub fn set_parts(&mut self, new_parts: Vec<String>) {
        self.parts = new_parts
    }

    pub fn __repr__(&self) -> String {
        format!("{self:?}")
    }

    pub fn __str__(&self) -> String {
        format!("{self}")
    }
}

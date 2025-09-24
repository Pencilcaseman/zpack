use anyhow::{Result, anyhow};
use chumsky::prelude::*;
use pyo3::{
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
    types::{PyDict, PyString, PyTuple},
};

use crate::{
    package::version::{PyVersion, Version},
    util::error::{ParserErrorType, ParserErrorWrapper},
};

/// Any other arbitrary version specifier
///
/// For example: beta+3.4/abc
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Other {
    /// The version string
    pub value: String,
}

impl Other {
    /// Create a new [`Other`] version instance
    ///
    /// This is an arbitrary string version.
    ///
    /// * `version`: String version
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

    /// [`Other`] version parser
    pub fn parser<'a>()
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

/// Python wrapper around the [`Other`] version type
#[pyclass(name = "Other", eq, ord)]
#[derive(Clone, PartialEq, PartialOrd)]
pub struct PyOther {
    pub inner: Other,
}

#[pymethods]
impl PyOther {
    /// Construct a new [`PyOther`] wrapper
    ///
    /// Valid inputs are:
    /// * String version
    /// * Other
    /// * Version::Other(...)
    /// * value=...
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
                    // String, Other or Version
                    let arg0 = args.get_item(0)?;

                    if let Ok(s) = arg0.downcast::<PyString>() {
                        // String
                        return Ok(Self { inner: Other::new(s.to_str()?)? });
                    } else if let Ok(other) = arg0.extract::<PyRef<Self>>() {
                        // PyOther copy
                        return Ok(Self { inner: (*other).inner.clone() });
                    } else if let Ok(version) =
                        arg0.extract::<PyRef<PyVersion>>()
                    {
                        // PyVersion
                        if let Version::Other(other) = &(*version).inner {
                            return Ok(Self { inner: other.clone() });
                        }

                        return Err(PyTypeError::new_err(format!(
                            "Expected Other; found {}",
                            version.__repr__()
                        )));
                    } else {
                        return Err(PyTypeError::new_err(format!(
                            "Cannot construct Other from type '{}'",
                            arg0.get_type().name()?
                        )));
                    }
                }
                _ => todo!(),
            }
        }

        // **kwargs
        if let Some(kwargs) = kwargs {
            let value: String = match kwargs.get_item("value")? {
                Some(item) => item.extract()?,
                None => {
                    return Err(PyValueError::new_err(
                        "invalid input to argument 'value'",
                    ));
                }
            };

            for key_obj in kwargs.keys() {
                let key_str: &str = key_obj.extract()?;

                if key_str != "parts" {
                    return Err(PyValueError::new_err(format!(
                        "'{}' is an invalid keyword argument for Other()",
                        key_str
                    )));
                }
            }

            return Ok(Self { inner: Other { value } });
        }

        Err(PyValueError::new_err("Other() requires arguments"))
    }

    #[getter]
    pub fn get_value(&self) -> &str {
        &self.inner.value
    }

    #[setter]
    pub fn set_parts(&mut self, new_value: String) {
        self.inner.value = new_value;
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    pub fn __str__(&self) -> String {
        format!("{}", self.inner)
    }
}

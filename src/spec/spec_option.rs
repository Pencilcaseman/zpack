use std::{hash::Hash, str::FromStr};

use pyo3::{IntoPyObjectExt, exceptions::PyTypeError, prelude::*};

use crate::package::{self, Version};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SpecOptionType {
    Bool,
    Int,
    Float,
    Str,
    Version,
    // List, // TODO: How best to handle this?
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpecOptionValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Version(Version),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct SpecOption {
    pub value: Option<SpecOptionValue>,
    pub default: Option<SpecOptionValue>,
    pub valid: Option<Vec<SpecOptionValue>>,
}

impl SpecOptionValue {
    /// Map a spec value to a spec type.
    ///
    /// This is commonly used for validation
    pub fn to_type(&self) -> SpecOptionType {
        match self {
            Self::Bool(_) => SpecOptionType::Bool,
            Self::Int(_) => SpecOptionType::Int,
            Self::Float(_) => SpecOptionType::Float,
            Self::Str(_) => SpecOptionType::Str,
            Self::Version(_) => SpecOptionType::Version,
        }
    }

    /// Compare a spec value to a spec type.
    ///
    /// * `t`: The type to compare against
    pub fn is_type(&self, t: SpecOptionType) -> bool {
        self.to_type() == t
    }

    /// Convert this value into a [`z3::ast::Dynamic`] value.
    ///
    /// The dynamic type of the returned value matches the enum variant held by
    /// [`Self`]
    pub fn to_z3_dynamic(
        &self,
        registry: &package::BuiltRegistry,
    ) -> z3::ast::Dynamic {
        use z3::ast::{Bool, Float, Int, String};

        match self {
            Self::Bool(b) => Bool::from_bool(*b).into(),
            Self::Int(i) => Int::from_i64(*i).into(),
            Self::Float(f) => Float::from_f64(*f).into(),
            Self::Str(s) => String::from_str(s).unwrap().into(),
            Self::Version(v) => {
                let idx = match registry.version_registry().lookup(v) {
                    Some(found) => found as u64,
                    None => {
                        tracing::error!(
                            "encountered version not in Registry: {v}"
                        );
                        u64::MAX
                    }
                };

                Int::from_u64(idx).into()
            }
        }
    }

    pub fn from_z3_dynamic(
        dtype: SpecOptionType,
        dynamic: &z3::ast::Dynamic,
        registry: &package::BuiltRegistry,
    ) -> Self {
        match dtype {
            SpecOptionType::Bool => {
                Self::Bool(dynamic.as_bool().unwrap().as_bool().unwrap())
            }
            SpecOptionType::Int => {
                Self::Int(dynamic.as_int().unwrap().as_i64().unwrap())
            }
            SpecOptionType::Float => {
                Self::Float(dynamic.as_float().unwrap().as_f64())
            }
            SpecOptionType::Str => {
                Self::Str(dynamic.as_string().unwrap().as_string().unwrap())
            }
            SpecOptionType::Version => {
                let idx = dynamic.as_int().unwrap().as_u64().unwrap();
                let v = registry
                    .version_registry()
                    .lookup_id(&usize::try_from(idx).unwrap())
                    .unwrap();
                Self::Version(v.clone())
            }
        }
    }
}

impl Hash for SpecOptionValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl std::cmp::Eq for SpecOptionValue {}

impl std::fmt::Display for SpecOptionValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(v) => write!(f, "{v}"),
            Self::Int(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Str(v) => write!(f, "{v}"),
            Self::Version(v) => write!(f, "{v}"),
        }
    }
}

impl SpecOption {
    /// Construct a type descriptor instance of a [`SpecOption`]
    ///
    /// * `t`: The datatype of this option
    pub fn new_from_type(t: SpecOptionType) -> Self {
        Self { value: None, default: None, valid: None }
    }

    pub fn serialize_name(&self, package: &str, name: &str) -> String {
        format!("{}/{}", package, name)
    }

    pub fn to_z3_dynamic<'a>(
        &self,
        package: &'a str,
        name: &'a str,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> z3::ast::Dynamic {
        use z3::ast::{Bool, Float, Int, String};

        let n = self.serialize_name(package, name);

        let Some(idx) = wip_registry.lookup_option(package, Some(name)) else {
            let msg = format!("no datatype set for {package}:{name}");
            tracing::error!("{msg}");
            panic!("{msg}");
        };

        match wip_registry.spec_options()[idx].0 {
            SpecOptionType::Bool => Bool::new_const(n).into(),
            SpecOptionType::Int => Int::new_const(n).into(),
            SpecOptionType::Float => Float::new_const_double(n).into(),
            SpecOptionType::Str => String::new_const(n).into(),
            SpecOptionType::Version => {
                if let Some(value) = &self.value {
                    let SpecOptionValue::Version(v) = value else {
                        let msg = "value and dtype are inconsistent; this is an internal error";
                        tracing::error!("{msg}");
                        panic!("{msg}");
                    };

                    wip_registry.version_registry_mut().push(v.clone());
                }

                Int::new_const(n).into()
            }
        }
    }
}

impl<'py> FromPyObject<'py> for SpecOptionValue {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(b) = ob.extract::<bool>() {
            Ok(Self::Bool(b))
        } else if let Ok(i) = ob.extract::<i64>() {
            Ok(Self::Int(i))
        } else if let Ok(f) = ob.extract::<f64>() {
            Ok(Self::Float(f))
        } else if let Ok(s) = ob.extract::<&str>() {
            Ok(Self::Str(s.to_string()))
        } else if let Ok(v) = ob.extract::<Version>() {
            Ok(Self::Version(v))
        } else {
            let msg = format!(
                "cannot cast Python type '{}' to SpecOptionValue",
                ob.get_type()
            );

            tracing::error!("{msg}");
            Err(PyTypeError::new_err(msg))
        }
    }
}

impl<'py> IntoPyObject<'py> for SpecOptionValue {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(
        self,
        py: Python<'py>,
    ) -> Result<Self::Output, Self::Error> {
        match self {
            Self::Bool(b) => Ok(b.into_bound_py_any(py)?),
            Self::Int(i) => Ok(i.into_bound_py_any(py)?),
            Self::Float(f) => Ok(f.into_bound_py_any(py)?),
            Self::Str(s) => Ok(s.into_bound_py_any(py)?),
            Self::Version(v) => Ok(v.into_bound_py_any(py)?),
        }
    }
}

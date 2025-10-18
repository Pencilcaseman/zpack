use std::{hash::Hash, str::FromStr};

use pyo3::{IntoPyObjectExt, exceptions::PyTypeError, prelude::*};

use crate::package::{
    registry::{Registry, WipRegistry},
    version,
};

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
    Version(version::Version),
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
    pub fn to_z3_dynamic(&self, registry: &Registry) -> z3::ast::Dynamic {
        use z3::ast::{Bool, Float, Int, String};

        match self {
            Self::Bool(b) => Bool::from_bool(*b).into(),
            Self::Int(i) => Int::from_i64(*i).into(),
            Self::Float(f) => Float::from_f64(*f).into(),
            Self::Str(s) => String::from_str(s).unwrap().into(),
            Self::Version(v) => {
                let idx = match registry.versions.lookup(v) {
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
}

impl Hash for SpecOptionValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl std::cmp::Eq for SpecOptionValue {}

impl SpecOption {
    /// Construct a type descriptor instance of a [`SpecOption`]
    ///
    /// * `t`: The datatype of this option
    pub fn new_from_type(t: SpecOptionType) -> Self {
        // Self { dtype: Some(t), value: None, default: None, valid: None }
        Self { value: None, default: None, valid: None }
    }

    pub fn serialize_name(&self, package: &str, name: &str) -> String {
        format!("{}/{}", package, name)
    }

    pub fn to_z3_dynamic(
        &self,
        package: &str,
        name: &str,
        wip_registry: &mut WipRegistry,
    ) -> z3::ast::Dynamic {
        let n = self.serialize_name(package, name);

        use z3::ast::{Bool, Float, Int, String};

        match wip_registry.option_type_map.get(&(package, Some(name))) {
            Some(SpecOptionType::Bool) => Bool::new_const(n).into(),
            Some(SpecOptionType::Int) => Int::new_const(n).into(),
            Some(SpecOptionType::Float) => Float::new_const_double(n).into(),
            Some(SpecOptionType::Str) => String::new_const(n).into(),
            Some(SpecOptionType::Version) => {
                tracing::error!("DEBUG POINT: {self:?}");

                if let Some(value) = &self.value {
                    let SpecOptionValue::Version(v) = value else {
                        let msg = "value and dtype are inconsistent; this is an internal error";
                        tracing::error!("{msg}");
                        panic!("{msg}");
                    };

                    wip_registry.versions.push(v.clone());
                }

                Int::new_const(n).into()
            }
            None => {
                let msg = format!("no datatype set for {package}:{name}");
                tracing::error!("{msg}");
                panic!("{msg}");
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
        } else if let Ok(v) = ob.extract::<version::Version>() {
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

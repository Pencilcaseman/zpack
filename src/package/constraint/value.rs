use std::collections::HashSet;

use pyo3::{
    IntoPyObjectExt, basic::CompareOp, exceptions::PyNotImplementedError,
    prelude::*,
};

use crate::{
    package::{
        self,
        constraint::{Cmp, CmpType, Constraint, ConstraintUtils},
        outline::SolverError,
    },
    spec::{self, SpecOptionValue},
};

#[pyclass]
#[derive(Clone, Debug)]
pub struct Value {
    #[pyo3(get, set)]
    pub value: SpecOptionValue,
}

impl ConstraintUtils for Value {
    fn get_value_type<'a, V>(
        &'a self,
        _registry: Option<&package::registry::Registry<'a, V>>,
    ) -> Option<spec::SpecOptionType> {
        Some(self.value.to_type())
    }

    fn set_value_type<'a>(
        &'a self,
        _wip_registry: &mut package::WipRegistry<'a>,
        _value_type: spec::SpecOptionType,
    ) {
        tracing::error!("Cannot change datatype of Value constraint")
    }

    fn type_check<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        match &self.value {
            SpecOptionValue::Bool(_)
            | SpecOptionValue::Int(_)
            | SpecOptionValue::Float(_)
            | SpecOptionValue::Str(_) => (),
            SpecOptionValue::Version(version) => {
                wip_registry.version_registry_mut().push(version.clone());
            }
        }

        Ok(())
    }

    fn extract_spec_options(&self) -> Vec<(&str, &str, spec::SpecOption)> {
        Vec::new()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        HashSet::default()
    }

    fn cmp_to_z3<'a>(
        &self,
        _other: &Constraint,
        _op: CmpType,
        _registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        todo!()
    }

    fn to_z3_clauses<'a>(
        &self,
        registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<Vec<z3::ast::Dynamic>, Box<SolverError>> {
        Ok(self.value.to_z3_dynamic(registry))
    }

    fn to_python_any<'py>(
        &self,
        py: pyo3::Python<'py>,
    ) -> pyo3::PyResult<pyo3::Bound<'py, pyo3::PyAny>> {
        self.clone().into_bound_py_any(py)
    }
}

impl From<Value> for Constraint {
    fn from(val: Value) -> Self {
        Constraint::Value(Box::new(val))
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[pymethods]
impl Value {
    fn __richcmp__(
        &self,
        rhs: Constraint,
        op: CompareOp,
    ) -> Result<Constraint, PyErr> {
        Cmp::py_richcmp_helper(self.clone().into(), rhs.clone(), op.into())
    }
}

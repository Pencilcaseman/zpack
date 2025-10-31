use std::collections::HashSet;

use pyo3::{IntoPyObjectExt, prelude::*};

use crate::{
    package::{
        self,
        constraint::{CmpType, Constraint, ConstraintType, ConstraintUtils},
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
    fn get_type<'a>(
        &'a self,
        _registry: &package::BuiltRegistry<'a>,
    ) -> ConstraintType {
        ConstraintType::Value(self.value.to_type())
    }

    fn try_get_type<'a>(
        &'a self,
        _wip_registry: &mut package::WipRegistry<'a>,
    ) -> Option<ConstraintType> {
        Some(ConstraintType::Value(self.value.to_type()))
    }

    fn set_type<'a>(
        &'a self,
        _wip_registry: &mut package::WipRegistry<'a>,
        _constraint_type: ConstraintType,
    ) {
        tracing::error!("Cannot change datatype of constraint type Value");
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
        other: &Constraint,
        op: CmpType,
        registry: &package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        todo!()
    }

    fn to_z3_clause<'a>(
        &self,
        registry: &package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        Ok(self.value.to_z3_dynamic(registry))
    }

    fn to_python_any<'py>(
        &self,
        py: pyo3::Python<'py>,
    ) -> pyo3::PyResult<pyo3::Bound<'py, pyo3::PyAny>> {
        self.clone().into_bound_py_any(py)
    }
}

impl Into<Constraint> for Value {
    fn into(self) -> Constraint {
        Constraint::Value(Box::new(self))
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

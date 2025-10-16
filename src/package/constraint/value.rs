use std::collections::{HashMap, HashSet};

use pyo3::{IntoPyObjectExt, prelude::*};

use crate::{
    package::{
        constraint::Constraint, outline::SolverError, registry::Registry,
    },
    spec,
    spec::SpecOptionValue,
};

#[pyclass]
#[derive(Clone, Debug)]
pub struct Value {
    #[pyo3(get, set)]
    pub value: SpecOptionValue,
}

impl Constraint for Value {
    fn extract_spec_options(
        &self,
        _package: &str,
    ) -> Vec<(&str, spec::SpecOption)> {
        Vec::new()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        HashSet::default()
    }

    fn get_type(&self) -> Option<super::ConstraintType> {
        todo!()
    }

    fn to_z3_clause<'a>(
        &self,
        _package: &str,
        registry: &Registry<'a>,
    ) -> Result<z3::ast::Dynamic, SolverError> {
        Ok(self.value.to_z3_dynamic(registry))
    }

    fn to_python_any<'py>(
        &self,
        py: pyo3::Python<'py>,
    ) -> pyo3::PyResult<pyo3::Bound<'py, pyo3::PyAny>> {
        self.clone().into_bound_py_any(py)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

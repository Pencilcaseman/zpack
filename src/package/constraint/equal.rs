use std::{any::Any, collections::HashSet};

use pyo3::{IntoPyObjectExt, prelude::*};

use crate::{
    package::{
        constraint::{Constraint, ConstraintType},
        outline::SolverError,
        registry::Registry,
    },
    spec,
};

#[pyclass]
#[derive(Debug, Clone)]
pub struct Equal {
    #[pyo3(get, set)]
    pub lhs: Box<dyn Constraint>,

    #[pyo3(get, set)]
    pub rhs: Box<dyn Constraint>,
}

impl Constraint for Equal {
    fn extract_spec_options(
        &self,
        package: &str,
    ) -> Vec<(&str, spec::SpecOption)> {
        let mut res = Vec::new();
        res.extend(self.lhs.extract_spec_options(package));
        res.extend(self.rhs.extract_spec_options(package));
        res
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        Default::default()
    }

    fn get_type(&self) -> Option<super::ConstraintType> {
        Some(ConstraintType::Equal)
    }

    fn to_z3_clause<'a>(
        &self,
        package: &str,
        registry: &Registry<'a>,
    ) -> Result<z3::ast::Dynamic, SolverError> {
        Ok(self
            .lhs
            .to_z3_clause(package, registry)?
            .eq(self.rhs.to_z3_clause(package, registry)?)
            .into())
    }

    fn to_python_any<'py>(
        &self,
        py: pyo3::Python<'py>,
    ) -> pyo3::PyResult<pyo3::Bound<'py, pyo3::PyAny>> {
        self.clone().into_bound_py_any(py)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

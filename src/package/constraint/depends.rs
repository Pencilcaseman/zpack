use std::collections::HashSet;

use pyo3::{IntoPyObjectExt, prelude::*};

use super::ConstraintUtils;
use crate::package::{
    self,
    constraint::{Constraint, ConstraintType},
    outline::SolverError,
};

#[pyclass]
#[derive(Clone, Debug)]
pub struct Depends {
    #[pyo3(get, set)]
    on: String,
}

impl Depends {
    pub fn new(on: String) -> Self {
        Self { on }
    }
}

impl ConstraintUtils for Depends {
    fn get_type(&self, _registry: &package::BuiltRegistry) -> ConstraintType {
        ConstraintType::Depends
    }

    fn try_get_type<'a>(
        &'a self,
        _wip_registry: &mut package::WipRegistry<'a>,
    ) -> Option<ConstraintType> {
        Some(ConstraintType::Depends)
    }

    fn set_type<'a>(
        &'a self,
        _wip_registry: &mut package::WipRegistry<'a>,
        _constraint_type: ConstraintType,
    ) {
        // Nothing to set
        tracing::warn!("attempting to set type of Depends. This does nothing");
    }

    fn type_check<'a>(
        &self,
        _wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        // Nothing to type-check
        Ok(())
    }

    fn extract_spec_options(
        &self,
    ) -> Vec<(&str, &str, crate::spec::SpecOption)> {
        Vec::new()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        HashSet::from([self.on.clone()])
    }

    fn to_z3_clauses<'a>(
        &self,
        registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<Vec<z3::ast::Dynamic>, Box<SolverError>> {
        let Some(idx) = registry.lookup_option(&self.on, None) else {
            tracing::error!("package '{}' has no activation variable", self.on);

            return Err(Box::new(SolverError::MissingPackage {
                dep: self.on.clone(),
            }));
        };

        let Some(dynamic) = &registry.spec_options()[idx].1 else {
            tracing::error!(
                "activation variable for package '{}' has not been initialized in the solver",
                self.on
            );

            panic!();
        };

        Ok(vec![dynamic.clone()])
    }

    fn to_python_any<'py>(
        &self,
        py: Python<'py>,
    ) -> pyo3::PyResult<pyo3::Bound<'py, pyo3::PyAny>> {
        self.clone().into_bound_py_any(py)
    }
}

#[pymethods]
impl Depends {
    #[new]
    pub fn py_new(name: String) -> PyResult<Self> {
        Ok(Self::new(name))
    }
}

impl From<Depends> for Constraint {
    fn from(val: Depends) -> Self {
        Constraint::Depends(Box::new(val))
    }
}

impl std::fmt::Display for Depends {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Depends( {} )", self.on)
    }
}

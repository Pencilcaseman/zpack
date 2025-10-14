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

    fn propagate_types(
        &mut self,
        required: Option<ConstraintType>,
    ) -> Result<(), SolverError> {
        if let Some(t) = required
            && t != ConstraintType::Equal
        {
            tracing::error!("expected Equal, found {t:?}");

            return Err(SolverError::IncorrectConstraintType {
                expected: ConstraintType::Equal,
                received: t,
            });
        }

        match (self.lhs.get_type(), self.rhs.get_type()) {
            (None, Some(r_type)) => self.lhs.propagate_types(Some(r_type)),
            (Some(l_type), None) => self.rhs.propagate_types(Some(l_type)),
            (Some(l_type), Some(r_type)) if l_type != r_type => {
                tracing::error!("cannot compare {l_type:?} and {r_type:?}");
                Err(SolverError::IncorrectConstraintType {
                    expected: l_type,
                    received: r_type,
                })
            }
            _ => Ok(()),
        }
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

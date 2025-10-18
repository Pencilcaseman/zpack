use std::{any::Any, collections::HashSet};

use pyo3::{IntoPyObjectExt, prelude::*};
use tracing::instrument;

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
    fn get_type<'a>(
        &'a self,
        _wip_registry: &mut crate::package::registry::WipRegistry<'a>,
    ) -> Option<ConstraintType> {
        Some(ConstraintType::Equal)
    }

    #[instrument]
    fn set_type<'a>(
        &'a self,
        _wip_registry: &mut crate::package::registry::WipRegistry<'a>,
        _constraint_type: ConstraintType,
    ) {
        tracing::warn!("attempting to set type of Equal. This does nothing");
    }

    #[instrument]
    fn type_check<'a>(
        &'a self,
        wip_registry: &mut crate::package::registry::WipRegistry<'a>,
    ) -> Result<(), SolverError> {
        // Types must be the same
        // Propagate types from known to unknown

        let lhs_type = self.lhs.get_type(wip_registry);
        let rhs_type = self.rhs.get_type(wip_registry);

        let res = match (lhs_type, rhs_type) {
            (None, None) => Ok(()),

            (None, Some(rhs)) => {
                self.lhs.set_type(wip_registry, rhs);
                Ok(())
            }

            (Some(lhs), None) => {
                self.rhs.set_type(wip_registry, lhs);
                Ok(())
            }

            (Some(lhs), Some(rhs)) => {
                if lhs != rhs {
                    tracing::error!(
                        "cannot compare differing types {lhs:?} and {rhs:?}"
                    );

                    Err(SolverError::IncorrectConstraintType {
                        expected: lhs,
                        received: rhs,
                    })
                } else {
                    Ok(())
                }
            }
        };

        // Continue type checking
        self.lhs.type_check(wip_registry)?;
        self.rhs.type_check(wip_registry)?;

        res
    }

    #[instrument]
    fn extract_spec_options(&self) -> Vec<(&str, &str, spec::SpecOption)> {
        tracing::info!("extracting spec options");

        let mut res = Vec::new();
        res.extend(self.lhs.extract_spec_options());
        res.extend(self.rhs.extract_spec_options());
        res
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        Default::default()
    }

    fn to_z3_clause<'a>(
        &self,
        registry: &Registry<'a>,
    ) -> Result<z3::ast::Dynamic, SolverError> {
        Ok(self
            .lhs
            .to_z3_clause(registry)?
            .eq(self.rhs.to_z3_clause(registry)?)
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

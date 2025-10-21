use std::{any::Any, collections::HashSet};

use pyo3::{IntoPyObjectExt, prelude::*};

use crate::{
    package::{
        self,
        constraint::{Constraint, ConstraintType},
        outline::SolverError,
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
        _wip_registry: &mut package::WipRegistry<'a>,
    ) -> Option<ConstraintType> {
        Some(ConstraintType::Equal)
    }

    fn set_type<'a>(
        &'a self,
        _wip_registry: &mut package::WipRegistry<'a>,
        _constraint_type: ConstraintType,
    ) {
        tracing::warn!("attempting to set type of Equal. This does nothing");
    }

    #[tracing::instrument(skip(self, wip_registry))]
    fn type_check<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        // Types must be the same
        // Propagate types from known to unknown

        let lhs_type = self.lhs.get_type(wip_registry);
        let rhs_type = self.rhs.get_type(wip_registry);

        match (lhs_type, rhs_type) {
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
        }?;

        // Continue type checking
        self.lhs.type_check(wip_registry)?;
        self.rhs.type_check(wip_registry)?;

        Ok(())
    }

    fn extract_spec_options(&self) -> Vec<(&str, &str, spec::SpecOption)> {
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
        registry: &package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
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

impl std::fmt::Display for Equal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ {} ] == [ {} ]", self.lhs, self.rhs)
    }
}

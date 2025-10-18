use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use pyo3::{IntoPyObjectExt, prelude::*};
use tracing::instrument;
use z3::SortKind;

use super::Constraint;
use crate::{
    package::{
        constraint::ConstraintType, outline::SolverError, registry::Registry,
    },
    spec,
};

#[pyclass(unsendable)]
#[derive(Clone, Debug)]
pub struct IfThen {
    #[pyo3(get, set)]
    pub cond: Box<dyn Constraint>,

    #[pyo3(get, set)]
    pub then: Box<dyn Constraint>,
}

impl Constraint for IfThen {
    fn get_type<'a>(
        &'a self,
        _wip_registry: &mut crate::package::registry::WipRegistry<'a>,
    ) -> Option<super::ConstraintType> {
        Some(ConstraintType::IfThen)
    }

    fn set_type<'a>(
        &'a self,
        _wip_registry: &mut crate::package::registry::WipRegistry<'a>,
        _constraint_type: ConstraintType,
    ) {
        tracing::warn!(
            "attempting to change data type of IfThen. This does nothing"
        );
    }

    fn type_check<'a>(
        &'a self,
        wip_registry: &mut crate::package::registry::WipRegistry<'a>,
    ) -> Result<(), SolverError> {
        let Some(cond_type) = self.cond.get_type(wip_registry) else {
            self.cond.set_type(
                wip_registry,
                ConstraintType::Value(spec::SpecOptionType::Bool),
            );

            return Ok(());
        };

        match cond_type {
            ConstraintType::Depends
            | ConstraintType::Equal
            | ConstraintType::NumOf => Ok(()),

            ConstraintType::IfThen => {
                tracing::error!(
                    "cannot have IfThen as condition for another IfThen. Consider using Boolean operators like And, Or, Not, etc."
                );

                Err(SolverError::InvalidConstraint(format!(
                    "nested IfThen constraint"
                )))
            }

            ConstraintType::Value(value) => {
                if matches!(value, spec::SpecOptionType::Bool) {
                    Ok(())
                } else {
                    tracing::error!(
                        "cannot use non-Boolean value {value:?} in IfThen condition"
                    );

                    Err(SolverError::InvalidConstraint(format!(
                        "non-Boolean condition in IfThen"
                    )))
                }
            }

            ConstraintType::SpecOption => {
                unreachable!()
            }
        }
    }

    #[instrument]
    fn extract_spec_options(&self) -> Vec<(&str, &str, spec::SpecOption)> {
        tracing::info!("extracting spec options");

        [&self.cond, &self.then]
            .iter()
            .flat_map(|c| c.extract_spec_options())
            .collect()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        [&self.cond, &self.then]
            .iter()
            .flat_map(|c| c.extract_dependencies())
            .collect()
    }

    fn to_z3_clause<'a>(
        &self,
        registry: &Registry<'a>,
    ) -> Result<z3::ast::Dynamic, SolverError> {
        tracing::info!("(if '{:?}' then '{:?}')", self.cond, self.then);

        let cond = self.cond.to_z3_clause(registry)?.as_bool().unwrap();
        let var = self.then.to_z3_clause(registry)?;

        let then = match var.sort_kind() {
            SortKind::Bool => Ok(var.as_bool().unwrap()),

            kind => {
                tracing::error!("`then` must be Bool");
                Err(SolverError::IncorrectZ3Type {
                    expected: SortKind::Bool,
                    received: kind,
                })
            }
        }?;

        Ok(cond.implies(then).into())
    }

    fn to_python_any<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, pyo3::PyAny>> {
        self.clone().into_bound_py_any(py)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

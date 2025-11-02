use std::collections::HashSet;

use pyo3::{IntoPyObjectExt, prelude::*};
use z3::SortKind;

use super::{ConstraintType, ConstraintUtils};
use crate::{
    package::{self, constraint::Constraint, outline::SolverError},
    spec,
};

#[pyclass(unsendable)]
#[derive(Clone, Debug)]
pub struct IfThen {
    #[pyo3(get, set)]
    pub cond: Constraint,

    #[pyo3(get, set)]
    pub then: Constraint,
}

impl ConstraintUtils for IfThen {
    fn get_type<'a>(
        &'a self,
        _registry: &package::BuiltRegistry<'a>,
    ) -> ConstraintType {
        ConstraintType::IfThen
    }

    fn try_get_type<'a>(
        &'a self,
        _wip_registry: &mut package::WipRegistry<'a>,
    ) -> Option<ConstraintType> {
        Some(ConstraintType::IfThen)
    }

    fn set_type<'a>(
        &'a self,
        _wip_registry: &mut package::WipRegistry<'a>,
        _constraint_type: ConstraintType,
    ) {
        tracing::warn!(
            "attempting to change data type of IfThen. This does nothing"
        );
    }

    fn type_check<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        let Some(cond_type) = self.cond.try_get_type(wip_registry) else {
            self.cond.set_type(
                wip_registry,
                ConstraintType::Value(spec::SpecOptionType::Bool),
            );

            return Ok(());
        };

        match cond_type {
            ConstraintType::Depends | ConstraintType::Cmp => Ok(()),

            ConstraintType::IfThen
            | ConstraintType::Maximize
            | ConstraintType::Minimize => {
                let msg = format!(
                    "invalid condition '{cond_type:?}' for IfThen. Consider using Boolean operators like And, Or, Not, etc."
                );

                tracing::error!("{msg}");

                Err(Box::new(SolverError::InvalidConstraint(msg)))
            }

            ConstraintType::Value(value) => {
                if matches!(value, spec::SpecOptionType::Bool) {
                    Ok(())
                } else {
                    tracing::error!(
                        "cannot use non-Boolean value {value:?} in IfThen condition"
                    );

                    Err(Box::new(SolverError::InvalidConstraint(
                        "non-Boolean condition in IfThen".into(),
                    )))
                }
            }

            ConstraintType::SpecOption => {
                unreachable!()
            }
        }
    }

    fn extract_spec_options(&self) -> Vec<(&str, &str, spec::SpecOption)> {
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

    #[tracing::instrument]
    fn to_z3_clauses<'a>(
        &self,
        registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<Vec<z3::ast::Dynamic>, Box<SolverError>> {
        tracing::info!("(If '{:?}' then '{:?}')", self.cond, self.then);

        let cond = self.cond.to_z3_clauses(registry)?;
        let var = self.then.to_z3_clauses(registry)?;

        if cond.len() != 1 || var.len() != 1 {
            tracing::error!("Cannot operate on multiple solver variables");
            panic!("Internal solver error");
        }

        let cond = cond[0].as_bool().unwrap();
        let var = &var[0];

        let then = match var.sort_kind() {
            SortKind::Bool => Ok(var.as_bool().unwrap()),

            kind => {
                tracing::error!("`then` must be Bool");
                Err(SolverError::IncorrectSolverType {
                    expected: SortKind::Bool,
                    received: kind,
                })
            }
        }?;

        Ok(vec![cond.implies(then).into()])
    }

    fn to_python_any<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, pyo3::PyAny>> {
        self.clone().into_bound_py_any(py)
    }
}

impl Into<Constraint> for IfThen {
    fn into(self) -> Constraint {
        Constraint::IfThen(Box::new(self))
    }
}

impl std::fmt::Display for IfThen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "If( {} ) Then [ {} ]", self.cond, self.then)
    }
}

use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use pyo3::{IntoPyObjectExt, prelude::*};
use z3::SortKind;

use super::Constraint;
use crate::{
    package::{
        self, constraint::ConstraintType, outline::SolverError,
        registry::Registry,
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
        _wip_registry: &mut package::WipRegistry<'a>,
    ) -> Option<super::ConstraintType> {
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
        let Some(cond_type) = self.cond.get_type(wip_registry) else {
            self.cond.set_type(
                wip_registry,
                ConstraintType::Value(spec::SpecOptionType::Bool),
            );

            return Ok(());
        };

        match cond_type {
            ConstraintType::Depends | ConstraintType::Equal => Ok(()),

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

    fn to_z3_clause<'a>(
        &self,
        registry: &package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        tracing::info!("(if '{:?}' then '{:?}')", self.cond, self.then);

        let cond = self.cond.to_z3_clause(registry)?.as_bool().unwrap();
        let var = self.then.to_z3_clause(registry)?;

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

impl std::fmt::Display for IfThen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "If( {} ) Then [ {} ]", self.cond, self.then)
    }
}

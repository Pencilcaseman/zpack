use std::collections::HashSet;

use pyo3::{
    IntoPyObjectExt, basic::CompareOp, exceptions::PyNotImplementedError,
    prelude::*,
};
use z3::SortKind;

use super::ConstraintUtils;
use crate::{
    package::{
        self,
        constraint::{Cmp, Constraint},
        outline::SolverError,
    },
    spec::{self, SpecOptionType},
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
    fn get_value_type<'a, V>(
        &'a self,
        _registry: Option<&package::registry::Registry<'a, V>>,
    ) -> Option<spec::SpecOptionType> {
        None
    }

    fn set_value_type<'a>(
        &'a self,
        _wip_registry: &mut package::WipRegistry<'a>,
        _value_type: spec::SpecOptionType,
    ) {
        panic!("Cannot set value type of IfThen");
    }

    fn type_check<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        match self.cond.get_value_type(Some(wip_registry)) {
            Some(SpecOptionType::Unknown) => {
                self.cond.set_value_type(wip_registry, SpecOptionType::Bool);
                Ok(())
            }

            Some(other) => {
                if other != SpecOptionType::Bool {
                    Err(Box::new(SolverError::IncorrectValueType {
                        expected: SpecOptionType::Bool,
                        received: other,
                    }))
                } else {
                    Ok(())
                }
            }

            None => Err(Box::new(SolverError::InvalidNonValueConstraint)),
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

impl From<IfThen> for Constraint {
    fn from(val: IfThen) -> Self {
        Constraint::IfThen(Box::new(val))
    }
}

impl std::fmt::Display for IfThen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "If( {} ) Then [ {} ]", self.cond, self.then)
    }
}

#[pymethods]
impl IfThen {
    #[new]
    fn py_new(cond: Constraint, then: Constraint) -> Self {
        Self { cond, then }
    }

    fn __richcmp__(
        &self,
        rhs: Constraint,
        op: CompareOp,
    ) -> Result<Constraint, PyErr> {
        Cmp::py_richcmp_helper(self.clone().into(), rhs.clone(), op.into())
    }
}

use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use pyo3::{IntoPyObjectExt, prelude::*};
use z3::SortKind;

use super::Constraint;
use crate::{
    package::{outline::SolverError, registry::Registry},
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
    fn extract_spec_options(
        &self,
        package: &str,
    ) -> Vec<(&str, spec::SpecOption)> {
        [&self.cond, &self.then]
            .iter()
            .flat_map(|c| c.extract_spec_options(package))
            .collect()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        [&self.cond, &self.then]
            .iter()
            .flat_map(|c| c.extract_dependencies())
            .collect()
    }

    fn get_type(&self) -> Option<super::ConstraintType> {
        todo!()
    }

    fn to_z3_clause<'a>(
        &self,
        package: &str,
        registry: &Registry<'a>,
    ) -> Result<z3::ast::Dynamic, SolverError> {
        tracing::info!(
            "{} -> (if '{:?}' then '{:?}')",
            package,
            self.cond,
            self.then
        );

        let cond =
            self.cond.to_z3_clause(package, registry)?.as_bool().unwrap();
        let var = self.then.to_z3_clause(package, registry)?;

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

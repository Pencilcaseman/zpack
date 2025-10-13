use std::{
    any::Any,
    collections::{HashMap, HashSet},
};

use pyo3::{IntoPyObjectExt, prelude::*};
use z3::{SortKind, ast::Bool};

use super::Constraint;
use crate::{
    package::{constraint::IfThen, outline::SolverError, registry::Registry},
    spec::spec_option::SpecOption,
};

#[pyclass]
#[derive(Clone, Debug)]
pub struct NumOf {
    #[pyo3(get, set)]
    pub n: i32,

    #[pyo3(get, set)]
    pub of: Vec<Box<dyn Constraint>>,
}

impl Constraint for NumOf {
    fn extract_spec_options(&self, package: &str) -> Vec<(&str, SpecOption)> {
        tracing::info!("extracting {} spec options", self.of.len());

        self.of.iter().flat_map(|c| c.extract_spec_options(package)).collect()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        self.of.iter().flat_map(|b| b.extract_dependencies()).collect()
    }

    fn to_z3_clause<'a>(
        &self,
        package: &str,
        registry: &Registry<'a>,
    ) -> Result<z3::ast::Dynamic, SolverError> {
        tracing::info!(
            "{} -> (exactly {} of {} constraints)",
            package,
            self.n,
            self.of.len()
        );

        let mut clauses = Vec::new();

        for constraint in &self.of {
            if (**constraint).as_any().is::<IfThen>() {
                tracing::error!("IfThen inside NumOf; this does nothing");
            }

            let cond = constraint.to_z3_clause(package, registry)?;
            let Some(cond) = cond.as_bool() else {
                let msg =
                    format!("expected Bool; received {:?}", cond.sort_kind());
                tracing::error!("{msg}");
                panic!("{msg}");
            };

            clauses.push((cond, 1));
        }

        let refs = clauses.iter().map(|(b, m)| (b, *m)).collect::<Vec<_>>();
        Ok(Bool::pb_eq(&refs, self.n).into())
    }

    fn to_python_any<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.clone().into_bound_py_any(py)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

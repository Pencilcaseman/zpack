use std::collections::{HashMap, HashSet};

use pyo3::{IntoPyObjectExt, prelude::*};
use z3::{SortKind, ast::Bool};

use super::Constraint;
use crate::{
    package::outline::SolverError,
    spec::spec_option::{PackageOptionAstMap, SpecOption},
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
    fn extract_spec_options(&self, package: &str) -> HashMap<&str, SpecOption> {
        self.of.iter().flat_map(|c| c.extract_spec_options(package)).collect()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        self.of.iter().flat_map(|b| b.extract_dependencies()).collect()
    }

    fn to_z3_clause<'a>(
        &self,
        package: &str,
        option_ast: &PackageOptionAstMap,
    ) -> Result<z3::ast::Dynamic, SolverError> {
        tracing::info!(
            "{} -> (exactly {} of {} constraints)",
            package,
            self.n,
            self.of.len()
        );

        // Ensure exactly n of the conditions are met and separately ensure the
        // implications of each condition are met

        let mut clauses = Vec::new();
        let mut implications = Vec::new();

        for constraint in &self.of {
            let cond = constraint.as_cond(package, option_ast)?;
            let imp = constraint.to_z3_clause(package, option_ast)?;

            match imp.sort_kind() {
                SortKind::Bool => {
                    clauses.push((cond, 1));
                    implications.push(imp.as_bool().unwrap());
                }
                kind => {
                    return Err(SolverError::IncorrectType {
                        expected: SortKind::Bool,
                        received: kind,
                    });
                }
            }
        }

        let refs = clauses.iter().map(|(b, m)| (b, *m)).collect::<Vec<_>>();
        let mut constraints = vec![Bool::pb_eq(&refs, self.n)];
        constraints.extend(implications);

        Ok(Bool::and(&constraints).into())
    }

    fn to_python_any<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.clone().into_bound_py_any(py)
    }
}

use std::collections::{HashMap, HashSet};

use pyo3::{IntoPyObjectExt, prelude::*};
use z3::SortKind;

use super::Constraint;
use crate::{
    package::outline::SolverError,
    spec::spec_option::{PackageOptionAstMap, SpecOption},
};

#[pyclass]
#[derive(Clone, Debug)]
pub struct Depends {
    #[pyo3(get, set)]
    on: String,
}

impl Depends {
    pub fn new(on: String) -> Self {
        Self { on }
    }
}

impl Constraint for Depends {
    fn extract_spec_options(
        &self,
        _package: &str,
    ) -> HashMap<&str, SpecOption> {
        HashMap::default()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        HashSet::from([self.on.clone()])
    }

    fn to_z3_clause<'a>(
        &self,
        package: &str,
        option_ast: &PackageOptionAstMap<'a>,
    ) -> Result<z3::ast::Dynamic, SolverError> {
        let Some(value) = option_ast.get(&(&self.on, None)) else {
            tracing::error!("package '{}' has no activation variable", self.on);

            return Err(SolverError::MissingDependency {
                package: package.to_string(),
                dep: self.on.clone(),
            });
        };

        match value.sort_kind() {
            SortKind::Bool => Ok(value.clone()),
            kind => {
                tracing::error!(
                    "package activation variable '{}' is not of type Bool",
                    self.on
                );

                Err(SolverError::IncorrectType {
                    expected: SortKind::Bool,
                    received: kind,
                })
            }
        }
    }

    fn to_python_any<'py>(
        &self,
        py: Python<'py>,
    ) -> pyo3::PyResult<pyo3::Bound<'py, pyo3::PyAny>> {
        self.clone().into_bound_py_any(py)
    }
}

#[pymethods]
impl Depends {
    #[new]
    pub fn py_new(name: String) -> PyResult<Self> {
        Ok(Self::new(name))
    }
}

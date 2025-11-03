use std::collections::HashSet;

use pyo3::{
    IntoPyObjectExt, basic::CompareOp, exceptions::PyNotImplementedError,
    prelude::*,
};
use z3::{Optimize, SortKind, ast::Bool};

use super::ConstraintUtils;
use crate::{
    package::{
        self, BuiltRegistry,
        constraint::{Cmp, Constraint},
        outline::SolverError,
    },
    spec::{SpecOption, SpecOptionType},
};

#[pyclass]
#[derive(Clone, Debug)]
pub struct Maximize {
    #[pyo3(get, set)]
    pub item: Constraint,
}

impl ConstraintUtils for Maximize {
    fn get_value_type<'a, V>(
        &'a self,
        _registry: Option<&package::registry::Registry<'a, V>>,
    ) -> Option<SpecOptionType> {
        None
    }

    fn set_value_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
        value_type: SpecOptionType,
    ) {
        self.item.set_value_type(wip_registry, value_type);
    }

    #[tracing::instrument(skip(wip_registry))]
    fn type_check<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        self.item.type_check(wip_registry)?;

        let Some(value_type) = self.item.get_value_type(Some(wip_registry))
        else {
            tracing::error!("Can only Maximize a Value constraint");
            return Err(Box::new(SolverError::InvalidNonValueConstraint));
        };

        match value_type {
            SpecOptionType::Unknown => todo!(),

            SpecOptionType::Bool | SpecOptionType::Str => {
                tracing::error!("Can only maximize Int, Float or Version");
                return Err(Box::new(SolverError::IncorrectValueType {
                    expected: SpecOptionType::Int,
                    received: value_type,
                }));
            }

            SpecOptionType::Int
            | SpecOptionType::Float
            | SpecOptionType::Version => Ok(()),
        }
    }

    fn extract_spec_options(&self) -> Vec<(&str, &str, SpecOption)> {
        self.item.extract_spec_options()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        self.item.extract_dependencies()
    }

    fn to_z3_clauses<'a>(
        &self,
        _registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<Vec<z3::ast::Dynamic>, Box<SolverError>> {
        panic!(
            "Cannot convert Maximize constraint into Z3 clause. Use add_to_solver"
        )
    }

    fn add_to_solver<'a>(
        &self,
        _toggle: &Bool,
        optimizer: &Optimize,
        registry: &mut BuiltRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        for item in self.item.to_z3_clauses(registry)? {
            if matches!(
                item.sort_kind(),
                SortKind::Int | SortKind::Real | SortKind::BV
            ) {
                optimizer.maximize(&item);
            }
        }

        Ok(())
    }

    fn to_python_any<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.clone().into_bound_py_any(py)
    }
}

impl From<Maximize> for Constraint {
    fn from(val: Maximize) -> Self {
        Constraint::Maximize(Box::new(val))
    }
}

impl std::fmt::Display for Maximize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Maximize( {} )", self.item)
    }
}

#[pymethods]
impl Maximize {
    #[new]
    fn py_new(item: Constraint) -> Self {
        Self { item }
    }

    fn __richcmp__(
        &self,
        rhs: Constraint,
        op: CompareOp,
    ) -> Result<Constraint, PyErr> {
        Cmp::py_richcmp_helper(self.clone().into(), rhs.clone(), op.into())
    }
}

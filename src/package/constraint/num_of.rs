use std::collections::HashSet;

use pyo3::{IntoPyObjectExt, prelude::*};
use z3::ast::Int;

use super::ConstraintUtils;
use crate::{
    package::{
        self,
        constraint::{CmpType, Constraint, ConstraintType, IfThen},
        outline::SolverError,
    },
    spec::{self, SpecOption},
};

#[pyclass]
#[derive(Clone, Debug)]
pub struct NumOf {
    #[pyo3(get, set)]
    pub of: Vec<Constraint>,
}

impl ConstraintUtils for NumOf {
    fn get_type<'a>(
        &'a self,
        _wip_registry: &mut package::WipRegistry<'a>,
    ) -> Option<ConstraintType> {
        Some(ConstraintType::Value(spec::SpecOptionType::Int))
    }

    fn set_type<'a>(
        &'a self,
        _wip_registry: &mut package::WipRegistry<'a>,
        _constraint_type: ConstraintType,
    ) {
        tracing::warn!(
            "attempting to change data type of NumOf. This does nothing"
        );
    }

    fn type_check<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        self.of.iter().try_for_each(|c| c.type_check(wip_registry))
    }

    fn extract_spec_options(&self) -> Vec<(&str, &str, SpecOption)> {
        tracing::info!("extracting {} spec options", self.of.len());
        self.of.iter().flat_map(|c| c.extract_spec_options()).collect()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        self.of.iter().flat_map(|b| b.extract_dependencies()).collect()
    }

    fn cmp_to_z3<'a>(
        &self,
        other: &Constraint,
        op: CmpType,
        registry: &package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        // Safe to unwrap since we've already type checked everything
        let s = self.to_z3_clause(registry)?.as_int().unwrap();
        let o = other.to_z3_clause(registry)?.as_int().unwrap();

        Ok(match op {
            CmpType::Less => s.lt(o).into(),
            CmpType::LessOrEqual => s.le(o).into(),
            CmpType::NotEqual => s.ne(o).into(),
            CmpType::Equal => s.eq(o).into(),
            CmpType::GreaterOrEqual => s.ge(o).into(),
            CmpType::Greater => s.gt(o).into(),
        })
    }

    fn to_z3_clause<'a>(
        &self,
        registry: &package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        let mut clauses = Vec::new();

        // TODO: Move this to type_check
        for constraint in &self.of {
            if matches!(constraint, Constraint::IfThen(_)) {
                tracing::error!("IfThen inside NumOf; this does nothing");
            }

            let cond = constraint.to_z3_clause(registry)?;
            let Some(cond) = cond.as_bool() else {
                let msg =
                    format!("expected Bool; received {:?}", cond.sort_kind());
                tracing::error!("{msg}");
                panic!("{msg}");
            };

            clauses.push(cond);
        }

        let refs = clauses
            .iter()
            .map(|b| b.ite(&Int::from_i64(1), &Int::from_i64(0)))
            .collect::<Vec<_>>();

        Ok(Int::add(&refs).into())
    }

    fn to_python_any<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.clone().into_bound_py_any(py)
    }
}

impl From<NumOf> for Constraint {
    fn from(val: NumOf) -> Self {
        Constraint::NumOf(Box::new(val))
    }
}

impl std::fmt::Display for NumOf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NumOf( ")?;
        self.of.iter().enumerate().try_for_each(|(idx, of)| {
            write!(f, "{}{}", of, if idx == self.of.len() { "" } else { ", " })
        })?;
        f.write_str(" )")
    }
}

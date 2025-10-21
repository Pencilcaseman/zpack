use std::{any::Any, collections::HashSet};

use pyo3::{IntoPyObjectExt, prelude::*};
use z3::{Optimize, ast::Bool};

use super::Constraint;
use crate::{
    package::{
        self, BuiltRegistry, constraint::ConstraintType, outline::SolverError,
    },
    spec::SpecOption,
};

#[pyclass]
#[derive(Clone, Debug)]
pub struct Minimize {
    #[pyo3(get, set)]
    pub item: Box<dyn Constraint>,
}

impl Constraint for Minimize {
    fn get_type<'a>(
        &'a self,
        _wip_registry: &mut package::WipRegistry<'a>,
    ) -> Option<ConstraintType> {
        Some(ConstraintType::Minimize)
    }

    fn set_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
        constraint_type: ConstraintType,
    ) {
        self.item.set_type(wip_registry, constraint_type);
    }

    fn type_check<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        self.item.type_check(wip_registry)?;

        if let Some(known) = self.item.get_type(wip_registry)
            && !matches!(
                known,
                ConstraintType::SpecOption | ConstraintType::Value(_)
            )
        {
            Err(Box::new(SolverError::IncorrectConstraintType {
                expected: ConstraintType::SpecOption,
                received: known,
            }))
        } else {
            Ok(())
        }
    }

    fn extract_spec_options(&self) -> Vec<(&str, &str, SpecOption)> {
        self.item.extract_spec_options()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        self.item.extract_dependencies()
    }

    fn to_z3_clause<'a>(
        &self,
        _registry: &package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        let msg = "cannot convert Minimize constraint into Z3 clause";
        tracing::error!(msg);
        Err(Box::new(SolverError::InvalidConstraint(msg.to_string())))
    }

    fn add_to_solver<'a>(
        &self,
        _toggle: &Bool,
        optimizer: &Optimize,
        registry: &mut BuiltRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        let item = self.item.to_z3_clause(registry)?;
        optimizer.minimize(&item);
        Ok(())
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

impl std::fmt::Display for Minimize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Minimize( {} )", self.item)
    }
}

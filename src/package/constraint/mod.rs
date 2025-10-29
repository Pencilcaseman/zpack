use std::collections::HashSet;

use pyo3::{exceptions::PyTypeError, prelude::*};
use tracing_subscriber::registry;
use z3::{Optimize, ast::Bool};

use crate::{
    package::{self, BuiltRegistry, outline::SolverError},
    spec,
};

pub const SOFT_PACKAGE_WEIGHT: usize = 1;

mod cmp;
mod depends;
mod if_then;
mod maximize;
mod minimize;
mod num_of;
mod spec_option;
mod value;

pub use cmp::{Cmp, CmpType};
pub use depends::Depends;
pub use if_then::IfThen;
pub use maximize::Maximize;
pub use minimize::Minimize;
pub use num_of::NumOf;
pub use spec_option::SpecOption;
pub use value::Value;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConstraintType {
    Depends,
    SpecOption,
    Value(spec::SpecOptionType),
    Cmp,
    IfThen,
    Maximize,
    Minimize,
}

#[derive(Debug, Clone)]
pub enum Constraint {
    Cmp(Box<Cmp>),
    Depends(Box<Depends>),
    IfThen(Box<IfThen>),
    Maximize(Box<Maximize>),
    Minimize(Box<Minimize>),
    NumOf(Box<NumOf>),
    SpecOption(Box<SpecOption>),
    Value(Box<Value>),
}

macro_rules! constraint_inner {
    ($constraint:ident, $inner:ident => $code:block) => {
        match $constraint {
            Constraint::Cmp($inner) => $code,
            Constraint::Depends($inner) => $code,
            Constraint::IfThen($inner) => $code,
            Constraint::Maximize($inner) => $code,
            Constraint::Minimize($inner) => $code,
            Constraint::NumOf($inner) => $code,
            Constraint::SpecOption($inner) => $code,
            Constraint::Value($inner) => $code,
        }
    };
}

impl std::fmt::Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        constraint_inner!(self, inner => { write!(f, "{}", inner) })
    }
}

impl ConstraintUtils for Constraint {
    fn get_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Option<ConstraintType> {
        constraint_inner!(self, inner => { inner.get_type(wip_registry) })
    }

    fn set_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
        constraint_type: ConstraintType,
    ) {
        constraint_inner!(self, inner => {
            inner.set_type(wip_registry, constraint_type);
        })
    }

    fn type_check<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        constraint_inner!(self, inner => { inner.type_check(wip_registry)})
    }

    fn extract_spec_options(&self) -> Vec<(&str, &str, spec::SpecOption)> {
        constraint_inner!(self, inner => { inner.extract_spec_options()})
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        constraint_inner!(self, inner => { inner.extract_dependencies()})
    }

    fn cmp_to_z3<'a>(
        &self,
        other: &Constraint,
        op: CmpType,
        registry: &package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        constraint_inner!(self, inner => {
            inner.cmp_to_z3(other, op, registry)
        })
    }

    fn to_z3_clause<'a>(
        &self,
        registry: &package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        constraint_inner!(self, inner => { inner.to_z3_clause(registry)})
    }

    fn to_python_any<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        constraint_inner!(self, inner => { inner.to_python_any(py)})
    }

    fn add_to_solver<'a>(
        &self,
        toggle: &Bool,
        optimizer: &Optimize,
        registry: &mut BuiltRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        constraint_inner!(self, inner => {
            inner.add_to_solver(toggle, optimizer, registry)
        })
    }
}

pub trait ConstraintUtils:
    Send + Sync + std::fmt::Debug + std::fmt::Display + Into<Constraint>
{
    fn get_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Option<ConstraintType>;

    // TODO: Refactor to take SpecOptionType?
    fn set_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
        constraint_type: ConstraintType,
    );

    fn type_check<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>>;

    fn extract_spec_options(&self) -> Vec<(&str, &str, spec::SpecOption)>;

    fn extract_dependencies(&self) -> HashSet<String>;

    #[tracing::instrument]
    fn cmp_to_z3<'a>(
        &self,
        other: &Constraint,
        op: CmpType,
        registry: &package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        match op {
            CmpType::Less
            | CmpType::LessOrEqual
            | CmpType::GreaterOrEqual
            | CmpType::Greater => {
                let msg = format!(
                    "only Equal and NotEqual are valid comparision operations for this constraint type; received {op:?}"
                );
                tracing::error!("{msg}");
                Err(Box::new(SolverError::InvalidConstraint(msg)))
            }

            CmpType::Equal => Ok(self
                .to_z3_clause(registry)?
                .eq(other.to_z3_clause(registry)?)
                .into()),

            CmpType::NotEqual => Ok(self
                .to_z3_clause(registry)?
                .ne(other.to_z3_clause(registry)?)
                .into()),
        }
    }

    fn to_z3_clause<'a>(
        &self,
        registry: &package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>>;

    fn add_to_solver<'a>(
        &self,
        toggle: &Bool,
        optimizer: &Optimize,
        registry: &mut BuiltRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        let clause = self.to_z3_clause(registry)?;
        let assertion = toggle.implies(clause.as_bool().unwrap());

        let boolean = z3::ast::Bool::new_const(
            registry.new_constraint_id(self.to_string()),
        );

        optimizer.assert_and_track(&assertion, &boolean);

        Ok(())
    }

    fn to_python_any<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>>;
}

impl<'py> FromPyObject<'py> for Constraint {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        fn extract<'py2, T, F>(
            ob: &Bound<'py2, PyAny>,
            to: F,
        ) -> PyResult<Constraint>
        where
            T: ConstraintUtils + FromPyObject<'py2> + 'static,
            F: FnOnce(Box<T>) -> Constraint,
        {
            Ok((to)(Box::new(ob.extract::<T>()?)))
        }

        extract::<Cmp, _>(ob, Constraint::Cmp)
            .or_else(|_| extract::<Depends, _>(ob, Constraint::Depends))
            .or_else(|_| extract::<IfThen, _>(ob, Constraint::IfThen))
            .or_else(|_| extract::<Maximize, _>(ob, Constraint::Maximize))
            .or_else(|_| extract::<Minimize, _>(ob, Constraint::Minimize))
            .or_else(|_| extract::<NumOf, _>(ob, Constraint::NumOf))
            .or_else(|_| extract::<SpecOption, _>(ob, Constraint::SpecOption))
            .or_else(|_| extract::<Value, _>(ob, Constraint::Value))
            .map_err(|_| {
                let msg =
                    format!("cannot convert '{}' to Constraint", ob.get_type());

                tracing::error!("{msg}");
                PyTypeError::new_err(msg)
            })
    }
}

impl<'py> IntoPyObject<'py> for Constraint {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(
        self,
        py: Python<'py>,
    ) -> Result<Self::Output, Self::Error> {
        match self {
            Constraint::Cmp(val) => val.to_python_any(py),
            Constraint::Depends(val) => val.to_python_any(py),
            Constraint::IfThen(val) => val.to_python_any(py),
            Constraint::Maximize(val) => val.to_python_any(py),
            Constraint::Minimize(val) => val.to_python_any(py),
            Constraint::NumOf(val) => val.to_python_any(py),
            Constraint::SpecOption(val) => val.to_python_any(py),
            Constraint::Value(val) => val.to_python_any(py),
        }
    }
}

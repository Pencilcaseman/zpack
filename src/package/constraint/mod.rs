use std::collections::HashSet;

use pyo3::{exceptions::PyTypeError, prelude::*};
use z3::{Optimize, ast::Bool};

use crate::{
    package::{
        self, BuiltRegistry,
        outline::SolverError,
        registry::{BuiltVersionRegistry, Registry},
        version::Version,
    },
    spec::{self, SpecOptionType, SpecOptionValue},
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

pub trait ConstraintUtils:
    Send + Sync + std::fmt::Debug + std::fmt::Display + Into<Constraint>
{
    fn get_value_type<'a, V>(
        &'a self,
        registry: Option<&Registry<'a, V>>,
    ) -> Option<SpecOptionType>;

    fn get_value_type_default<'a>(&'a self) -> Option<SpecOptionType> {
        self.get_value_type::<BuiltVersionRegistry>(None)
    }

    fn set_value_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
        value_type: SpecOptionType,
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
        registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        let s = self.to_z3_clauses(registry)?;
        let o = other.to_z3_clauses(registry)?;

        match op {
            CmpType::Less
            | CmpType::LessOrEqual
            | CmpType::GreaterOrEqual
            | CmpType::Greater => {
                let msg = format!(
                    "Only Equal and NotEqual are valid comparision operations for this constraint type; received {op:?}"
                );
                tracing::error!("{msg}");
                Err(Box::new(SolverError::InvalidConstraint(msg)))
            }

            CmpType::Equal => {
                if s.len() != o.len() {
                    Ok(Bool::from_bool(false).into())
                } else {
                    let conds: Vec<Bool> = s
                        .into_iter()
                        .zip(o.into_iter())
                        .map(|(s, o)| s.eq(o))
                        .collect();

                    Ok(Bool::and(&conds).into())
                }
            }

            CmpType::NotEqual => {
                if s.len() != o.len() {
                    Ok(Bool::from_bool(true).into())
                } else {
                    let conds: Vec<Bool> = s
                        .into_iter()
                        .zip(o.into_iter())
                        .map(|(s, o)| s.ne(o))
                        .collect();

                    Ok(Bool::or(&conds).into())
                }
            }
        }
    }

    fn to_z3_clauses<'a>(
        &self,
        registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<Vec<z3::ast::Dynamic>, Box<SolverError>>;

    fn add_to_solver<'a>(
        &self,
        toggle: &Bool,
        optimizer: &Optimize,
        registry: &mut BuiltRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        for clause in self.to_z3_clauses(registry)? {
            let assertion = toggle.implies(clause.as_bool().unwrap());

            let boolean = z3::ast::Bool::new_const(
                registry.new_constraint_id(self.to_string()),
            );

            optimizer.assert_and_track(&assertion, &boolean);
        }

        Ok(())
    }

    fn to_python_any<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>>;
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

impl std::fmt::Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        constraint_inner!(self, inner => { write!(f, "{}", inner) })
    }
}

impl ConstraintUtils for Constraint {
    fn get_value_type<'a, V>(
        &'a self,
        registry: Option<&Registry<'a, V>>,
    ) -> Option<SpecOptionType> {
        constraint_inner!(self, inner => {
            inner.get_value_type(registry)
        })
    }

    fn set_value_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
        value_type: SpecOptionType,
    ) {
        constraint_inner!(self, inner => {
            inner.set_value_type(wip_registry, value_type);
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
        registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        constraint_inner!(self, inner => {
            inner.cmp_to_z3(other, op, registry)
        })
    }

    fn to_z3_clauses<'a>(
        &self,
        registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<Vec<z3::ast::Dynamic>, Box<SolverError>> {
        constraint_inner!(self, inner => { inner.to_z3_clauses(registry)})
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

    fn to_python_any<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        constraint_inner!(self, inner => { inner.to_python_any(py)})
    }
}

impl<'a, 'py> FromPyObject<'a, 'py> for Constraint {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        fn extract_constraint<'a2, 'py2, T, F, E>(
            obj: &Borrowed<'a2, 'py2, PyAny>,
            to: F,
        ) -> Result<Constraint, E>
        where
            T: ConstraintUtils + FromPyObject<'a2, 'py2, Error = E> + 'static,
            F: FnOnce(Box<T>) -> Constraint,
            E: Into<<Constraint as FromPyObject<'a2, 'py2>>::Error>,
        {
            Ok((to)(Box::new(obj.extract::<T>()?)))
        }

        fn extract_value<'a2, 'py2, T, F, E>(
            obj: &Borrowed<'a2, 'py2, PyAny>,
            to: F,
        ) -> Result<Constraint, E>
        where
            T: FromPyObject<'a2, 'py2, Error = E> + 'static,
            F: FnOnce(T) -> SpecOptionValue,
            E: Into<<Constraint as FromPyObject<'a2, 'py2>>::Error>,
        {
            Ok(Constraint::Value(Box::new(Value {
                value: (to)(obj.extract::<T>()?),
            })))
        }

        extract_constraint::<Cmp, _, _>(&obj, Constraint::Cmp)
            .or_else(|_| {
                extract_constraint::<Depends, _, _>(&obj, Constraint::Depends)
            })
            .or_else(|_| {
                extract_constraint::<IfThen, _, _>(&obj, Constraint::IfThen)
            })
            .or_else(|_| {
                extract_constraint::<Maximize, _, _>(&obj, Constraint::Maximize)
            })
            .or_else(|_| {
                extract_constraint::<Minimize, _, _>(&obj, Constraint::Minimize)
            })
            .or_else(|_| {
                extract_constraint::<NumOf, _, _>(&obj, Constraint::NumOf)
            })
            .or_else(|_| {
                extract_constraint::<SpecOption, _, _>(
                    &obj,
                    Constraint::SpecOption,
                )
            })
            .or_else(|_| {
                extract_constraint::<Value, _, _>(&obj, Constraint::Value)
            })
            .or_else(|_| {
                extract_value::<bool, _, _>(&obj, SpecOptionValue::Bool)
            })
            .or_else(|_| extract_value::<i64, _, _>(&obj, SpecOptionValue::Int))
            .or_else(|_| {
                extract_value::<f64, _, _>(&obj, SpecOptionValue::Float)
            })
            .or_else(|_| {
                extract_value::<String, _, _>(&obj, SpecOptionValue::Str)
            })
            .or_else(|_| {
                extract_value::<Version, _, _>(&obj, SpecOptionValue::Version)
            })
            .map_err(|_| {
                let msg = format!(
                    "cannot convert '{}' to Constraint",
                    obj.get_type()
                );

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

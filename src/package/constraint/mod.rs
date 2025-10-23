use std::{any::Any, collections::HashSet};

use dyn_clone::DynClone;
use pyo3::{exceptions::PyTypeError, prelude::*};
use z3::{Optimize, ast::Bool};

use crate::{
    package::{self, BuiltRegistry, outline::SolverError},
    spec,
};

pub const SOFT_PACKAGE_WEIGHT: usize = 1;

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

pub trait Constraint:
    Send + Sync + DynClone + Any + std::fmt::Debug + std::fmt::Display
{
    fn get_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Option<ConstraintType>;

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

    fn as_any(&self) -> &dyn Any;
}

dyn_clone::clone_trait_object!(Constraint);

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

impl<'py> FromPyObject<'py> for Box<dyn Constraint> {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        fn try_extract<'py2, T>(
            ob: &Bound<'py2, PyAny>,
        ) -> PyResult<Box<dyn Constraint>>
        where
            T: Constraint + FromPyObject<'py2> + 'static,
        {
            Ok(Box::new(ob.extract::<T>()?))
        }

        try_extract::<Depends>(ob)
            .or_else(|_| try_extract::<IfThen>(ob))
            .or_else(|_| try_extract::<NumOf>(ob))
            .or_else(|_| try_extract::<SpecOption>(ob))
            .map_err(|_| {
                let msg =
                    format!("cannot convert '{}' to Constraint", ob.get_type());

                tracing::error!("{msg}");
                PyTypeError::new_err(msg)
            })
    }
}

impl<'py> IntoPyObject<'py> for Box<dyn Constraint> {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(
        self,
        py: Python<'py>,
    ) -> Result<Self::Output, Self::Error> {
        self.to_python_any(py)
    }
}

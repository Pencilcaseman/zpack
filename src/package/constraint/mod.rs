use std::{any::Any, collections::HashSet};

use dyn_clone::DynClone;
use pyo3::{exceptions::PyTypeError, prelude::*};

use crate::{
    package::{
        outline::SolverError,
        registry::{Registry, WipRegistry},
    },
    spec,
};

pub const SOFT_PACKAGE_WEIGHT: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintType {
    Depends,
    Equal,
    IfThen,
    SpecOption,
    Value(spec::SpecOptionType),
}

pub trait Constraint:
    Send + Sync + DynClone + Any + std::fmt::Debug + std::fmt::Display
{
    fn get_type<'a>(
        &'a self,
        wip_registry: &mut WipRegistry<'a>,
    ) -> Option<ConstraintType>;

    fn set_type<'a>(
        &'a self,
        wip_registry: &mut WipRegistry<'a>,
        constraint_type: ConstraintType,
    );

    fn type_check<'a>(
        &'a self,
        wip_registry: &mut WipRegistry<'a>,
    ) -> Result<(), SolverError>;

    fn extract_spec_options(&self) -> Vec<(&str, &str, spec::SpecOption)>;

    fn extract_dependencies(&self) -> HashSet<String>;

    fn to_z3_clause<'a>(
        &self,
        registry: &Registry<'a>,
    ) -> Result<z3::ast::Dynamic, SolverError>;

    fn to_python_any<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>>;

    fn as_any(&self) -> &dyn Any;
}

dyn_clone::clone_trait_object!(Constraint);

mod depends;
mod equal;
mod if_then;
mod num_of;
mod spec_option;
mod value;

pub use depends::Depends;
pub use equal::Equal;
pub use if_then::IfThen;
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

use std::{any::Any, collections::HashSet};

use dyn_clone::DynClone;
use pyo3::{exceptions::PyTypeError, prelude::*};

use crate::{
    package::{outline::SolverError, registry::Registry},
    spec::spec_option::SpecOption,
};

pub const SOFT_PACKAGE_WEIGHT: usize = 1;

pub trait Constraint: std::fmt::Debug + Send + Sync + DynClone + Any {
    fn extract_spec_options(&self, package: &str) -> Vec<(&str, SpecOption)>;
    fn extract_dependencies(&self) -> HashSet<String>;

    fn to_z3_clause<'a>(
        &self,
        package: &str,
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
mod if_then;
mod num_of;
mod spec_option;

pub use depends::Depends;
pub use if_then::IfThen;
pub use num_of::NumOf;
pub use spec_option::SpecOptionEqual;

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
            .or_else(|_| try_extract::<SpecOptionEqual>(ob))
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

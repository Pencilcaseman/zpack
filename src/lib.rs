#![warn(clippy::pedantic, clippy::nursery)]

use pyo3::prelude::*;

pub mod cli;
pub mod constraint;
pub mod interface;
pub mod package;
pub mod spec;
pub mod util;

fn gen_init(m: &Bound<'_, PyModule>, name: &str) -> PyResult<()> {
    Python::attach(|py| py.import("sys")?.getattr("modules")?.set_item(name, m))
}

#[pymodule(name = "constraint")]
pub mod py_constraint {
    use pyo3::prelude::*;

    #[pymodule_export]
    pub use crate::constraint::Cmp;
    #[pymodule_export]
    pub use crate::constraint::CmpType;
    #[pymodule_export]
    pub use crate::constraint::Depends;
    #[pymodule_export]
    pub use crate::constraint::IfThen;
    #[pymodule_export]
    pub use crate::constraint::Maximize;
    #[pymodule_export]
    pub use crate::constraint::Minimize;
    #[pymodule_export]
    pub use crate::constraint::NumOf;
    #[pymodule_export]
    pub use crate::constraint::SpecOption;
    #[pymodule_export]
    pub use crate::constraint::Value;

    /// Hacky workaround from <https://github.com/PyO3/pyo3/issues/759>
    ///
    /// # Errors
    /// May error if sys.modules is not loadable
    #[pymodule_init]
    pub fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        super::gen_init(m, "zpack.constraint")
    }
}

#[pymodule(name = "package")]
pub mod py_package {
    use pyo3::prelude::*;

    #[pymodule_export]
    pub use crate::package::outline::PackageOutline;
    #[pymodule_export]
    pub use crate::package::version::Version;

    /// Hacky workaround from <https://github.com/PyO3/pyo3/issues/759>
    ///
    /// # Errors
    /// May error if sys.modules is not loadable
    #[pymodule_init]
    pub fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        super::gen_init(m, "zpack.package")
    }
}

#[pymodule(name = "zpack")]
pub mod py_zpack {
    use pyo3::{exceptions::PyRuntimeError, prelude::*};

    #[pymodule_export]
    pub use super::py_constraint;
    #[pymodule_export]
    pub use super::py_package;

    /// The main python entry point
    ///
    /// # Errors
    /// Errors here will contain information from the root cause of the issue.
    /// It may also be worth calling
    /// [`zpack.init_tracing()`](py_tracing::init_tracing), as additional
    /// information is logged throughout the execution of `zpack`.
    #[pyfunction]
    pub fn main_entry() -> PyResult<()> {
        crate::cli::entry(true)
            .map_err(|e| PyRuntimeError::new_err(format!("{e:?}")))
    }

    /// Initialize the tracing subscriber in Python so internal logs are printed
    ///
    /// # Panics
    /// Panics if the subscriber cannot be created or set as the default
    #[pyfunction]
    pub fn init_tracing() {
        Python::attach(|_py| {
            tracing::subscriber::set_global_default(
                crate::util::subscriber::subscriber(),
            )
            .expect("Failed to set subscriber");
        });

        tracing::warn!("tracing activated");
    }
}

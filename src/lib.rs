use pyo3::{exceptions::PyRuntimeError, prelude::*};

use anyhow::Result;

pub mod entry;
pub mod package;
pub mod spec;
pub mod util;

fn register_submodule(
    parent: &Bound<'_, PyModule>,
    submodule: &Bound<'_, PyModule>,
    full_name: &str,
) -> PyResult<()> {
    parent.add_submodule(submodule)?;

    // Register in sys.modules
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item(full_name, submodule)?;

    Ok(())
}

fn register_module_package_outline(
    parent_module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let child_module = PyModule::new(parent_module.py(), "outline")?;
    use package::outline;

    child_module.add_class::<outline::PackageOutline>()?;

    register_submodule(parent_module, &child_module, "zpack.package.outline")?;

    Ok(())
}

fn register_module_package_constraint(
    parent_module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let child_module = PyModule::new(parent_module.py(), "constraint")?;
    use package::constraint;

    child_module.add_class::<constraint::Cmp>()?;
    child_module.add_class::<constraint::CmpType>()?;
    child_module.add_class::<constraint::Depends>()?;
    child_module.add_class::<constraint::IfThen>()?;
    child_module.add_class::<constraint::Maximize>()?;
    child_module.add_class::<constraint::Minimize>()?;
    child_module.add_class::<constraint::NumOf>()?;
    child_module.add_class::<constraint::SpecOption>()?;
    child_module.add_class::<constraint::Value>()?;

    register_submodule(
        parent_module,
        &child_module,
        "zpack.package.constraint",
    )?;

    Ok(())
}

fn register_module_package_version(
    parent_module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let child_module = PyModule::new(parent_module.py(), "version")?;

    child_module.add_class::<package::version::Version>()?;

    register_submodule(parent_module, &child_module, "zpack.package.version")?;

    Ok(())
}

fn register_module_package(
    parent_module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let child_module = PyModule::new(parent_module.py(), "package")?;

    register_module_package_outline(&child_module)?;
    register_module_package_constraint(&child_module)?;
    register_module_package_version(&child_module)?;

    register_submodule(parent_module, &child_module, "zpack.package")?;

    Ok(())
}

#[pyfunction]
fn init_tracing() {
    Python::attach(|_py| {
        tracing::subscriber::set_global_default(
            crate::util::subscriber::subscriber(),
        )
        .expect("Failed to set subscriber");
    });

    tracing::warn!("tracing activated");
}

#[pyfunction]
fn main_entry() -> PyResult<()> {
    entry::entry().map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

#[pymodule]
fn zpack(m: &Bound<'_, PyModule>) -> PyResult<()> {
    register_module_package(m)?;

    m.add_function(wrap_pyfunction!(init_tracing, m)?)?;

    m.add_function(wrap_pyfunction!(main_entry, m)?)?;

    Ok(())
}

use pyo3::prelude::*;

pub mod package;
pub mod spec;
pub mod util;

fn register_version_module(
    parent_module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let child_module = PyModule::new(parent_module.py(), "version")?;
    use package::version;

    child_module.add_class::<version::Version>()?;
    child_module.add_class::<version::SemVer>()?;
    child_module.add_class::<version::DotSeparated>()?;
    child_module.add_class::<version::Other>()?;

    parent_module.add_submodule(&child_module)?;
    Ok(())
}

fn register_package_module(
    parent_module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let child_module = PyModule::new(parent_module.py(), "package")?;

    register_version_module(&child_module)?;

    parent_module.add_submodule(&child_module)?;
    Ok(())
}

/// A Python module implemented in Rust.
#[pymodule]
fn zpack(m: &Bound<'_, PyModule>) -> PyResult<()> {
    register_package_module(&m)?;

    Ok(())
}

use pyo3::prelude::*;

pub mod package;
pub mod spec;
pub mod util;

fn register_package_module(
    parent_module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let child_module = PyModule::new(parent_module.py(), "package")?;

    child_module.add_class::<package::Version>()?;

    parent_module.add_submodule(&child_module)?;
    Ok(())
}

/// A Python module implemented in Rust.
#[pymodule]
fn zpack(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<package::version::Version>()?;

    register_package_module(m)?;

    Ok(())
}

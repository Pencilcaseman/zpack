use pyo3::prelude::*;

pub mod package;
pub mod spec;
pub mod util;

fn register_version_module(
    parent_module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let child_module = PyModule::new(parent_module.py(), "version")?;
    use package::version;

    child_module.add_class::<version::PyVersion>()?;
    child_module.add_class::<version::semver::PySemVer>()?;
    child_module.add_class::<version::dot_separated::PyDotSeparated>()?;
    child_module.add_class::<version::other::PyOther>()?;

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

#[pyfunction]
fn init_tracing() {
    Python::attach(|_py| {
        tracing::subscriber::set_global_default(
            crate::util::subscriber::subscriber(),
        )
        .expect("Failed to set subscriber");
    });
}

#[pyfunction]
fn test_function(version: &package::version::PyVersion) {
    println!("Got version: {}", version.inner);

    tracing::info!("Information?");
    tracing::warn!("Information?");
}

/// A Python module implemented in Rust.
#[pymodule]
fn zpack(m: &Bound<'_, PyModule>) -> PyResult<()> {
    register_package_module(m)?;

    m.add_function(wrap_pyfunction!(test_function, m)?)?;
    m.add_function(wrap_pyfunction!(init_tracing, m)?)?;

    Ok(())
}

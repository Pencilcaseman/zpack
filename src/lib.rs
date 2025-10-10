use pyo3::prelude::*;

pub mod package;
pub mod spec;
pub mod util;

fn register_module_package_version(
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

fn register_module_package_outline(
    parent_module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let child_module = PyModule::new(parent_module.py(), "outline")?;
    use package::outline;

    child_module.add_class::<outline::PackageOutline>()?;

    parent_module.add_submodule(&child_module)?;

    Ok(())
}

fn register_module_package_constraint(
    parent_module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let child_module = PyModule::new(parent_module.py(), "constraint")?;
    use package::constraint;

    child_module.add_class::<constraint::Depends>()?;
    child_module.add_class::<constraint::IfThen>()?;
    child_module.add_class::<constraint::NumOf>()?;
    child_module.add_class::<constraint::SpecOptionEqual>()?;

    parent_module.add_submodule(&child_module)?;

    Ok(())
}

fn register_module_package(
    parent_module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let child_module = PyModule::new(parent_module.py(), "package")?;

    register_module_package_version(&child_module)?;
    register_module_package_outline(&child_module)?;
    register_module_package_constraint(&child_module)?;

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

    tracing::warn!("tracing activated");
}

#[pymodule]
fn zpack(m: &Bound<'_, PyModule>) -> PyResult<()> {
    register_module_package(m)?;

    m.add_function(wrap_pyfunction!(init_tracing, m)?)?;

    Ok(())
}

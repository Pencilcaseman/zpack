use std::path::{Path, PathBuf};

use pyo3::{call::PyCallArgs, prelude::*};

#[derive(Debug)]
pub enum ReadError {
    PyErr(String),
    InvalidInstance,
    PathDoesNotExist(PathBuf),
    NotAFile(PathBuf),
    IoError(std::io::Error),
    NotCString,
}

pub fn read_from_class<'py, T, Args>(
    instance: Bound<'py, PyAny>,
    method: &str,
    args: Args,
) -> Result<T, ReadError>
where
    T: for<'a> FromPyObject<'a, 'py>,
    for<'a> <T as FromPyObject<'a, 'py>>::Error: std::fmt::Display,
    Args: PyCallArgs<'py>,
{
    let res = instance
        .call_method1(method, args)
        .map_err(|e| ReadError::PyErr(e.to_string()))?;

    res.extract::<T>().map_err(|e| ReadError::PyErr(e.to_string()))
}

pub fn read_from_class0<'py, T>(
    instance: Bound<'py, PyAny>,
    method: &str,
) -> Result<T, ReadError>
where
    T: for<'a> FromPyObject<'a, 'py>,
    for<'a> <T as FromPyObject<'a, 'py>>::Error: std::fmt::Display,
{
    println!("instance = {instance:?}");

    let res = instance
        .call_method0(method)
        .map_err(|e| ReadError::PyErr(e.to_string()))?;

    println!("res = {res:?}");

    res.extract::<T>().map_err(|e| ReadError::PyErr(e.to_string()))
}

pub fn process_file<'py>(
    py: Python<'py>,
    path: &Path,
) -> Result<Vec<Bound<'py, PyAny>>, ReadError> {
    if !path.exists() {
        return Err(ReadError::PathDoesNotExist(path.to_path_buf()));
    }

    if !path.is_file() {
        return Err(ReadError::PathDoesNotExist(path.to_path_buf()));
    }

    let contents = std::fs::read_to_string(path).map_err(ReadError::IoError)?;
    let cstr =
        std::ffi::CString::new(contents).map_err(|_| ReadError::NotCString)?;

    let module = PyModule::from_code(py, &cstr, c"package.py", c"package")
        .map_err(|e| ReadError::PyErr(e.to_string()))?;

    let packages_fn = module
        .getattr("zpack_packages")
        .map_err(|e| ReadError::PyErr(e.to_string()))?;

    packages_fn
        .call0()
        .map_err(|e| ReadError::PyErr(e.to_string()))?
        .extract()
        .map_err(|e: PyErr| ReadError::PyErr(e.to_string()))
}

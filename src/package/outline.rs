//! The package outline is the loose description of the versions, options,
//! dependencies, conflicts, etc. for a given package. This outline is then
//! refined with information from the package configuration options provided
//! from a configuration file or the command line.
//!
//! This outline definition is then passed to the [`Planner`], which solves for
//! a concrete, satisfiable set of dependencies and options which can then be
//! built and installed.

use pyo3::prelude::*;

use super::version;

#[derive(Debug)]
struct SpecOption;

#[derive(Debug)]
struct Constraint;

#[pyclass]
#[derive(Debug)]
pub struct Outline {
    name: String,
    category: Option<String>,
    versions: Vec<version::Version>,
    options: Vec<SpecOption>,
    dependencies: Vec<Outline>,
    constraints: Vec<Constraint>,
}

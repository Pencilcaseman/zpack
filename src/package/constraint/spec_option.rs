use std::collections::{HashMap, HashSet};

use chumsky::container::Seq;
use pyo3::{IntoPyObjectExt, prelude::*};
use tracing_subscriber::registry;

use crate::{
    package::{
        self,
        constraint::{Constraint, ConstraintType},
        outline::SolverError,
        registry::Registry,
    },
    spec,
};

#[pyclass]
#[derive(Clone, Debug)]
pub struct SpecOption {
    #[pyo3(get, set)]
    pub package_name: String,

    #[pyo3(get, set)]
    pub option_name: String,
}

impl Constraint for SpecOption {
    fn get_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Option<ConstraintType> {
        let idx = wip_registry.lookup_option(
            &self.package_name,
            Some(self.option_name.as_ref()),
        )?;

        Some(ConstraintType::Value(wip_registry.spec_options()[idx].0))
    }

    fn set_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
        constraint_type: ConstraintType,
    ) {
        let ConstraintType::Value(option_type) = constraint_type else {
            tracing::error!(
                "cannot set data type of SpecOption to anything but ConstraintType::Value(...)"
            );
            panic!("TODO: Improve error handling here");
        };

        wip_registry.insert_option(
            &self.package_name,
            Some(self.option_name.as_ref()),
            option_type,
            None,
        );
    }

    fn type_check<'a>(
        &self,
        _wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        // Nothing to type check
        Ok(())
    }

    fn extract_spec_options(&self) -> Vec<(&str, &str, spec::SpecOption)> {
        tracing::warn!("extracing SpecOption '{}'", self.option_name);
        vec![(
            &self.package_name,
            &self.option_name,
            spec::SpecOption::default(),
        )]
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        HashSet::default()
    }

    fn to_z3_clause<'a>(
        &self,
        registry: &package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        tracing::info!("{}:{}", self.package_name, self.option_name);

        let Some(idx) = registry
            .lookup_option(&self.package_name, Some(self.option_name.as_ref()))
        else {
            return if let Some(_) =
                registry.lookup_option(&self.package_name, None)
            {
                tracing::error!(
                    "missing variable {}:{}",
                    self.package_name,
                    self.option_name
                );

                Err(Box::new(SolverError::MissingVariable {
                    package: self.package_name.clone(),
                    name: self.option_name.clone(),
                }))
            } else {
                tracing::error!("missing package {}", self.package_name);

                Err(Box::new(SolverError::MissingPackage {
                    dep: self.package_name.clone(),
                }))
            };
        };

        let Some(dynamic) = &registry.spec_options()[idx].1 else {
            tracing::error!(
                "{}:{} not initialized in solver",
                self.package_name,
                self.option_name
            );
            panic!();
        };

        Ok(dynamic.clone())
    }

    fn to_python_any<'py>(
        &self,
        py: pyo3::Python<'py>,
    ) -> pyo3::PyResult<pyo3::Bound<'py, pyo3::PyAny>> {
        self.clone().into_bound_py_any(py)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl std::fmt::Display for SpecOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Package '{}' -> Option '{}'",
            self.package_name, self.option_name
        )
    }
}

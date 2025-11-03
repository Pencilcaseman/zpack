use std::collections::HashSet;

use pyo3::{
    IntoPyObjectExt, basic::CompareOp, exceptions::PyNotImplementedError,
    prelude::*,
};

use crate::{
    package::{
        self,
        constraint::{Cmp, CmpType, Constraint, ConstraintUtils},
        outline::SolverError,
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

impl ConstraintUtils for SpecOption {
    fn get_value_type<'a, V>(
        &'a self,
        registry: Option<&package::registry::Registry<'a, V>>,
    ) -> Option<spec::SpecOptionType> {
        match registry {
            Some(r) => {
                let Some(idx) = r
                    .lookup_option(&self.package_name, Some(&self.option_name))
                else {
                    return Some(spec::SpecOptionType::Unknown);
                };

                Some(r.spec_options()[idx].0)
            }
            None => Some(spec::SpecOptionType::Unknown),
        }
    }

    fn set_value_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
        value_type: spec::SpecOptionType,
    ) {
        wip_registry
            .insert_option(
                &self.package_name,
                Some(self.option_name.as_ref()),
                value_type,
                None,
            )
            .expect("Internal solver error");
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

    fn cmp_to_z3<'a>(
        &self,
        other: &Constraint,
        op: CmpType,
        registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        let value_type =
            self.get_value_type(Some(registry)).expect("Internal solver error");

        let t = self.to_z3_clauses(registry)?;
        let o = other.to_z3_clauses(registry)?;

        macro_rules! conv_op {
            ($in:ident $op:ident $out:ident, $conv:ident) => {{
                if $in.len() != 1 || $out.len() != 1 {
                    Err(Box::new(SolverError::InvalidNumberOfClauses(
                        match ($in.len(), $out.len()) {
                            (any, 1) => any,
                            (1, any) => any,
                            (any, _) => any,
                        },
                    )))
                } else {
                    Ok($in[0]
                        .$conv()
                        .unwrap()
                        .$op($out[0].$conv().unwrap())
                        .into())
                }
            }};
        }

        match value_type {
            spec::SpecOptionType::Unknown => panic!("Internal solver error"),

            spec::SpecOptionType::Bool => match op {
                CmpType::Less
                | CmpType::LessOrEqual
                | CmpType::GreaterOrEqual
                | CmpType::Greater => unreachable!(),

                CmpType::NotEqual => conv_op!(t ne o, as_bool),
                CmpType::Equal => conv_op!(t eq o, as_bool),
            },

            spec::SpecOptionType::Int => match op {
                CmpType::Less => conv_op!(t lt o, as_int),
                CmpType::LessOrEqual => conv_op!(t le o, as_int),
                CmpType::NotEqual => conv_op!(t ne o, as_int),
                CmpType::Equal => conv_op!(t eq o, as_int),
                CmpType::GreaterOrEqual => conv_op!(t ge o, as_int),
                CmpType::Greater => conv_op!(t gt o, as_int),
            },

            spec::SpecOptionType::Float => match op {
                CmpType::Less => conv_op!(t lt o, as_float),
                CmpType::LessOrEqual => conv_op!(t le o, as_float),
                CmpType::NotEqual => conv_op!(t ne o, as_float),
                CmpType::Equal => conv_op!(t eq o, as_float),
                CmpType::GreaterOrEqual => conv_op!(t ge o, as_float),
                CmpType::Greater => conv_op!(t gt o, as_float),
            },

            spec::SpecOptionType::Str => match op {
                CmpType::Less => conv_op!(t str_lt o, as_string),
                CmpType::LessOrEqual => conv_op!(t str_le o, as_string),
                CmpType::NotEqual => conv_op!(t ne o, as_string),
                CmpType::Equal => conv_op!(t eq o, as_string),
                CmpType::GreaterOrEqual => conv_op!(t str_ge o, as_string),
                CmpType::Greater => conv_op!(t str_gt o, as_string),
            },

            spec::SpecOptionType::Version => {
                let Constraint::Value(boxed) = other else {
                    panic!("Internal solver error");
                };

                let spec::SpecOptionValue::Version(version) = &boxed.value
                else {
                    panic!("Internal solver error");
                };

                // Ensure there is room
                registry.expand_version_to_fit(
                    &self.package_name,
                    Some(self.option_name.as_ref()),
                    version.num_segments(),
                )?;

                let v_reg = registry.version_registry();

                let Some(vars) = registry.lookup_version_solver_vars(
                    &self.package_name,
                    Some(self.option_name.as_ref()),
                ) else {
                    return Err(Box::new(SolverError::MissingVariable {
                        package: self.package_name.clone(),
                        name: self.option_name.clone(),
                    }));
                };

                let res = version.cmp_dynamic(op, vars, v_reg);
                Ok(res.into())
            }
        }
    }

    fn to_z3_clauses<'a>(
        &self,
        registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<Vec<z3::ast::Dynamic>, Box<SolverError>> {
        tracing::info!("{}:{}", self.package_name, self.option_name);

        let Some(idx) = registry
            .lookup_option(&self.package_name, Some(self.option_name.as_ref()))
        else {
            if registry.lookup_option(&self.package_name, None).is_some() {
                tracing::error!(
                    "Missing variable {}:{}",
                    self.package_name,
                    self.option_name
                );

                return Err(Box::new(SolverError::MissingVariable {
                    package: self.package_name.clone(),
                    name: self.option_name.clone(),
                }));
            } else {
                tracing::error!("Missing package {}", self.package_name);

                return Err(Box::new(SolverError::MissingPackage {
                    dep: self.package_name.clone(),
                }));
            };
        };

        let value_type =
            self.get_value_type(Some(registry)).expect("Internal solver error");

        if matches!(value_type, spec::SpecOptionType::Version) {
            Ok(registry
                .version_registry()
                .lookup_solver_vars(idx)
                .map(|vars| vars.to_vec())
                .unwrap())
        } else {
            let Some(dynamic) = &registry.spec_options()[idx].1 else {
                tracing::error!(
                    "{}:{} not initialized in solver",
                    self.package_name,
                    self.option_name
                );
                panic!("Internal solver error");
            };

            Ok(vec![dynamic.clone()])
        }
    }

    fn to_python_any<'py>(
        &self,
        py: pyo3::Python<'py>,
    ) -> pyo3::PyResult<pyo3::Bound<'py, pyo3::PyAny>> {
        self.clone().into_bound_py_any(py)
    }
}

impl From<SpecOption> for Constraint {
    fn from(val: SpecOption) -> Self {
        Constraint::SpecOption(Box::new(val))
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

#[pymethods]
impl SpecOption {
    #[new]
    fn py_new(package_name: String, option_name: String) -> Self {
        Self { package_name, option_name }
    }

    fn __richcmp__(
        &self,
        rhs: Constraint,
        op: CompareOp,
    ) -> Result<Constraint, PyErr> {
        Cmp::py_richcmp_helper(self.clone().into(), rhs.clone(), op.into())
    }
}

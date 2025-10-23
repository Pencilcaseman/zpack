use std::collections::HashMap;

use z3::Model;

use crate::{
    package::{
        BuiltRegistry,
        outline::SolverError,
        version::{Other, Version},
    },
    spec,
};

#[derive(Debug, Default, Clone)]
pub struct Registry<'a, VersionRegistryType> {
    // // Tracking variables for better error messages and debug information
    // current_package: Option<&'a str>,
    // current_option: Option<&'a str>,

    // Map from constraint ID to human-readable description
    constraint_descriptions: HashMap<String, String>,
    constraint_id: usize,

    // Lookup tables for type checking and solver generation
    spec_option_map: HashMap<(&'a str, Option<&'a str>), usize>,
    spec_options: Vec<(spec::SpecOptionType, Option<z3::ast::Dynamic>)>,

    version_registry: VersionRegistryType,
}

#[derive(Debug, Clone, Default)]
pub struct WipVersionRegistry {
    versions: Vec<Version>,
}

#[derive(Debug, Clone, Default)]
pub struct BuiltVersionRegistry {
    versions: HashMap<Version, usize>,
    ids: HashMap<usize, Version>,
}

impl WipVersionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new version to the registry
    pub fn push(&mut self, ver: Version) {
        tracing::info!("pushing version {ver}");
        self.versions.push(ver);
    }

    pub fn build(mut self) -> BuiltVersionRegistry {
        self.versions.sort();

        let mut versions = HashMap::with_capacity(self.versions.len());
        let mut ids = HashMap::with_capacity(self.versions.len());

        self.versions.iter().enumerate().for_each(|(idx, v)| {
            versions.insert(v.clone(), idx);
            ids.insert(idx, v.clone());
        });

        BuiltVersionRegistry { versions, ids }
    }
}

impl BuiltVersionRegistry {
    pub fn lookup(&self, ver: &Version) -> Option<usize> {
        self.versions.get(ver).copied()
    }

    pub fn lookup_id(&self, id: &usize) -> Option<&Version> {
        self.ids.get(id)
    }

    pub fn num_version(&self) -> usize {
        self.versions.len()
    }
}

impl<'a> Registry<'a, WipVersionRegistry> {
    pub fn build(self) -> Registry<'a, BuiltVersionRegistry> {
        Registry {
            // current_package_name: self.current_package_name,
            // current_option_name: self.current_option_name,
            constraint_descriptions: self.constraint_descriptions,
            constraint_id: self.constraint_id,
            spec_option_map: self.spec_option_map,
            spec_options: self.spec_options,
            version_registry: self.version_registry.build(),
        }
    }
}

impl<'a, T> Registry<'a, T> {
    pub fn lookup_option(
        &self,
        package: &'a str,
        option: Option<&'a str>,
    ) -> Option<usize> {
        self.spec_option_map.get(&(package, option)).copied()
    }

    pub fn insert_option_type(
        &mut self,
        package: &'a str,
        option: Option<&'a str>,
        dtype: spec::SpecOptionType,
    ) -> Result<(), SolverError> {
        self.insert_option(package, option, dtype, None)
    }

    pub fn set_option_value(
        &mut self,
        package: &'a str,
        option: Option<&'a str>,
        value: z3::ast::Dynamic,
    ) -> Result<(), SolverError> {
        let Some(idx) = self.lookup_option(package, option) else {
            tracing::error!("option {package}:{option:?} does not exist");

            return Err(match option {
                Some(name) => SolverError::MissingVariable {
                    package: package.to_string(),
                    name: name.to_string(),
                },
                None => {
                    SolverError::MissingPackage { dep: package.to_string() }
                }
            });
        };

        if self.spec_options[idx].1.is_some() {
            tracing::error!(
                "solver variable {package}:{option:?} already set. not overwriting"
            );
            panic!();
        } else {
            self.spec_options[idx].1 = Some(value);
            Ok(())
        }
    }

    pub fn insert_option(
        &mut self,
        package: &'a str,
        option: Option<&'a str>,
        dtype: spec::SpecOptionType,
        value: Option<z3::ast::Dynamic>,
    ) -> Result<(), SolverError> {
        if let Some(idx) = self.lookup_option(package, option) {
            if self.spec_options[idx].0 != dtype {
                // TODO: Proper error handling
                panic!("Conflicting datatypes")
            }
        }

        let idx = self.spec_options.len();
        self.spec_option_map.insert((package, option), idx);
        self.spec_options.push((dtype, value));

        Ok(())
    }

    pub fn spec_option_names(&self) -> Vec<&(&'a str, Option<&'a str>)> {
        self.spec_option_map.keys().collect()
    }

    pub fn spec_options(
        &self,
    ) -> &[(spec::SpecOptionType, Option<z3::ast::Dynamic>)] {
        &self.spec_options
    }

    pub fn version_registry(&self) -> &T {
        &self.version_registry
    }

    pub fn version_registry_mut(&mut self) -> &mut T {
        &mut self.version_registry
    }

    pub fn new_constraint_id(&mut self, description: String) -> String {
        let idx = format!("{}", self.constraint_id);
        self.constraint_id += 1;
        self.constraint_descriptions.insert(idx.clone(), description);
        idx
    }

    pub fn constraint_description(
        &self,
        lit: &z3::ast::Bool,
    ) -> Option<&String> {
        let name = lit.to_string();

        let id = if name.starts_with('|') {
            &name[1..name.len() - 1]
        } else {
            &name
        };

        self.constraint_descriptions.get(id)
    }

    pub fn eval_option(
        &self,
        package: &'a str,
        option: Option<&'a str>,
        model: &Model,
        registry: &'a BuiltRegistry,
    ) -> Result<spec::SpecOptionValue, Box<SolverError>> {
        let idx = self.lookup_option(package, option).ok_or_else(|| {
            tracing::error!("missing option {package}:{option:?}");

            if let Some(name) = option {
                SolverError::MissingVariable {
                    package: package.to_string(),
                    name: name.to_string(),
                }
            } else {
                SolverError::MissingPackage { dep: package.to_string() }
            }
        })?;

        let val = &self.spec_options()[idx];
        let Some(dynamic) = &val.1 else {
            return Err(Box::new(SolverError::NoSolverVariable {
                package: package.to_string(),
                option: option.map(str::to_string),
            }));
        };

        let model_eval = model.eval(dynamic, true).unwrap();

        Ok(spec::SpecOptionValue::from_z3_dynamic(val.0, &model_eval, registry))
    }
}

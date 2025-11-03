use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use crate::{
    package::{
        BuiltRegistry,
        outline::SolverError,
        version::{self, Part, Version},
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
    strings: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct BuiltVersionRegistry {
    offset: usize,
    string_to_id: HashMap<String, usize>,
    id_to_string: HashMap<usize, String>,

    /// Alternates between values and separators. Values are always integers
    /// and separators are always strings.
    ///
    /// E.g. 1.2.3 => [1, ".", 2, ".", 3] (ignoring string versions)
    versions: HashMap<usize, usize>,
    solver_vars: Vec<Vec<z3::ast::Dynamic>>,
    current_id: usize,
}

impl WipVersionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new version to the registry
    pub fn push(&mut self, ver: Version) {
        tracing::info!("pushing version {ver}");

        self.strings.extend(ver.parts().iter().filter_map(|p| match p {
            version::Part::Str(s) => Some(s.clone()),

            version::Part::Int(_)
            | version::Part::Sep(_)
            | version::Part::Wildcard(_) => None,
        }))
    }

    /// Thing
    ///
    /// - `versions` maps from registry index to count
    pub fn build(
        self,
        versions: HashMap<usize, usize>,
    ) -> BuiltVersionRegistry {
        let mut strings: Vec<String> = self.strings.into_iter().collect();
        strings.sort();
        strings.extend(
            version::STATIC_STRING_VERSIONS.iter().map(|s| s.to_string()),
        );

        let offset = strings.len();
        let mut string_to_id = HashMap::with_capacity(offset);
        let mut id_to_string = HashMap::with_capacity(offset);

        strings.into_iter().enumerate().for_each(|(idx, v)| {
            string_to_id.insert(v.clone(), idx);
            id_to_string.insert(idx, v);
        });

        let num_versions = versions.len();
        let solver_vars = (0..num_versions).map(|_| Vec::new()).collect();

        let mut res = BuiltVersionRegistry {
            string_to_id,
            id_to_string,
            offset,
            versions,
            solver_vars,
            current_id: 0,
        };

        for i in 0..num_versions {
            let var = z3::ast::Int::new_const(res.next_id_name()).into();
            res.solver_vars[i].push(var);
        }

        res
    }
}

impl BuiltVersionRegistry {
    pub fn lookup_str(&self, txt: &str) -> Option<&usize> {
        self.string_to_id.get(txt)
    }

    pub fn lookup_id(&self, id: &usize) -> Option<&String> {
        self.id_to_string.get(id)
    }

    pub fn lookup_solver_vars(
        &self,
        idx: usize,
    ) -> Option<&[z3::ast::Dynamic]> {
        let idx = self.versions.get(&idx)?;
        Some(&self.solver_vars[*idx])
    }

    pub fn part_to_dynamic(&self, part: Part) -> z3::ast::Dynamic {
        match part {
            Part::Int(i) => {
                z3::ast::Int::from_u64((self.offset() + i) as u64).into()
            }

            Part::Str(s) => z3::ast::Int::from_u64(
                *self.lookup_str(&s).expect("Internal solver error") as u64,
            )
            .into(),

            Part::Sep(c) => {
                z3::ast::String::from_str(&c.to_string()).unwrap().into()
            }

            Part::Wildcard(_) => unreachable!(),
        }
    }

    pub fn int_to_part(&self, int: usize) -> version::Part {
        if int >= self.offset() {
            version::Part::Int(int - self.offset())
        } else {
            version::Part::Str(self.lookup_id(&int).unwrap().to_string())
        }
    }

    pub fn next_id_name(&mut self) -> String {
        // TODO: Ensure names cannot collided
        let old = self.current_id;
        self.current_id += 1;
        format!("version_component_{old}")
    }

    /// Expand a solver variable collection to hold `parts` components.
    /// Note that `parts` **does not include** separators
    pub fn expand_to_fit(&mut self, idx: usize, parts: usize) {
        let Some(&idx) = self.versions.get(&idx) else {
            panic!("Missing entry for version at index {idx}");
        };

        let mut current_parts = self.solver_vars[idx].len() / 2 + 1;

        while current_parts < parts {
            let sep = z3::ast::String::new_const(self.next_id_name());
            let int = z3::ast::Int::new_const(self.next_id_name());

            self.solver_vars[idx].push(sep.into());
            self.solver_vars[idx].push(int.into());

            current_parts += 1;
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl<'a> Registry<'a, WipVersionRegistry> {
    pub fn build(self) -> Registry<'a, BuiltVersionRegistry> {
        let mut versions = HashMap::new();
        let mut count = 0;

        for idx in self.spec_option_map.values() {
            if matches!(
                self.spec_options[*idx].0,
                spec::SpecOptionType::Version
            ) {
                versions.insert(*idx, count);
                count += 1;
            }
        }

        Registry {
            constraint_descriptions: self.constraint_descriptions,
            constraint_id: self.constraint_id,
            spec_option_map: self.spec_option_map,
            spec_options: self.spec_options,
            version_registry: self.version_registry.build(versions),
        }
    }
}

impl<'a> Registry<'a, BuiltVersionRegistry> {
    pub fn lookup_version_solver_vars<'b>(
        &self,
        package: &'b str,
        option: Option<&'b str>,
    ) -> Option<&[z3::ast::Dynamic]> {
        let idx = self.lookup_option(package, option)?;
        self.version_registry().lookup_solver_vars(idx)
    }

    pub fn expand_version_to_fit<'b>(
        &mut self,
        package: &'b str,
        option: Option<&'b str>,
        parts: usize,
    ) -> Result<(), Box<SolverError>> {
        let idx = self.lookup_option(package, option).ok_or_else(|| {
            Box::new(match option {
                Some(name) => SolverError::MissingVariable {
                    package: package.to_string(),
                    name: name.to_string(),
                },
                None => {
                    SolverError::MissingPackage { dep: package.to_string() }
                }
            })
        })?;

        self.version_registry.expand_to_fit(idx, parts);

        Ok(())
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
    ) -> Result<(), Box<SolverError>> {
        self.insert_option(package, option, dtype, None)
    }

    pub fn set_option_value(
        &mut self,
        package: &'a str,
        option: Option<&'a str>,
        value: z3::ast::Dynamic,
    ) -> Result<(), Box<SolverError>> {
        let Some(idx) = self.lookup_option(package, option) else {
            tracing::error!("Option {package}:{option:?} does not exist");

            return Err(match option {
                Some(name) => Box::new(SolverError::MissingVariable {
                    package: package.to_string(),
                    name: name.to_string(),
                }),
                None => Box::new(SolverError::MissingPackage {
                    dep: package.to_string(),
                }),
            });
        };

        if self.spec_options[idx].1.is_some() {
            tracing::error!(
                "Solver variable {package}:{option:?} already set. not overwriting"
            );
            panic!("Internal solver error");
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
    ) -> Result<(), Box<SolverError>> {
        if let Some(idx) = self.lookup_option(package, option) {
            if self.spec_options[idx].0 != dtype {
                // TODO: Proper error handling
                panic!("Conflicting datatypes")
            }
        }

        let idx = self.spec_option_map.len();
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
        model: &z3::Model,
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

        Ok(spec::SpecOptionValue::from_z3_dynamic(
            package,
            option,
            val.0,
            &model_eval,
            model,
            registry,
        ))
    }
}

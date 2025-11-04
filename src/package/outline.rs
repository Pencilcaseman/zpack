//! The package outline is the loose description of the versions, options,
//! dependencies, conflicts, etc. for a given package. This outline is then
//! refined with information from the package configuration options provided
//! from a configuration file or the command line.
//!
//! This outline definition is then passed to the `Planner`, which solves for
//! a concrete, satisfiable set of dependencies and options which can then be
//! built and installed.

use std::collections::HashMap;

use petgraph::{algo::Cycle, graph::DiGraph, visit::EdgeRef};
use pyo3::{exceptions::PyValueError, prelude::*, types::PyDict};
use z3::{Optimize, SortKind};

use super::constraint::ConstraintUtils;
use crate::{
    package::{
        self,
        constraint::{
            self, Constraint, SOFT_PACKAGE_WEIGHT, SpecOption, Value,
        },
    },
    spec::{self, SpecOptionType},
};

pub type PackageDiGraph = DiGraph<PackageOutline, u8>;
pub type SpecMap = HashMap<String, Option<spec::SpecOptionValue>>;

#[pyclass]
#[derive(Clone, Debug, Default)]
pub struct PackageOutline {
    pub name: String,
    pub constraints: Vec<Constraint>,
    pub set_options: HashMap<String, spec::SpecOptionValue>,
    pub set_defaults: HashMap<String, Option<spec::SpecOptionValue>>,
}

impl std::fmt::Display for PackageOutline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl PackageOutline {
    pub fn dependencies(&self) -> Vec<String> {
        let mut res = Vec::new();

        for constraint in &self.constraints {
            res.extend(constraint.extract_dependencies());
        }

        res
    }
}

pub struct SpecOutline {
    pub graph: PackageDiGraph,
    pub lookup: HashMap<String, petgraph::graph::NodeIndex>,
    pub required: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum GenSpecSolverError {
    DuplicateOption(String),
    InvalidConstraint(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SolverError {
    DuplicateOption(String),

    MissingPackage {
        dep: String,
    },

    MissingVariable {
        package: String,
        name: String,
    },

    InvalidNonValueConstraint,

    IncorrectValueType {
        expected: SpecOptionType,
        received: SpecOptionType,
    },

    InvalidConstraint(String),

    IncorrectSolverType {
        expected: SortKind,
        received: SortKind,
    },

    DuplicatePackageEntry(String),

    NoSolverVariable {
        package: String,
        option: Option<String>,
    },

    Cycle(Cycle<<PackageDiGraph as petgraph::visit::GraphBase>::NodeId>),

    DefaultConflict {
        package_name: String,
        default_name: String,
        first_setter: String,
        first_value: spec::SpecOptionValue,
        conflict_setter: String,
        conflict_value: spec::SpecOptionValue,
    },

    InvalidNumberOfClauses(usize),
}

impl SpecOutline {
    pub fn new(
        outlines: Vec<PackageOutline>,
    ) -> Result<Self, Box<SolverError>> {
        let mut lookup = HashMap::new();
        let mut graph = PackageDiGraph::new();

        for outline in outlines {
            let name = outline.name.clone();
            let idx = graph.add_node(outline);
            lookup.insert(name, idx);
        }

        let mut edges = Vec::new();

        for src in graph.node_indices() {
            let src_name = &graph[src].name;

            for dep in &graph[src].dependencies() {
                edges.push((
                    src,
                    *lookup.get(dep).ok_or_else(|| {
                        tracing::error!(
                            "missing dependency '{dep}'; required by '{}'",
                            src_name
                        );

                        SolverError::MissingPackage { dep: dep.clone() }
                    })?,
                ))
            }
        }

        graph.extend_with_edges(edges);

        let required = Vec::new();

        Ok(Self { graph, lookup, required })
    }

    /// Propagate default values throughout the DAG.
    ///
    /// Defaults are propagated as follows:
    /// - old value does not exist => use current default
    /// - new value is None => remove from defaults
    /// - new value set explicitly => use explicit value
    /// - new value is inherited and conflicts with an inherited value => error
    ///
    /// The return value of this function indicates either successful
    /// propagation or an error for one of two reasons:
    /// - A cycle exists in the graph, in which case it is impossible to
    ///   propagate default values
    /// - Two inherited defaults conflict
    pub fn propagate_defaults(&mut self) -> Result<(), Box<SolverError>> {
        use petgraph::algo::toposort;

        tracing::info!("propagating default values");

        let mut reason_tracker = HashMap::<(String, String), String>::new();

        let sorted = toposort(&self.graph, None).map_err(SolverError::Cycle)?;

        for idx in sorted {
            let src_name = self.graph[idx].name.clone();
            let src_defaults = self.graph[idx].set_defaults.clone();

            tracing::info!("propagating default values for {src_name}");

            let deps: Vec<_> = self
                .graph
                .edges_directed(idx, petgraph::Direction::Outgoing)
                .map(|e| e.target())
                .collect();

            for dep in deps {
                let dep = &mut self.graph[dep];

                for (opt_name, src_val) in src_defaults.iter() {
                    tracing::info!(
                        "propagating default value {src_name}:{opt_name}"
                    );

                    // Skip None values
                    let Some(src_val) = src_val else {
                        continue;
                    };

                    match &dep.set_defaults.get(opt_name) {
                        Some(Some(old_val)) => {
                            if let Some(reason) = reason_tracker
                                .get(&(dep.name.clone(), opt_name.clone()))
                                && old_val != src_val
                            {
                                tracing::error!(
                                    "conflicting default values detected"
                                );

                                let e = SolverError::DefaultConflict {
                                    package_name: dep.name.clone(),
                                    default_name: opt_name.clone(),
                                    first_setter: reason.clone(),
                                    first_value: old_val.clone(),
                                    conflict_setter: match reason_tracker.get(
                                        &(src_name.clone(), opt_name.clone()),
                                    ) {
                                        Some(val) => val.clone(),
                                        None => src_name.clone(),
                                    },
                                    conflict_value: src_val.clone(),
                                };

                                return Err(Box::new(e));
                            }
                        }
                        Some(None) => {
                            dep.set_defaults.remove(opt_name);
                        }

                        None => {
                            // Insert and track default

                            dep.set_defaults.insert(
                                opt_name.clone(),
                                Some(src_val.clone()),
                            );

                            let reason = match reason_tracker
                                .get(&(src_name.clone(), opt_name.clone()))
                            {
                                Some(prev) => prev.clone(),
                                None => src_name.clone(),
                            };

                            reason_tracker.insert(
                                (dep.name.clone(), opt_name.clone()),
                                reason,
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn type_check<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        for idx in self.graph.node_indices() {
            let package = &self.graph[idx];

            tracing::info!("checking types for package '{}'", package.name);

            for constraint in &package.constraints {
                tracing::info!("checking types for constraint '{constraint}'");

                constraint.type_check(wip_registry)?;
            }
        }

        Ok(())
    }

    pub fn create_tracking_variables<'a>(
        &'a self,
        optimizer: &Optimize,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>>
    where
        Self: 'a,
    {
        for idx in self.graph.node_indices() {
            let package = &self.graph[idx];

            tracing::info!("creating activation toggle for {}", package.name);

            let package_toggle =
                z3::ast::Bool::new_const(package.name.to_string());

            optimizer.assert_soft(
                &package_toggle.not(),
                SOFT_PACKAGE_WEIGHT,
                None,
            );

            wip_registry.insert_option(
                &package.name,
                None,
                spec::SpecOptionType::Bool,
                Some(package_toggle.into()),
            )?;

            for (package_name, option_name, value) in package
                .constraints
                .iter()
                .flat_map(|c| c.extract_spec_options())
            {
                tracing::info!(
                    "creating variable for {}:{}",
                    package_name,
                    option_name
                );

                // Cannot skip this call since registry must be updated
                let val = value.to_z3_dynamic(
                    package_name,
                    option_name,
                    wip_registry,
                );

                if let Some(idx) =
                    wip_registry.lookup_option(package_name, Some(option_name))
                {
                    if wip_registry.spec_options()[idx].1.is_some() {
                        tracing::info!(
                            "solver variable {package_name}:{option_name} already exists. Continuing"
                        );
                    } else {
                        wip_registry.set_option_value(
                            package_name,
                            Some(option_name),
                            val,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn handle_explicit_options<'a>(
        &'a self,
        optimizer: &Optimize,
        registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<(), Box<SolverError>>
    where
        Self: 'a,
    {
        for idx in self.graph.node_indices() {
            let package = &self.graph[idx];

            for (name, value) in &package.set_options {
                tracing::info!(
                    "adding explicit value {}:{name} -> {value}",
                    package.name
                );

                let eq = constraint::Cmp {
                    lhs: SpecOption {
                        package_name: package.name.clone(),
                        option_name: name.clone(),
                    }
                    .into(),

                    rhs: Value { value: value.clone() }.into(),

                    op: constraint::CmpType::Equal,
                };

                let Some(idx) = registry.lookup_option(&package.name, None)
                else {
                    panic!("package '{}' does not exist", package.name);
                };

                let Some(dynamic) = &registry.spec_options()[idx].1 else {
                    panic!(
                        "{}:{} not assigned variable in solver",
                        package.name, name
                    );
                };

                // Safe because package toggle guaranteed to exist and `eq` will
                // only return a single clause
                optimizer.assert_and_track(
                    &dynamic.as_bool().unwrap().implies(
                        eq.to_z3_clauses(registry).unwrap()[0]
                            .as_bool()
                            .unwrap(),
                    ),
                    &z3::ast::Bool::new_const(
                        registry.new_constraint_id(eq.to_string()),
                    ),
                );
            }
        }

        Ok(())
    }

    pub fn require_packages<'a>(
        &'a self,
        optimizer: &Optimize,
        registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<(), Box<SolverError>>
    where
        Self: 'a,
    {
        for r in &self.required {
            let Some(idx) = registry.lookup_option(r, None) else {
                tracing::error!("missing explicitly required dependency '{r}'");
                return Err(Box::new(SolverError::MissingPackage {
                    dep: r.clone(),
                }));
            };

            let Some(dynamic) = &registry.spec_options()[idx].1 else {
                panic!(
                    "activation toggle for package '{}' not assigned variable in solver",
                    r
                );
            };

            let assertion = &dynamic.as_bool().unwrap();

            let boolean = z3::ast::Bool::new_const(
                registry
                    .new_constraint_id(format!("'{r}' required explicitly")),
            );

            optimizer.assert_and_track(assertion, &boolean);
        }

        Ok(())
    }

    pub fn add_constraints<'a>(
        &'a self,
        optimizer: &Optimize,
        registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<(), Box<SolverError>>
    where
        Self: 'a,
    {
        for idx in self.graph.node_indices() {
            let package = &self.graph[idx];

            tracing::info!("adding constraints for {}", package.name);

            let Some(idx) = registry.lookup_option(&package.name, None) else {
                tracing::error!("package '{}' not found", package.name);

                return Err(Box::new(SolverError::MissingPackage {
                    dep: package.name.clone(),
                }));
            };

            let Some(dynamic) = &registry.spec_options()[idx].1 else {
                panic!(
                    "activation toggle for package '{}' not assigned variable in solver",
                    package.name
                );
            };

            let package_toggle = &dynamic.as_bool().unwrap();

            for constraint in &package.constraints {
                tracing::info!(
                    "adding constraint {} -> {}",
                    package.name,
                    constraint
                );

                constraint.add_to_solver(
                    package_toggle,
                    optimizer,
                    registry,
                )?;
            }
        }

        Ok(())
    }

    pub fn gen_spec_solver(
        &mut self,
    ) -> Result<(Optimize, package::BuiltRegistry<'_>), Box<SolverError>> {
        tracing::info!("generating spec solver");

        let optimizer = Optimize::new();
        let mut wip_registry = package::WipRegistry::default();

        self.propagate_defaults()?;
        self.type_check(&mut wip_registry)?;
        self.create_tracking_variables(&optimizer, &mut wip_registry)?;

        let mut registry = wip_registry.build();

        self.handle_explicit_options(&optimizer, &mut registry)?;
        self.require_packages(&optimizer, &mut registry)?;
        self.add_constraints(&optimizer, &mut registry)?;

        Ok((optimizer, registry))
    }
}

#[pymethods]
impl PackageOutline {
    #[new]
    #[pyo3(signature = (name, constraints=None, set_options=None, set_defaults=None, **kwargs))]
    pub fn py_new(
        name: &str,
        constraints: Option<Vec<Constraint>>,
        set_options: Option<HashMap<String, spec::SpecOptionValue>>,
        set_defaults: Option<HashMap<String, Option<spec::SpecOptionValue>>>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let mut instance = Self {
            name: name.to_string(),
            constraints: constraints.unwrap_or_default(),
            set_options: set_options.unwrap_or_default(),
            set_defaults: set_defaults.unwrap_or_default(),
        };

        if let Some(kwargs) = kwargs {
            for (key, value) in kwargs {
                match key.extract::<&str>()? {
                    "constraints" => instance.constraints = value.extract()?,
                    "set_options" => instance.set_options = value.extract()?,
                    "set_defaults" => {
                        instance.set_defaults = value.extract()?
                    }
                    _ => {
                        tracing::error!(
                            "PyPackageOutline unexpected keyword argument '{key}'"
                        );

                        return Err(PyValueError::new_err(format!(
                            "PyPackageOutline unexpected keyword argument '{key}'"
                        )));
                    }
                }
            }
        }

        Ok(instance)
    }
}

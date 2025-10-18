//! The package outline is the loose description of the versions, options,
//! dependencies, conflicts, etc. for a given package. This outline is then
//! refined with information from the package configuration options provided
//! from a configuration file or the command line.
//!
//! This outline definition is then passed to the `Planner`, which solves for
//! a concrete, satisfiable set of dependencies and options which can then be
//! built and installed.

use std::{
    collections::{HashMap, hash_map},
    sync::{Arc, Mutex},
};

use petgraph::{algo::Cycle, graph::DiGraph, visit::EdgeRef};
use pyo3::{exceptions::PyValueError, prelude::*, types::PyDict};
use z3::{Optimize, SortKind};

use super::constraint::Constraint;
use crate::{
    package::{
        constraint::{self, ConstraintType, SOFT_PACKAGE_WEIGHT},
        registry::{Registry, WipRegistry},
    },
    spec,
};

pub type PackageDiGraph = DiGraph<PackageOutline, u8>;
pub type SpecMap = HashMap<String, Option<spec::SpecOptionValue>>;

#[pyclass]
#[derive(Clone, Debug, Default)]
pub struct PackageOutline {
    #[pyo3(get, set)]
    pub name: String,

    #[pyo3(get, set)]
    pub constraints: Vec<Box<dyn Constraint>>,

    #[pyo3(get, set)]
    pub set_options: HashMap<String, spec::SpecOptionValue>,

    #[pyo3(get, set)]
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

#[derive(Debug)]
pub enum PropagateDefaultError {
    Cycle(Cycle<<PackageDiGraph as petgraph::visit::GraphBase>::NodeId>),
    Conflict {
        package_name: String,
        default_name: String,
        first_setter: String,
        first_value: spec::SpecOptionValue,
        conflict_setter: String,
        conflict_value: spec::SpecOptionValue,
    },
}

#[derive(Clone, Debug)]
pub enum GenSpecSolverError {
    DuplicateOption(String),
    InvalidConstraint(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SolverError {
    DuplicateOption(String),
    MissingDependency {
        dep: String,
    },
    MissingVariable {
        package: String,
        name: String,
    },
    IncorrectZ3Type {
        expected: SortKind,
        received: SortKind,
    },
    IncorrectConstraintType {
        expected: ConstraintType,
        received: ConstraintType,
    },
    InvalidConstraint(String),
}

impl SpecOutline {
    pub fn new(outlines: Vec<PackageOutline>) -> Result<Self, SolverError> {
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

                        SolverError::MissingDependency { dep: dep.clone() }
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
    pub fn propagate_defaults(
        &mut self,
    ) -> Result<(), Box<PropagateDefaultError>> {
        use petgraph::algo::toposort;

        tracing::info!("propagating default values");

        let mut reason_tracker = HashMap::<(String, String), String>::new();

        let sorted = toposort(&self.graph, None)
            .map_err(PropagateDefaultError::Cycle)?;

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

                    let Some(src_val) = src_val else {
                        tracing::warn!(
                            "Top-level package '{}' has default option '{}' with value None. This has no effect; consider removing it",
                            &src_name,
                            opt_name
                        );
                        continue;
                    };

                    if dep.set_defaults.contains_key(opt_name) {
                        match &dep.set_defaults[opt_name] {
                            Some(old_val) => {
                                if let Some(reason) = reason_tracker
                                    .get(&(dep.name.clone(), opt_name.clone()))
                                {
                                    if old_val != src_val {
                                        // Conflict

                                        tracing::error!(
                                            "conflicting default values detected"
                                        );

                                        let e =
                                            PropagateDefaultError::Conflict {
                                                package_name: dep.name.clone(),
                                                default_name: opt_name.clone(),
                                                first_setter: reason.clone(),
                                                first_value: old_val.clone(),
                                                conflict_setter:
                                                    match reason_tracker.get(&(
                                                        src_name.clone(),
                                                        opt_name.clone(),
                                                    )) {
                                                        Some(val) => {
                                                            val.clone()
                                                        }
                                                        None => {
                                                            src_name.clone()
                                                        }
                                                    },
                                                conflict_value: src_val.clone(),
                                            };

                                        return Err(Box::new(e));
                                    }
                                }
                            }
                            None => {
                                dep.set_defaults.remove(opt_name);
                            }
                        }
                    } else {
                        // Insert and track default

                        dep.set_defaults
                            .insert(opt_name.clone(), Some(src_val.clone()));

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

        Ok(())
    }

    pub fn create_tracking_variables<'a>(
        &'a self,
        optimizer: &Optimize,
        wip_registry: &mut WipRegistry<'a>,
    ) -> Result<(), SolverError>
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

            wip_registry
                .option_ast_map
                .insert((&package.name, None), package_toggle.into());

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

                let val = value.to_z3_dynamic(
                    package_name,
                    option_name,
                    wip_registry,
                );

                wip_registry
                    .option_ast_map
                    .insert((package_name, Some(option_name)), val);
            }
        }

        Ok(())
    }

    pub fn handle_explicit_options<'a>(
        &'a self,
        optimizer: &Optimize,
        registry: &Registry<'a>,
    ) -> Result<(), SolverError>
    where
        Self: 'a,
    {
        for idx in self.graph.node_indices() {
            let package = &self.graph[idx];

            for (name, value) in &package.set_options {
                tracing::info!(
                    "adding explicit value {}:{name} -> {value:?}",
                    package.name
                );

                let eq: Box<dyn Constraint> = Box::new(constraint::Equal {
                    lhs: Box::new(constraint::SpecOption {
                        package_name: package.name.clone(),
                        option_name: name.clone(),
                    }),

                    rhs: Box::new(constraint::Value { value: value.clone() }),
                });

                optimizer.assert_and_track(
                    &registry.option_ast_map[&(package.name.as_ref(), None)]
                        .as_bool()
                        .unwrap() // Safe because package toggle guaranteed to exist
                        .implies(
                            eq.to_z3_clause(&registry)
                                .unwrap()
                                .as_bool()
                                .unwrap(),
                        ),
                    &z3::ast::Bool::new_const(format!("{value:?}")),
                );
            }
        }

        Ok(())
    }

    pub fn require_packages<'a>(
        &'a self,
        optimizer: &Optimize,
        registry: &Registry<'a>,
    ) -> Result<(), SolverError>
    where
        Self: 'a,
    {
        for r in &self.required {
            let Some(d) = registry.option_ast_map.get(&(r.as_str(), None))
            else {
                tracing::error!("missing explicitly required dependency '{r}'");

                return Err(SolverError::MissingDependency { dep: r.clone() });
            };

            optimizer.assert_and_track(
                &d.as_bool().unwrap(),
                &z3::ast::Bool::new_const(format!("'{r}' required explicitly")),
            );
        }

        Ok(())
    }

    pub fn add_constraints<'a>(
        &'a self,
        optimizer: &Optimize,
        registry: &Registry<'a>,
    ) -> Result<(), SolverError>
    where
        Self: 'a,
    {
        for idx in self.graph.node_indices() {
            let package = &self.graph[idx];

            tracing::info!("adding constraints for {}", package.name);

            for constraint in &package.constraints {
                tracing::info!(
                    "adding constraint {} -> {:?}",
                    package.name,
                    constraint
                );

                let package_toggle = &registry.option_ast_map
                    [&(package.name.as_str(), None)]
                    .as_bool()
                    .unwrap();

                let clause = constraint.to_z3_clause(registry)?;

                match clause.sort_kind() {
                    SortKind::Bool => {
                        optimizer.assert_and_track(
                            &package_toggle.implies(clause.as_bool().unwrap()),
                            &z3::ast::Bool::new_const(format!(
                                "{constraint:?}"
                            )),
                        );
                    }
                    kind => {
                        tracing::error!("clause must be Bool");

                        return Err(SolverError::IncorrectZ3Type {
                            expected: SortKind::Bool,
                            received: kind,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    pub fn type_check<'a>(
        &'a self,
        wip_registry: &mut WipRegistry<'a>,
    ) -> Result<(), SolverError> {
        for idx in self.graph.node_indices() {
            let package = &self.graph[idx];

            tracing::info!("checking types for package '{}'", package.name);

            for constraint in &package.constraints {
                tracing::info!(
                    "checking types for constraint '{constraint:?}'"
                );

                constraint.type_check(wip_registry)?;
            }
        }

        Ok(())
    }

    pub fn gen_spec_solver(
        &self,
    ) -> Result<(Optimize, Registry<'_>), SolverError> {
        tracing::info!("generating spec solver");

        let optimizer = Optimize::new();
        let mut wip_registry = WipRegistry::new();

        self.type_check(&mut wip_registry)?;

        println!("Type Registry: {:?}", wip_registry.option_type_map);

        self.create_tracking_variables(&optimizer, &mut wip_registry)?;

        tracing::error!("Version Registry: {:?}", wip_registry.versions);

        let registry = wip_registry.build();

        self.handle_explicit_options(&optimizer, &registry)?;
        self.require_packages(&optimizer, &registry)?;
        self.add_constraints(&optimizer, &registry)?;

        Ok((optimizer, registry))
    }
}

#[pymethods]
impl PackageOutline {
    #[new]
    #[pyo3(signature = (name, constraints=None, set_options=None, set_defaults=None, **kwargs))]
    pub fn py_new(
        name: &str,
        constraints: Option<Vec<Box<dyn Constraint>>>,
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

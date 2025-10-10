//! The package outline is the loose description of the versions, options,
//! dependencies, conflicts, etc. for a given package. This outline is then
//! refined with information from the package configuration options provided
//! from a configuration file or the command line.
//!
//! This outline definition is then passed to the [`Planner`], which solves for
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
    package::constraint::{SOFT_PACKAGE_WEIGHT, SpecOptionEqual},
    spec::spec_option::{PackageOptionAstMap, SpecOptionValue},
};

pub type PackageDiGraph = DiGraph<PackageOutline, u8>;
pub type SpecMap = HashMap<String, Option<SpecOptionValue>>;

#[pyclass]
#[derive(Clone, Debug, Default)]
pub struct PackageOutline {
    #[pyo3(get, set)]
    pub name: String,

    #[pyo3(get, set)]
    pub constraints: Vec<Box<dyn Constraint>>,

    #[pyo3(get, set)]
    pub set_options: HashMap<String, SpecOptionValue>,

    #[pyo3(get, set)]
    pub set_defaults: HashMap<String, Option<SpecOptionValue>>,
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
        first_value: SpecOptionValue,
        conflict_setter: String,
        conflict_value: SpecOptionValue,
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
    MissingDependency { package: String, dep: String },
    MissingVariable { package: String, name: String },
    IncorrectType { expected: SortKind, received: SortKind },
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

                        SolverError::MissingDependency {
                            package: src_name.clone(),
                            dep: dep.clone(),
                        }
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

    pub fn gen_spec_solver(
        &mut self,
    ) -> Result<(Optimize, PackageOptionAstMap<'_>), SolverError> {
        tracing::info!("generating spec solver");

        let mut option_asts = PackageOptionAstMap::new();

        let optimizer = Optimize::new();

        let mut additional_constraints = Vec::new();

        for idx in self.graph.node_indices() {
            let package = &self.graph[idx];

            tracing::info!("creating activation toggle for {}", package.name);

            // Whether the package is enabled.
            // This variable implies all the package's constraints are true,
            // allowing us to effectively toggle the package on and off.

            let package_toggle =
                z3::ast::Bool::new_const(format!("{}", package.name));

            optimizer.assert_soft(
                &package_toggle.not(),
                SOFT_PACKAGE_WEIGHT,
                None,
            );

            option_asts.insert((&package.name, None), package_toggle.into());

            // Create variables for each package option
            for (name, value) in package
                .constraints
                .iter()
                .flat_map(|c| c.extract_spec_options(&package.name))
            {
                tracing::info!(
                    "creating variable for {}:{}",
                    package.name,
                    name
                );

                option_asts.insert(
                    (&package.name, Some(name)),
                    value.to_z3_dynamic(&package.name, name),
                );
            }

            for (name, value) in &package.set_options {
                tracing::info!(
                    "adding explicit value {}:{name} -> {value:?}",
                    package.name
                );

                additional_constraints.push((
                    idx,
                    Box::new(SpecOptionEqual {
                        package_name: None,
                        option_name: name.clone(),
                        equal_to: value.clone(),
                    }),
                ));
            }
        }

        for (idx, value) in additional_constraints {
            let package = &self.graph[idx];

            optimizer.assert_and_track(
                &option_asts[&(package.name.as_ref(), None)]
                    .as_bool()
                    .unwrap() // Safe because package toggle guaranteed to exist
                    .implies(
                        value
                            .to_z3_clause(&package.name, &option_asts)
                            .unwrap()
                            .as_bool()
                            .unwrap(),
                    ),
                &z3::ast::Bool::new_const(format!("{value:?}")),
            );
        }

        for r in &self.required {
            let Some(d) = option_asts.get(&(r.as_str(), None)) else {
                tracing::error!("missing explicitly required dependency '{r}'");

                return Err(SolverError::MissingDependency {
                    package: "REQUIRED".to_string(),
                    dep: r.clone(),
                });
            };

            optimizer.assert_and_track(
                &d.as_bool().unwrap(),
                &z3::ast::Bool::new_const(format!(
                    "'{r}' reqauired explicitly"
                )),
            );
        }

        // Add constraints to the solver
        for idx in self.graph.node_indices() {
            let package = &self.graph[idx];

            tracing::info!("adding constraints for {}", package.name,);

            for constraint in &package.constraints {
                tracing::info!(
                    "adding constraint {} -> {:?}",
                    package.name,
                    constraint
                );

                let package_toggle = option_asts
                    [&(package.name.as_str(), None)]
                    .as_bool()
                    .unwrap();

                let clause =
                    constraint.to_z3_clause(&package.name, &option_asts)?;

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

                        return Err(SolverError::IncorrectType {
                            expected: SortKind::Bool,
                            received: kind,
                        });
                    }
                }
            }
        }

        Ok((optimizer, option_asts))
    }
}

// #[pyclass(name = "PackageOutline")]
// #[derive(Debug, Clone)]
// pub struct PyPackageOutline {
//     #[pyo3(get, set)]
//     pub name: String,
//
//     #[pyo3(get, set)]
//     pub constraints: Vec<Py<PyAny>>,
//
//     #[pyo3(get, set)]
//     pub set_options: HashMap<String, SpecOptionValue>,
//
//     #[pyo3(get, set)]
//     pub set_defaults: HashMap<String, Option<SpecOptionValue>>,
// }

#[pymethods]
impl PackageOutline {
    #[new]
    #[pyo3(signature = (name, constraints=None, set_options=None, set_defaults=None, **kwargs))]
    pub fn py_new(
        name: &str,
        constraints: Option<Vec<Box<dyn Constraint>>>,
        set_options: Option<HashMap<String, SpecOptionValue>>,
        set_defaults: Option<HashMap<String, Option<SpecOptionValue>>>,
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

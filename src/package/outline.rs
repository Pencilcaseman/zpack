//! The package outline is the loose description of the versions, options,
//! dependencies, conflicts, etc. for a given package. This outline is then
//! refined with information from the package configuration options provided
//! from a configuration file or the command line.
//!
//! This outline definition is then passed to the [`Planner`], which solves for
//! a concrete, satisfiable set of dependencies and options which can then be
//! built and installed.

use std::collections::HashMap;

use petgraph::{algo::Cycle, graph::DiGraph, visit::EdgeRef};
use z3::{Optimize, Solver};

use super::constraint::{Constraint, ZPACK_ACTIVE_STR};
use crate::{
    package::constraint::SOFT_PACKAGE_WEIGHT,
    spec::spec_option::{SpecOption, SpecOptionValue},
};

pub type PackageDiGraph = DiGraph<PackageOutline, u8>;
pub type SpecMap = HashMap<String, Option<SpecOptionValue>>;
pub type PackageOptionAstMap<'a> =
    HashMap<(&'a str, &'a str), z3::ast::Dynamic>;

#[derive(Debug, Default)]
pub struct PackageOutline {
    pub name: String,
    pub options: HashMap<String, SpecOption>,
    pub constraints: Vec<Box<dyn Constraint>>,
    pub defaults: HashMap<String, Option<SpecOptionValue>>,
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

impl SpecOutline {
    pub fn new(outlines: Vec<PackageOutline>) -> Self {
        let mut lookup = HashMap::new();
        let mut graph = PackageDiGraph::new();

        for outline in outlines {
            let name = outline.name.clone();
            let idx = graph.add_node(outline);
            lookup.insert(name, idx);
        }

        let mut edges = Vec::new();

        for src in graph.node_indices() {
            for dep in &graph[src].dependencies() {
                let dst = lookup[dep];
                edges.push((src, dst));
            }
        }

        graph.extend_with_edges(edges);

        let required = Vec::new();

        Self { graph, lookup, required }
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
            let src_defaults = self.graph[idx].defaults.clone();

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

                    if dep.defaults.contains_key(opt_name) {
                        match &dep.defaults[opt_name] {
                            Some(old_val) => {
                                if let Some(reason) = reason_tracker
                                    .get(&(dep.name.clone(), opt_name.clone()))
                                {
                                    if old_val != src_val {
                                        // Conflict

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
                                dep.defaults.remove(opt_name);
                            }
                        }
                    } else {
                        // Insert and track default

                        dep.defaults
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
        &self,
    ) -> Result<(Optimize, PackageOptionAstMap<'_>), GenSpecSolverError> {
        tracing::info!("generating spec solver");

        let mut option_asts = HashMap::<(&str, &str), z3::ast::Dynamic>::new();

        let optimizer = Optimize::new();

        for idx in self.graph.node_indices() {
            let package = &self.graph[idx];

            tracing::info!("creating activation toggle for {}", package.name);

            // Whether the package is enabled.
            // This variable implies all the package's constraints are true,
            // allowing us to effectively toggle the package on and off.

            let package_toggle = z3::ast::Bool::new_const(format!(
                "{}:__zpack_active",
                package.name
            ));

            optimizer.assert_soft(
                &package_toggle.not(),
                SOFT_PACKAGE_WEIGHT,
                None,
            );

            option_asts.insert(
                (&package.name, ZPACK_ACTIVE_STR),
                package_toggle.into(),
            );

            // Create variables for each package option
            for (name, value) in package.options.iter() {
                tracing::info!(
                    "creating variable for {}:{}",
                    package.name,
                    name
                );

                let exists = option_asts
                    .insert(
                        (&package.name, name),
                        value.to_z3_dynamic(&package.name, name),
                    )
                    .is_some();

                if exists {
                    return Err(GenSpecSolverError::DuplicateOption(
                        name.clone(),
                    ));
                }
            }
        }

        for required in &self.required {
            optimizer.assert(
                &option_asts[&(required.as_str(), ZPACK_ACTIVE_STR)]
                    .as_bool()
                    .unwrap(),
            );
        }

        // Add constraints to the solver
        for idx in self.graph.node_indices() {
            let package = &self.graph[idx];

            tracing::info!("adding constraints for {}", package.name,);

            for constraint in &package.constraints {
                tracing::info!(
                    "adding constraint {}:{:?}",
                    package.name,
                    constraint
                );

                let package_toggle = option_asts
                    [&(package.name.as_str(), ZPACK_ACTIVE_STR)]
                    .as_bool()
                    .unwrap();

                match constraint.to_z3_clause(&package.name, &option_asts) {
                    Some(c) => optimizer.assert_and_track(
                        &package_toggle.implies(c.as_bool().unwrap()),
                        &z3::ast::Bool::new_const(format!("{constraint:?}")),
                    ),
                    None => {
                        return Err(GenSpecSolverError::InvalidConstraint(
                            package.name.clone(),
                        ));
                    }
                }
            }
        }

        Ok((optimizer, option_asts))
    }
}

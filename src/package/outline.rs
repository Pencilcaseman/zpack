//! The package outline is the loose description of the versions, options,
//! dependencies, conflicts, etc. for a given package. This outline is then
//! refined with information from the package configuration options provided
//! from a configuration file or the command line.
//!
//! This outline definition is then passed to the [`Planner`], which solves for
//! a concrete, satisfiable set of dependencies and options which can then be
//! built and installed.

use std::{cell::Cell, collections::HashMap, rc::Rc};

use petgraph::{
    Graph,
    algo::Cycle,
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
};

use crate::spec::spec_option::SpecOption;

#[derive(Clone, Debug)]
pub struct Constraint;

pub type PackageDiGraph = DiGraph<PackageOutline, u8>;
pub type SpecMap = HashMap<String, Option<SpecOption>>;

#[derive(Clone, Debug, Default)]
pub struct PackageOutline {
    pub name: String,
    pub options: SpecMap,
    pub constraints: Vec<Constraint>,
    pub dependencies: Vec<String>,
    pub defaults: SpecMap,
}

impl std::fmt::Display for PackageOutline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

pub struct SpecOutline {
    pub graph: PackageDiGraph,
    pub lookup: HashMap<String, petgraph::graph::NodeIndex>,
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
            for dep in &graph[src].dependencies {
                let dst = lookup[dep];
                edges.push((src, dst));
            }
        }

        graph.extend_with_edges(edges);

        Self { graph, lookup }
    }

    pub fn propagate_defaults(
        &mut self,
    ) -> Result<(), Cycle<<PackageDiGraph as petgraph::visit::GraphBase>::NodeId>>
    {
        use petgraph::algo::toposort;

        let sorted = toposort(&self.graph, None)?;

        for idx in sorted {
            let src = self.graph[idx].clone();

            let deps: Vec<_> = self
                .graph
                .edges_directed(idx, petgraph::Direction::Outgoing)
                .map(|e| e.target())
                .collect();

            for dep in deps {
                let dep = &mut self.graph[dep];

                for (key, val) in src.defaults.iter() {
                    if !dep.defaults.contains_key(key) {
                        dep.defaults.insert(key.clone(), val.clone());
                    } else if dep.defaults[key].is_none() {
                        dep.defaults.remove(key);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn source_nodes(&self) -> Option<Vec<NodeIndex>> {
        if petgraph::algo::is_cyclic_directed(&self.graph) {
            tracing::error!(
                "Graph contains a cycle. Cannot extract source nodes",
            );
            return None;
        }

        fn helper(
            graph: &PackageDiGraph,
            node: NodeIndex,
            out: &mut Vec<NodeIndex>,
        ) {
            let mut count = 0;

            for edge in
                graph.edges_directed(node, petgraph::Direction::Incoming)
            {
                helper(graph, edge.target(), out);
                count += 1;
            }

            // No incoming edges
            if count == 0 {
                out.push(node);
            }
        }

        let mut res = Vec::new();
        helper(&self.graph, NodeIndex::new(0), &mut res);
        Some(res)
    }
}

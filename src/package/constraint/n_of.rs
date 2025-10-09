use std::collections::{HashMap, HashSet};

use z3::ast::Bool;

use super::Constraint;
use crate::spec::spec_option::SpecOption;

#[derive(Debug)]
pub struct NOf {
    pub n: i32,
    pub of: Vec<Box<dyn Constraint>>,
}

impl Constraint for NOf {
    fn extract_spec_options(&self, package: &str) -> HashMap<&str, SpecOption> {
        self.of.iter().flat_map(|c| c.extract_spec_options(package)).collect()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        self.of.iter().flat_map(|b| b.extract_dependencies()).collect()
    }

    fn to_z3_clause<'a>(
        &self,
        package: &str,
        option_ast: &std::collections::HashMap<
            (&'a str, &'a str),
            z3::ast::Dynamic,
        >,
    ) -> Option<z3::ast::Dynamic> {
        tracing::info!(
            "{}->(exactly {} of {} constraints)",
            package,
            self.n,
            self.of.len()
        );

        // Ensure exactly n of the conditions are met and separately ensure the
        // implications of each condition are met

        let mut clauses = Vec::new();
        let mut implications = Vec::new();

        for constraint in &self.of {
            clauses.push((constraint.as_cond(package, option_ast)?, 1));
            implications
                .push(constraint.to_z3_clause(package, option_ast)?.as_bool()?);
        }

        let refs = clauses.iter().map(|(b, m)| (b, *m)).collect::<Vec<_>>();
        let mut constraints = vec![Bool::pb_eq(&refs, self.n)];
        constraints.extend(implications);

        Some(Bool::and(&constraints).into())
    }
}

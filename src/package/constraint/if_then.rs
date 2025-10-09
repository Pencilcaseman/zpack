use std::collections::{HashMap, HashSet};

use super::Constraint;
use crate::spec::spec_option::SpecOption;

#[derive(Debug)]
pub struct IfThen {
    pub cond: Box<dyn Constraint>,
    pub then: Box<dyn Constraint>,
}

impl Constraint for IfThen {
    fn extract_spec_options(&self, package: &str) -> HashMap<&str, SpecOption> {
        [&self.cond, &self.then]
            .iter()
            .flat_map(|c| c.extract_spec_options(package))
            .collect()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        [&self.cond, &self.then]
            .iter()
            .flat_map(|c| c.extract_dependencies())
            .collect()
    }

    fn as_cond<'a>(
        &self,
        package: &str,
        option_ast: &std::collections::HashMap<
            (&'a str, &'a str),
            z3::ast::Dynamic,
        >,
    ) -> Option<z3::ast::Bool> {
        self.cond.to_z3_clause(package, option_ast).map(|v| v.as_bool())?
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
            "{}->(if '{:?}' then '{:?}')",
            package,
            self.cond,
            self.then
        );

        let Some(cond) = self.cond.as_cond(package, option_ast) else {
            tracing::error!("invalid `cond` in IfThen");
            return None;
        };

        let Some(then) = self.then.as_cond(package, option_ast) else {
            tracing::error!("invalid `then` in IfThen");
            return None;
        };

        Some(cond.implies(then).into())
    }
}

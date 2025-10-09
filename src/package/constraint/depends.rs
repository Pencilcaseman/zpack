use std::collections::{HashMap, HashSet};

use z3::{SortKind, ast::Ast};

use super::{Constraint, ZPACK_ACTIVE_STR};
use crate::spec::spec_option::SpecOption;

#[derive(Clone, Debug)]
pub struct Depends(pub String);

impl Constraint for Depends {
    fn extract_spec_options(&self, package: &str) -> HashMap<&str, SpecOption> {
        HashMap::default()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        HashSet::from([self.0.clone()])
    }

    fn to_z3_clause<'a>(
        &self,
        _package: &str,
        option_ast: &std::collections::HashMap<
            (&'a str, &'a str),
            z3::ast::Dynamic,
        >,
    ) -> Option<z3::ast::Dynamic> {
        let value = match option_ast.get(&(&self.0, ZPACK_ACTIVE_STR)) {
            Some(v) => v,
            None => {
                tracing::error!(
                    "package '{}' has no activation variable",
                    self.0
                );
                return None;
            }
        };

        if value.sort_kind() != SortKind::Bool {
            tracing::error!(
                "package activation variable '{}' is not of type Bool",
                self.0
            );
            None
        } else {
            Some(value.clone())
        }
    }
}

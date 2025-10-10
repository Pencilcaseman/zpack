use std::collections::{HashMap, HashSet};

use z3::{Sort, SortKind, ast::Ast};

use super::Constraint;
use crate::{
    package::outline::SolverError,
    spec::spec_option::{PackageOptionAstMap, SpecOption},
};

#[derive(Clone, Debug)]
pub struct Depends(pub String);

impl Constraint for Depends {
    fn extract_spec_options(
        &self,
        _package: &str,
    ) -> HashMap<&str, SpecOption> {
        HashMap::default()
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        HashSet::from([self.0.clone()])
    }

    fn to_z3_clause<'a>(
        &self,
        package: &str,
        option_ast: &PackageOptionAstMap<'a>,
    ) -> Result<z3::ast::Dynamic, SolverError> {
        let Some(value) = option_ast.get(&(&self.0, None)) else {
            tracing::error!("package '{}' has no activation variable", self.0);

            return Err(SolverError::MissingDependency {
                package: package.to_string(),
                dep: self.0.clone(),
            });
        };

        match value.sort_kind() {
            SortKind::Bool => Ok(value.clone()),
            kind => {
                tracing::error!(
                    "package activation variable '{}' is not of type Bool",
                    self.0
                );

                Err(SolverError::IncorrectType {
                    expected: SortKind::Bool,
                    received: kind,
                })
            }
        }
    }
}

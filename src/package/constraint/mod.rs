use std::collections::{HashMap, HashSet};

use z3::SortKind;

use crate::{
    package::outline::SolverError,
    spec::spec_option::{PackageOptionAstMap, SpecOption},
};

pub const SOFT_PACKAGE_WEIGHT: usize = 1;

pub trait Constraint: std::fmt::Debug {
    fn extract_spec_options(&self, package: &str) -> HashMap<&str, SpecOption>;
    fn extract_dependencies(&self) -> HashSet<String>;

    fn as_cond<'a>(
        &self,
        package: &str,
        option_ast: &PackageOptionAstMap<'a>,
    ) -> Result<z3::ast::Bool, SolverError> {
        let val = self.to_z3_clause(package, option_ast)?;
        val.as_bool().ok_or(SolverError::IncorrectType {
            expected: SortKind::Bool,
            received: val.sort_kind(),
        })
    }

    fn to_z3_clause<'a>(
        &self,
        package: &str,
        option_ast: &PackageOptionAstMap<'a>,
    ) -> Result<z3::ast::Dynamic, SolverError>;
}

pub mod depends;
pub mod if_then;
pub mod n_of;
pub mod spec_option;

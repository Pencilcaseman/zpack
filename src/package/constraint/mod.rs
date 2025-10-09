use std::collections::{HashMap, HashSet};

use crate::spec::spec_option::SpecOption;

pub const ZPACK_ACTIVE_STR: &str = "__zpack_active";
pub const SOFT_PACKAGE_WEIGHT: usize = 100;

pub trait Constraint: std::fmt::Debug {
    fn extract_spec_options(&self, package: &str) -> HashMap<&str, SpecOption>;
    fn extract_dependencies(&self) -> HashSet<String>;

    fn as_cond<'a>(
        &self,
        package: &str,
        option_ast: &HashMap<(&'a str, &'a str), z3::ast::Dynamic>,
    ) -> Option<z3::ast::Bool> {
        self.to_z3_clause(package, option_ast)?.as_bool()
    }

    fn to_z3_clause<'a>(
        &self,
        package: &str,
        option_ast: &HashMap<(&'a str, &'a str), z3::ast::Dynamic>,
    ) -> Option<z3::ast::Dynamic>;
}

pub mod depends;
pub mod if_then;
pub mod n_of;
pub mod spec_option;

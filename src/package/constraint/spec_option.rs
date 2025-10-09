use std::collections::{HashMap, HashSet};

use crate::{
    package::constraint::Constraint,
    spec::spec_option::{SpecOption, SpecOptionValue},
};

#[derive(Clone, Debug)]
pub struct SpecOptionEqual {
    pub package_name: Option<String>, // None => Caller
    pub option_name: String,
    pub equal_to: SpecOptionValue,
}

impl Constraint for SpecOptionEqual {
    fn extract_spec_options(&self, package: &str) -> HashMap<&str, SpecOption> {
        if self.package_name.as_ref().is_none_or(|p| package == p) {
            HashMap::from([(
                self.option_name.as_ref(),
                SpecOption {
                    dtype: self.equal_to.to_type(),
                    value: Some(self.equal_to.clone()),
                    default: None,
                    valid: None,
                },
            )])
        } else {
            HashMap::default()
        }
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        HashSet::default()
    }

    fn to_z3_clause<'a>(
        &self,
        package: &str,
        option_ast: &std::collections::HashMap<
            (&'a str, &'a str),
            z3::ast::Dynamic,
        >,
    ) -> Option<z3::ast::Dynamic> {
        let package_name = match &self.package_name {
            Some(name) => name,
            None => package,
        };

        tracing::info!(
            "{package_name}:{} == {:?}",
            self.option_name,
            self.equal_to
        );

        Some(
            option_ast[&(package_name, self.option_name.as_str())]
                .eq(self.equal_to.to_z3_dynamic())
                .into(),
        )
    }
}

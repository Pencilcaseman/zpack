use std::collections::{HashMap, HashSet};

use crate::{
    package::{constraint::Constraint, outline::SolverError},
    spec::spec_option::{PackageOptionAstMap, SpecOption, SpecOptionValue},
};

#[derive(Clone, Debug)]
pub struct SpecOptionEqual {
    pub package_name: Option<String>,
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
        option_ast: &PackageOptionAstMap,
    ) -> Result<z3::ast::Dynamic, SolverError> {
        let package_name = match &self.package_name {
            Some(name) => name,
            None => package,
        };

        tracing::info!(
            "{package_name}:{} == {:?}",
            self.option_name,
            self.equal_to
        );

        match option_ast.get(&(package_name, Some(self.option_name.as_str()))) {
            Some(var) => Ok(var.eq(self.equal_to.to_z3_dynamic()).into()),
            None => {
                if option_ast.contains_key(&(package_name, None)) {
                    tracing::error!(
                        "missing variable {package_name}:{}",
                        self.option_name
                    );

                    Err(SolverError::MissingVariable {
                        package: package_name.to_string(),
                        name: self.option_name.clone(),
                    })
                } else {
                    tracing::error!("missing package {package_name}");

                    Err(SolverError::MissingDependency {
                        package: package.to_string(),
                        dep: package_name.to_string(),
                    })
                }
            }
        }
    }
}

use super::Constraint;

#[derive(Debug)]
pub struct IfThen {
    pub cond: Box<dyn Constraint>,
    pub then: Box<dyn Constraint>,
}

impl Constraint for IfThen {
    fn extract_dependencies(&self) -> Vec<String> {
        let mut res = Vec::new();
        res.extend(self.cond.extract_dependencies());
        res.extend(self.then.extract_dependencies());
        res
    }

    fn to_z3_bool<'a>(
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

        let Some(cond) = self.cond.to_z3_bool(package, option_ast) else {
            tracing::error!("invalid `cond` in IfThen");
            return None;
        };

        let Some(then) = self.then.to_z3_bool(package, option_ast) else {
            tracing::error!("invalid `then` in IfThen");
            return None;
        };

        Some(cond.implies(then).into())
    }
}

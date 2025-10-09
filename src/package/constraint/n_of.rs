use super::Constraint;

#[derive(Debug)]
pub struct NOf {
    pub n: i32,
    pub of: Vec<Box<dyn Constraint>>,
}

impl Constraint for NOf {
    fn extract_dependencies(&self) -> Vec<String> {
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

        let mut bool_clauses = Vec::new();

        for constraint in &self.of {
            // let clause = constraint.to_z3_clause(package, option_ast)?;
            bool_clauses.push((constraint.to_z3_bool(package, option_ast)?, 1));
        }

        let refs =
            bool_clauses.iter().map(|(b, m)| (b, *m)).collect::<Vec<_>>();
        Some(z3::ast::Bool::pb_eq(&refs, self.n).into())
    }
}

use z3::ast::Bool;

pub trait Constraint: std::fmt::Debug {
    fn to_z3_variables(&self) -> Vec<()>;
    fn to_z3_clause(&self) -> impl Into<Bool>;
}

#[derive(Debug)]
pub struct Depends {
    pub package_name: String,
}

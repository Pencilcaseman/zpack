use std::collections::HashSet;

use pyo3::{
    IntoPyObjectExt, basic::CompareOp, exceptions::PyNotImplementedError,
    prelude::*,
};
use z3::Solver;

use crate::{
    package::{
        self,
        constraint::{Constraint, ConstraintUtils, IfThen},
        outline::SolverError,
        registry::BuiltVersionRegistry,
    },
    spec::{self, SpecOptionType},
};

#[pyclass]
#[derive(Debug, Copy, Clone)]
pub enum CmpType {
    Less,
    LessOrEqual,
    NotEqual,
    Equal,
    GreaterOrEqual,
    Greater,
}

impl From<CompareOp> for CmpType {
    fn from(value: CompareOp) -> Self {
        match value {
            CompareOp::Lt => Self::Less,
            CompareOp::Le => Self::LessOrEqual,
            CompareOp::Eq => Self::Equal,
            CompareOp::Ne => Self::NotEqual,
            CompareOp::Gt => Self::Greater,
            CompareOp::Ge => Self::GreaterOrEqual,
        }
    }
}

impl std::fmt::Display for CmpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CmpType::Less => "<",
            CmpType::LessOrEqual => "<=",
            CmpType::NotEqual => "!=",
            CmpType::Equal => "==",
            CmpType::GreaterOrEqual => ">=",
            CmpType::Greater => ">",
        })
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Cmp {
    #[pyo3(get, set)]
    pub lhs: Constraint,

    #[pyo3(get, set)]
    pub rhs: Constraint,

    #[pyo3(get, set)]
    pub op: CmpType,
}

impl Cmp {
    pub fn can_cmp(t: SpecOptionType, op: CmpType) -> bool {
        match op {
            CmpType::Less
            | CmpType::LessOrEqual
            | CmpType::GreaterOrEqual
            | CmpType::Greater => match t {
                SpecOptionType::Bool => false,

                SpecOptionType::Unknown
                | SpecOptionType::Int
                | SpecOptionType::Float
                | SpecOptionType::Str
                | SpecOptionType::Version => true,
            },

            CmpType::NotEqual | CmpType::Equal => true,
        }
    }

    pub(crate) fn py_richcmp_helper(
        lhs: Constraint,
        rhs: Constraint,
        op: CmpType,
    ) -> Result<Constraint, PyErr> {
        let lhs_type = lhs.get_value_type_default();
        let rhs_type = rhs.get_value_type_default();

        if let (Some(lhs_type), Some(rhs_type)) = (lhs_type, rhs_type)
            && lhs_type == rhs_type
            && Cmp::can_cmp(lhs_type, op)
        {
            Ok(Cmp { lhs, rhs, op }.into())
        } else {
            Err(PyNotImplementedError::new_err(format!(
                "Cannot compare type {lhs_type:?} from constraint {lhs} with type {rhs_type:?} from constraint {rhs} using operator '{op}'"
            )))
        }
    }
}

impl ConstraintUtils for Cmp {
    fn get_value_type<'a, V>(
        &'a self,
        _registry: Option<&package::registry::Registry<'a, V>>,
    ) -> Option<spec::SpecOptionType> {
        Some(spec::SpecOptionType::Bool)
    }

    fn set_value_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
        value_type: spec::SpecOptionType,
    ) {
        self.lhs.set_value_type(wip_registry, value_type);
        self.rhs.set_value_type(wip_registry, value_type);
    }

    #[tracing::instrument(skip(self, wip_registry))]
    fn type_check<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        let Some(lhs_type) = self.lhs.get_value_type(Some(wip_registry)) else {
            return Err(Box::new(SolverError::InvalidNonValueConstraint));
        };

        let Some(rhs_type) = self.rhs.get_value_type(Some(wip_registry)) else {
            return Err(Box::new(SolverError::InvalidNonValueConstraint));
        };

        match (lhs_type, rhs_type) {
            (SpecOptionType::Unknown, SpecOptionType::Unknown) => Ok(()),
            (SpecOptionType::Unknown, known) => {
                self.lhs.set_value_type(wip_registry, known);
                Ok(())
            }
            (known, SpecOptionType::Unknown) => {
                self.rhs.set_value_type(wip_registry, known);
                Ok(())
            }
            (lhs_known, rhs_known) => {
                if lhs_known == rhs_known {
                    Ok(())
                } else {
                    tracing::error!(
                        "Cannot compare differing types {lhs_type:?} and {rhs_type:?}"
                    );

                    Err(Box::new(SolverError::IncorrectValueType {
                        expected: lhs_type,
                        received: rhs_type,
                    }))
                }
            }
        }?;

        // Continue type checking
        self.lhs.type_check(wip_registry)?;
        self.rhs.type_check(wip_registry)?;

        // Both types are now the same, so we can get away with checking just
        // one of lhs and rhs

        let Some(lhs_type) = self.lhs.get_value_type(Some(wip_registry)) else {
            return Ok(());
        };

        if Cmp::can_cmp(lhs_type, self.op) {
            Ok(())
        } else {
            let msg = format!(
                "Cannot compare type {lhs_type:?} with operator '{}'",
                self.op
            );
            tracing::error!("{msg}");
            Err(Box::new(SolverError::InvalidConstraint(msg)))
        }
    }

    fn extract_spec_options(&self) -> Vec<(&str, &str, spec::SpecOption)> {
        let mut res = Vec::new();
        res.extend(self.lhs.extract_spec_options());
        res.extend(self.rhs.extract_spec_options());
        res
    }

    fn extract_dependencies(&self) -> HashSet<String> {
        Default::default()
    }

    fn to_z3_clauses<'a>(
        &self,
        registry: &mut package::BuiltRegistry<'a>,
    ) -> Result<Vec<z3::ast::Dynamic>, Box<SolverError>> {
        Ok(vec![self.lhs.cmp_to_z3(&self.rhs, self.op, registry)?])
    }

    fn to_python_any<'py>(
        &self,
        py: pyo3::Python<'py>,
    ) -> pyo3::PyResult<pyo3::Bound<'py, pyo3::PyAny>> {
        self.clone().into_bound_py_any(py)
    }
}

impl From<Cmp> for Constraint {
    fn from(val: Cmp) -> Self {
        Constraint::Cmp(Box::new(val))
    }
}

impl std::fmt::Display for Cmp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ {} ] {} [ {} ]", self.lhs, self.op, self.rhs)
    }
}

#[pymethods]
impl Cmp {
    #[new]
    fn py_new(lhs: Constraint, rhs: Constraint, op: CmpType) -> Self {
        Self { lhs, rhs, op }
    }

    /// Wrap this condition in an IfThen constraint.
    ///
    /// cond.if_then(then) => If ( cond ) Then ( then )
    fn if_then(&self, then: Constraint) -> IfThen {
        IfThen { cond: self.clone().into(), then }
    }

    fn __richcmp__(
        &self,
        rhs: Constraint,
        op: CompareOp,
    ) -> Result<Constraint, PyErr> {
        Cmp::py_richcmp_helper(self.clone().into(), rhs.clone(), op.into())
    }
}

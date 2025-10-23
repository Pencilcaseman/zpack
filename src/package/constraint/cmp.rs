use std::{any::Any, collections::HashSet};

use pyo3::{IntoPyObjectExt, prelude::*};
use z3::{
    SortKind,
    ast::{Bool, Dynamic},
};

use crate::{
    package::{
        self,
        constraint::{Constraint, ConstraintType, spec_option},
        outline::SolverError,
    },
    spec,
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
    pub lhs: Box<dyn Constraint>,

    #[pyo3(get, set)]
    pub rhs: Box<dyn Constraint>,

    #[pyo3(get, set)]
    pub op: CmpType,
}

impl Constraint for Cmp {
    fn get_type<'a>(
        &'a self,
        _wip_registry: &mut package::WipRegistry<'a>,
    ) -> Option<ConstraintType> {
        Some(ConstraintType::Cmp)
    }

    fn set_type<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
        constraint_type: ConstraintType,
    ) {
        self.lhs.set_type(wip_registry, constraint_type);
        self.rhs.set_type(wip_registry, constraint_type);
    }

    #[tracing::instrument(skip(self, wip_registry))]
    fn type_check<'a>(
        &'a self,
        wip_registry: &mut package::WipRegistry<'a>,
    ) -> Result<(), Box<SolverError>> {
        // Types must be the same
        // Propagate types from known to unknown

        let lhs_type = self.lhs.get_type(wip_registry);
        let rhs_type = self.rhs.get_type(wip_registry);

        match (lhs_type, rhs_type) {
            (None, None) => Ok(()),

            (None, Some(rhs)) => {
                self.lhs.set_type(wip_registry, rhs);
                Ok(())
            }

            (Some(lhs), None) => {
                self.rhs.set_type(wip_registry, lhs);
                Ok(())
            }

            (Some(lhs), Some(rhs)) => {
                if lhs != rhs {
                    tracing::error!(
                        "cannot compare differing types {lhs:?} and {rhs:?}"
                    );

                    Err(SolverError::IncorrectConstraintType {
                        expected: lhs,
                        received: rhs,
                    })
                } else {
                    Ok(())
                }
            }
        }?;

        // Continue type checking
        self.lhs.type_check(wip_registry)?;
        self.rhs.type_check(wip_registry)?;

        // Both types are now the same, so we can get away with checking just
        // one of lhs and rhs

        // For each operation type, ensure the operation is valid
        let Some(lhs_type) = self.lhs.get_type(wip_registry) else {
            return Ok(());
        };

        let can_cmp = match self.op {
            CmpType::Less
            | CmpType::LessOrEqual
            | CmpType::GreaterOrEqual
            | CmpType::Greater => can_compare_non_eq(lhs_type),

            CmpType::NotEqual | CmpType::Equal => can_compare_eq(lhs_type),
        };

        if can_cmp {
            Ok(())
        } else {
            let msg = format!(
                "cannot compare type {lhs_type:?} with operator '{}'",
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

    fn to_z3_clause<'a>(
        &self,
        registry: &package::BuiltRegistry<'a>,
    ) -> Result<z3::ast::Dynamic, Box<SolverError>> {
        let l = self.lhs.to_z3_clause(registry)?;
        let r = self.rhs.to_z3_clause(registry)?;

        // We know lhs and rhs type check and are comparable, so simply match
        // against the Z3 type of the clause

        Ok(match self.op {
            CmpType::Less => compare_less(&l, &r),
            CmpType::LessOrEqual => compare_less_equal(&l, &r),
            CmpType::NotEqual => compare_not_equal(&l, &r),
            CmpType::Equal => compare_equal(&l, &r),
            CmpType::GreaterOrEqual => compare_greater_equal(&l, &r),
            CmpType::Greater => compare_greater(&l, &r),
        }
        .into())
    }

    fn to_python_any<'py>(
        &self,
        py: pyo3::Python<'py>,
    ) -> pyo3::PyResult<pyo3::Bound<'py, pyo3::PyAny>> {
        self.clone().into_bound_py_any(py)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Display for Cmp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ {} ] {} [ {} ]", self.lhs, self.op, self.rhs)
    }
}

fn can_compare_non_eq(ct: ConstraintType) -> bool {
    match ct {
        ConstraintType::Depends
        | ConstraintType::Cmp
        | ConstraintType::IfThen
        | ConstraintType::Maximize
        | ConstraintType::Minimize => false,

        ConstraintType::SpecOption => unreachable!(),

        ConstraintType::Value(spec_option_type) => match spec_option_type {
            spec::SpecOptionType::Bool => false,

            spec::SpecOptionType::Int
            | spec::SpecOptionType::Float
            | spec::SpecOptionType::Str
            | spec::SpecOptionType::Version => true,
        },
    }
}

fn can_compare_eq(ct: ConstraintType) -> bool {
    match ct {
        ConstraintType::Depends
        | ConstraintType::Cmp
        | ConstraintType::IfThen => true,

        ConstraintType::Maximize | ConstraintType::Minimize => false,

        ConstraintType::SpecOption => unreachable!(),

        ConstraintType::Value(spec_option_type) => match spec_option_type {
            spec::SpecOptionType::Bool
            | spec::SpecOptionType::Int
            | spec::SpecOptionType::Float
            | spec::SpecOptionType::Str
            | spec::SpecOptionType::Version => true,
        },
    }
}

fn compare_less(lhs: &Dynamic, rhs: &Dynamic) -> Bool {
    match lhs.sort_kind() {
        SortKind::Uninterpreted
        | SortKind::RoundingMode
        | SortKind::RE
        | SortKind::Unknown
        | SortKind::BV
        | SortKind::Datatype
        | SortKind::Relation
        | SortKind::FiniteDomain
        | SortKind::Real
        | SortKind::Array => unimplemented!(),

        SortKind::Bool => unreachable!(),

        SortKind::Int => lhs.as_int().unwrap().lt(rhs.as_int().unwrap()),
        SortKind::FloatingPoint => {
            lhs.as_float().unwrap().lt(rhs.as_float().unwrap())
        }
        SortKind::Seq => {
            lhs.as_string().unwrap().str_lt(rhs.as_string().unwrap())
        }
    }
}

fn compare_less_equal(lhs: &Dynamic, rhs: &Dynamic) -> Bool {
    match lhs.sort_kind() {
        SortKind::Uninterpreted
        | SortKind::RoundingMode
        | SortKind::RE
        | SortKind::Unknown
        | SortKind::BV
        | SortKind::Datatype
        | SortKind::Relation
        | SortKind::FiniteDomain
        | SortKind::Real
        | SortKind::Array => unimplemented!(),

        SortKind::Bool => unreachable!(),

        SortKind::Int => lhs.as_int().unwrap().le(rhs.as_int().unwrap()),
        SortKind::FloatingPoint => {
            lhs.as_float().unwrap().le(rhs.as_float().unwrap())
        }
        SortKind::Seq => {
            lhs.as_string().unwrap().str_le(rhs.as_string().unwrap())
        }
    }
}

fn compare_not_equal(lhs: &Dynamic, rhs: &Dynamic) -> Bool {
    lhs.ne(rhs)
}

fn compare_equal(lhs: &Dynamic, rhs: &Dynamic) -> Bool {
    lhs.eq(rhs)
}

fn compare_greater(lhs: &Dynamic, rhs: &Dynamic) -> Bool {
    match lhs.sort_kind() {
        SortKind::Uninterpreted
        | SortKind::RoundingMode
        | SortKind::RE
        | SortKind::Unknown
        | SortKind::BV
        | SortKind::Datatype
        | SortKind::Relation
        | SortKind::FiniteDomain
        | SortKind::Real
        | SortKind::Array => unimplemented!(),

        SortKind::Bool => unreachable!(),

        SortKind::Int => lhs.as_int().unwrap().gt(rhs.as_int().unwrap()),
        SortKind::FloatingPoint => {
            lhs.as_float().unwrap().gt(rhs.as_float().unwrap())
        }
        SortKind::Seq => {
            lhs.as_string().unwrap().str_gt(rhs.as_string().unwrap())
        }
    }
}

fn compare_greater_equal(lhs: &Dynamic, rhs: &Dynamic) -> Bool {
    match lhs.sort_kind() {
        SortKind::Uninterpreted
        | SortKind::RoundingMode
        | SortKind::RE
        | SortKind::Unknown
        | SortKind::BV
        | SortKind::Datatype
        | SortKind::Relation
        | SortKind::FiniteDomain
        | SortKind::Real
        | SortKind::Array => unimplemented!(),

        SortKind::Bool => unreachable!(),

        SortKind::Int => lhs.as_int().unwrap().ge(rhs.as_int().unwrap()),
        SortKind::FloatingPoint => {
            lhs.as_float().unwrap().ge(rhs.as_float().unwrap())
        }
        SortKind::Seq => {
            lhs.as_string().unwrap().str_ge(rhs.as_string().unwrap())
        }
    }
}

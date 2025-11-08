//! Generic Version implementation.
//!
//! The [`Version`] struct does not implement any single version specification
//! standard, but aims to encompass as many common ones as possible.
//!
//! A version is stored as a list of [`Part`]s, which include integers, strings,
//! wildcards and separators.
//!
//! To operate within the bounds of the solver, we specify some comparison rules
//! that are logically consistent and have beneficial properties:
//!
//! - We can only compare versions with the same length
//!     - x.y > 1.2.3 ==> x.y.z > 1.2.3 (z is introduced as a new component)
//! - Strings are smaller than numbers => 1.alpha < 1.2
//! - Numbers are sorted by value => 1.2.3 < 1.2.4 < 1.3.2 < 2.3 < 3
//! - Strings are sorted lexicographically with a few exceptions:
//!     - git > dev > devel > main > master > alpha > beta > latest > stable >
//!       everything else (see [`STATIC_STRING_VERSIONS`])
//! - Separators must match
//! - Wildcards => 1.2.3 == 1.*.3 == 1.> == 1.2.> == 1.*.*
//!     - Single matches any string or number
//!     - Rest matches the rest of a version
//!         - Regardless of remaining separators
//!
//! Before adding versions to the solver, we track them in a
//! [`WipVersionRegistry`]. Explicit string version orderings will all be added
//! by default, and any other strings found during the outlining phase will also
//! be added.
//!
//! String version parts are mapped to indices and treated as integers. Integer
//! version parts are also treated as integers, but are offset by the number of
//! string versions in the registry. This may lead to some slightly strange
//! errors in cases where the specification is unsatisfiable, but the UNSAT core
//! should provide enough context to identify the cause of the issue.

use std::{fmt::Write, str::FromStr};

use pyo3::{exceptions::PyValueError, prelude::*};

use crate::{constraint::CmpType, package::registry::BuiltVersionRegistry};

/// Version strings with a specified, non-lexicographic order
pub const STATIC_STRING_VERSIONS: [&str; 9] = [
    "stable", "latest", "beta", "alpha", "master", "main", "devel", "dev",
    "git",
];

/// Valid version separators
pub const VERSION_SEPARATORS: [char; 3] = ['.', '-', '+'];

/// Wildcard specifier in a version.
///
/// - [`WildcardType::Single`] is an asterisk ('*') and represents any value
/// - [`WildcardType::Rest`] is a right chevron ('>') and matches any remaining
///   version components. This must be the final part of a version.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WildcardType {
    Single,
    Rest,
}

/// Parts of a version.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Part {
    /// An integer component
    Int(usize),

    /// An alphanumeric component
    Str(String),

    /// A wildcard component
    Wildcard(WildcardType),

    /// A separator component
    Sep(char),
}

/// A generic version.
///
/// See the documentation for this module for more information.
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    parts: Vec<Part>,
}

#[derive(Debug, Clone)]
pub enum ParseError {
    TrailingSeparator,
    InvalidCharacter(char),
    InvalidPart(String),
    EmptyPart,
    PartAfterRest,
}

impl Part {
    /// Convert a [`Part`] into a [`z3::ast::Dynamic`], if possible.
    ///
    /// - [`Part::Int`] => [`z3::ast::Int`]
    /// - [`Part::Int`] => [`z3::ast::Int`]
    /// - [`Part::Int`] => [`z3::ast::Int`]
    ///
    /// # Panics
    /// Panics if this is a [`Part::Str`] but the string does not appear in the
    /// provided [`BuiltVersionRegistry`]
    #[must_use]
    pub fn to_z3_dynamic(
        &self,
        version_registry: &BuiltVersionRegistry,
    ) -> Option<z3::ast::Dynamic> {
        use z3::ast::{Int, String};

        match self {
            Self::Int(v) => Some(
                Int::from_u64((*v + version_registry.offset()) as u64).into(),
            ),

            Self::Str(v) => {
                let idx = version_registry
                    .lookup_str(v)
                    .expect("Internal solver error");
                Some(Int::from_u64(*idx as u64).into())
            }

            Self::Sep(v) => {
                Some(String::from_str(&v.to_string()).unwrap().into())
            }

            Self::Wildcard(_) => None,
        }
    }
}

impl Version {
    pub fn new(txt: &str) -> Result<Self, ParseError> {
        let mut segments = Vec::new();

        let mut seen_rest = false;

        let mut parse_seg = |seg: &str| -> Result<Part, ParseError> {
            if seen_rest {
                Err(ParseError::PartAfterRest)
            } else if seg == "*" {
                Ok(Part::Wildcard(WildcardType::Single))
            } else if seg == ">" {
                seen_rest = true;
                Ok(Part::Wildcard(WildcardType::Rest))
            } else if let Ok(num) = seg.parse::<usize>() {
                Ok(Part::Int(num))
            } else if seg.chars().all(|c| c.is_ascii_alphanumeric()) {
                if seg.is_empty() {
                    Err(ParseError::EmptyPart)
                } else {
                    Ok(Part::Str(seg.to_string()))
                }
            } else {
                Err(ParseError::InvalidPart(seg.to_string()))
            }
        };

        let mut last = 0;
        for (idx, m) in txt.match_indices(|c| VERSION_SEPARATORS.contains(&c)) {
            segments.push(parse_seg(&txt[last..idx])?);
            segments.push(Part::Sep(m.chars().next().unwrap()));
            last = idx + 1;
        }

        if last == txt.len() {
            return Err(ParseError::TrailingSeparator);
        }

        segments.push(parse_seg(&txt[last..])?);

        Ok(Self { parts: segments })
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self { parts: Vec::new() }
    }

    /// # Safety
    /// `part` must be valid and ensure alternating values and separators
    pub unsafe fn push(&mut self, part: Part) {
        self.parts.push(part);
    }

    #[must_use]
    pub fn parts(&self) -> &[Part] {
        &self.parts
    }

    #[must_use]
    pub fn num_segments(&self) -> usize {
        assert_eq!(
            self.parts.len() & 1,
            1,
            "Invalid version. Expected n separators and n + 1 segments"
        );
        self.parts.len() / 2 + 1
    }

    #[must_use]
    pub fn num_separators(&self) -> usize {
        assert_eq!(
            self.parts.len() & 1,
            1,
            "Invalid version. Expected n separators and n + 1 segments"
        );
        self.parts.len() / 2
    }

    #[must_use]
    pub fn cmp_dynamic(
        &self,
        op: CmpType,
        vars: &[z3::ast::Dynamic],
        registry: &BuiltVersionRegistry,
    ) -> z3::ast::Bool {
        unsafe {
            match op {
                CmpType::LessOrEqual => {
                    self.cmp_less_equal_dynamic(vars, registry)
                }

                CmpType::Greater => {
                    self.cmp_less_equal_dynamic(vars, registry).not()
                }

                CmpType::NotEqual => self.cmp_eq_dynamic(vars, registry).not(),

                CmpType::Equal => self.cmp_eq_dynamic(vars, registry),

                CmpType::GreaterOrEqual => {
                    self.cmp_greater_equal_dynamic(vars, registry)
                }

                CmpType::Less => {
                    self.cmp_greater_equal_dynamic(vars, registry).not()
                }
            }
        }
    }

    /// # Safety
    /// `len(vars) == self.parts().len()`.
    ///
    /// # Panics
    /// Any panics are internal solver errors.
    #[must_use]
    pub unsafe fn cmp_less_equal_dynamic(
        &self,
        vars: &[z3::ast::Dynamic],
        registry: &BuiltVersionRegistry,
    ) -> z3::ast::Bool {
        let mut bools = Vec::new();

        for (var, val) in vars.iter().zip(self.parts()) {
            let cond = match val {
                Part::Int(i) => var.as_int().unwrap().le(registry
                    .part_to_dynamic(Part::Int(*i))
                    .as_int()
                    .unwrap()),

                Part::Str(s) => var.as_int().unwrap().le(registry
                    .part_to_dynamic(Part::Str(s.clone()))
                    .as_int()
                    .unwrap()),

                Part::Sep(c) => var.as_string().unwrap().eq(registry
                    .part_to_dynamic(Part::Sep(*c))
                    .as_string()
                    .unwrap()),

                Part::Wildcard(w) => match w {
                    WildcardType::Single => continue,
                    WildcardType::Rest => break,
                },
            };

            bools.push(cond);
        }

        z3::ast::Bool::and(&bools)
    }

    /// # Safety
    /// `len(vars) == self.parts().len()`
    #[must_use]
    pub unsafe fn cmp_eq_dynamic(
        &self,
        vars: &[z3::ast::Dynamic],
        registry: &BuiltVersionRegistry,
    ) -> z3::ast::Bool {
        let mut bools = Vec::new();

        println!("vars: {vars:?}");
        println!("parts: {:?}", self.parts());

        for (var, val) in vars.iter().zip(self.parts()) {
            println!("{var:?} {val:?}");

            let cond = match val {
                Part::Int(i) => var.as_int().unwrap().eq(registry
                    .part_to_dynamic(Part::Int(*i))
                    .as_int()
                    .unwrap()),

                Part::Str(s) => var.as_int().unwrap().eq(registry
                    .part_to_dynamic(Part::Str(s.clone()))
                    .as_int()
                    .unwrap()),

                Part::Sep(c) => var.eq(registry.part_to_dynamic(Part::Sep(*c))),

                Part::Wildcard(w) => match w {
                    WildcardType::Single => continue,
                    WildcardType::Rest => break,
                },
            };

            bools.push(cond);
        }

        z3::ast::Bool::and(&bools)
    }

    /// # Safety
    /// `len(vars) == self.parts().len()`
    #[must_use]
    pub unsafe fn cmp_greater_equal_dynamic(
        &self,
        vars: &[z3::ast::Dynamic],
        registry: &BuiltVersionRegistry,
    ) -> z3::ast::Bool {
        let mut bools = Vec::new();

        for (var, val) in vars.iter().zip(self.parts()) {
            let cond = match val {
                Part::Int(i) => var.as_int().unwrap().ge(registry
                    .part_to_dynamic(Part::Int(*i))
                    .as_int()
                    .unwrap()),

                Part::Str(s) => var.as_int().unwrap().ge(registry
                    .part_to_dynamic(Part::Str(s.clone()))
                    .as_int()
                    .unwrap()),

                Part::Sep(c) => var.as_string().unwrap().eq(registry
                    .part_to_dynamic(Part::Sep(*c))
                    .as_string()
                    .unwrap()),

                Part::Wildcard(w) => match w {
                    WildcardType::Single => continue,
                    WildcardType::Rest => break,
                },
            };

            bools.push(cond);
        }

        z3::ast::Bool::and(&bools)
    }
}

impl std::fmt::Display for WildcardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single => f.write_char('*'),
            Self::Rest => f.write_char('>'),
        }
    }
}

impl std::fmt::Display for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(i) => write!(f, "{i}"),
            Self::Str(s) => f.write_str(s),
            Self::Sep(c) => f.write_char(*c),
            Self::Wildcard(w) => w.fmt(f),
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for part in &self.parts {
            f.write_str(&part.to_string())?;
        }

        Ok(())
    }
}

#[pymethods]
impl Version {
    #[new]
    fn py_new(ver: &str) -> Result<Self, PyErr> {
        Self::new(ver).map_err(|e| PyValueError::new_err(format!("{e:?}")))
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }

    fn __str__(&self) -> String {
        format!("{self}")
    }
}

use std::fmt::Write;

use pyo3::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WildcardType {
    Single,
    Rest,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Part {
    Int(u64),
    Str(String),
    Sep(char),
    Wildcard(WildcardType),
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    segments: Vec<Part>,
}

#[derive(Debug, Clone)]
pub enum ParseError {
    TrailingSeparator,
    InvalidCharacter(char),
    InvalidSegment(String),
    EmptySegment,
    SegmentAfterRest,
}

impl Version {
    pub fn new(txt: &str) -> Result<Self, ParseError> {
        let mut segments = Vec::new();

        let mut seen_rest = false;

        let mut parse_seg = |seg: &str| -> Result<Part, ParseError> {
            if seen_rest {
                Err(ParseError::SegmentAfterRest)
            } else if seg == "*" {
                Ok(Part::Wildcard(WildcardType::Single))
            } else if seg == ">" {
                seen_rest = true;
                Ok(Part::Wildcard(WildcardType::Rest))
            } else if let Ok(num) = seg.parse::<u64>() {
                Ok(Part::Int(num))
            } else if seg.chars().all(|c| c.is_ascii_alphanumeric()) {
                if !seg.is_empty() {
                    Ok(Part::Str(seg.to_string()))
                } else {
                    Err(ParseError::EmptySegment)
                }
            } else {
                Err(ParseError::InvalidSegment(seg.to_string()))
            }
        };

        let seps = ['.', '-', '+'];

        let mut last = 0;
        for (idx, m) in txt.match_indices(|c| seps.contains(&c)) {
            segments.push(parse_seg(&txt[last..idx])?);
            segments.push(Part::Sep(m.chars().next().unwrap()));
            last = idx + 1;
        }

        if last == txt.len() {
            return Err(ParseError::TrailingSeparator);
        }

        segments.push(parse_seg(&txt[last..])?);

        Ok(Self { segments })
    }

    pub fn segments(&self) -> &[Part] {
        &self.segments
    }
}

impl std::cmp::Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Version comparison logic:
        //
        // - Short versions are bigger => 1 > 1.2 > 1.2.3 > 1.2.3.4
        // - Strings are smaller than numbers => 1.alpha < 1.2
        // - Numbers are sorted by value => 1.2.3 < 1.2.4 < 1.3.2 < 2.3 < 3
        // - Strings are sorted lexicographically with a few exceptions:
        //     - git > dev > devel > main > master > alpha > beta > latest >
        //       stable > everything else
        // - Separators must match
        // - Wildcards => 1.2.3 == 1.*.3 == 1.> == 1.2.> == 1.*.*
        //     - Single matches any string or number
        //     - Rest matches the rest of a version
        //         - Regardless of remaining separators

        todo!()
    }
}

impl std::cmp::PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for WildcardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WildcardType::Single => f.write_char('*'),
            WildcardType::Rest => f.write_char('>'),
        }
    }
}

impl std::fmt::Display for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Part::Int(i) => write!(f, "{i}"),
            Part::Str(s) => f.write_str(s),
            Part::Sep(c) => f.write_char(*c),
            Part::Wildcard(w) => w.fmt(f),
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for part in &self.segments {
            f.write_str(&part.to_string())?
        }

        Ok(())
    }
}

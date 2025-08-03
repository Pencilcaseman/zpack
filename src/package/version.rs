use color_eyre::{Result, eyre};

/// A type representing a concrete version.
///
/// Multiple version types are supported, including:
///  - Semantic Versioning: [`Version::SemVer`]
///  - Other: [`Version::Other`]
pub enum Version {
    SemVer { major: u64, minor: u64, patch: u64, rc: Option<String> },
    Other(String),
}

/// Parse a version string and return a concrete Version option, if possible
fn parse_version(version: &str) -> Result<Version> {
    todo!()
}

impl Version {
    pub fn new(ver: &str) -> Result<Self> {
        parse_version(ver)
    }
}

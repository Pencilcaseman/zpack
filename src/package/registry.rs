use std::collections::HashMap;

use crate::{package::version::Version, spec};

#[derive(Debug, Default, Clone)]
pub struct WipRegistry<'a> {
    pub current_package_name: Option<&'a str>,
    pub current_option_name: Option<&'a str>,

    pub option_type_map:
        HashMap<(&'a str, Option<&'a str>), spec::SpecOptionType>,
    pub option_ast_map: HashMap<(&'a str, Option<&'a str>), z3::ast::Dynamic>,
    pub versions: WipVersionRegistry,
}

#[derive(Debug, Default, Clone)]
pub struct Registry<'a> {
    pub option_type_map:
        HashMap<(&'a str, Option<&'a str>), spec::SpecOptionType>,
    pub option_ast_map: HashMap<(&'a str, Option<&'a str>), z3::ast::Dynamic>,
    pub versions: VersionRegistry,
}

impl<'a> WipRegistry<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self) -> Registry<'a> {
        Registry {
            option_type_map: self.option_type_map,
            option_ast_map: self.option_ast_map,
            versions: self.versions.build(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct WipVersionRegistry {
    versions: Vec<Version>,
}

#[derive(Debug, Clone, Default)]
pub struct VersionRegistry {
    versions: HashMap<Version, usize>,
}

impl WipVersionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new version to the registry
    pub fn push(&mut self, ver: Version) {
        tracing::info!("pushing version {ver}");
        self.versions.push(ver);
    }

    pub fn build(mut self) -> VersionRegistry {
        self.versions.sort();

        let mut versions = HashMap::with_capacity(self.versions.len());
        self.versions.iter().enumerate().for_each(|(idx, v)| {
            versions.insert(v.clone(), idx);
        });

        VersionRegistry { versions }
    }
}

impl VersionRegistry {
    pub fn lookup(&self, ver: &Version) -> Option<usize> {
        self.versions.get(ver).copied()
    }
}

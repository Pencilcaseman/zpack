use std::collections::HashMap;

use color_eyre::{
    Result,
    eyre::{OptionExt, eyre},
};

/// The valid data types a configuration option can have in a package
/// description.
///
/// Each data type supports a syntax in YAML for assigning a value of that type.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SpecOption {
    Bool(bool),
    Int(i64),
    String(String),
    List(Vec<SpecOption>),
}

pub struct PackageSpec {
    pub downloader: (),
    pub compiler: (),
    pub builder: (),
    pub options: HashMap<String, SpecOption>,
}

/// Parses a boolean option.
///
/// Valid syntaxes are:
/// - "my_option=true"  => my_option = True
/// - "my_option=false" => my_option = False
/// - "+my_option"      => my_option = True
/// - "-my_option"      => my_option = False
/// - "~my_option"      => my_option = False
fn parse_bool(opt: &str) -> Option<(String, SpecOption)> {
    fn parse_equals(opt: &str) -> Option<(String, SpecOption)> {
        let truth_map: HashMap<&'static str, bool> = [
            ("true", true),
            ("on", true),
            ("yes", true),
            ("false", false),
            ("off", false),
            ("no", false),
        ]
        .into_iter()
        .collect();

        let trim = opt.trim();
        let lower = trim.to_lowercase();
        let eq = trim.rfind("=")?;

        let first = lower.split_at(eq).0.trim();
        let second = lower.split_at(eq + 1).1.trim();

        truth_map.get(second).map(|val| (first.into(), SpecOption::Bool(*val)))
    }

    let mut iter = opt.bytes().skip_while(|&b| b.is_ascii_whitespace());

    let value = match iter.next() {
        Some(b'+') => true,
        Some(b'-') => false,
        Some(b'~') => false,
        _ => return parse_equals(opt),
    };

    let name: String = iter.map(|b| b as char).collect();

    Some((name, SpecOption::Bool(value)))
}

pub fn parse_spec_option(opt: &str) -> Result<(String, SpecOption)> {
    // +name => Bool(true)
    // -name => Bool(true)
    // ~name => Bool(true)
    //
    // name=+1234 => Int(1234)
    // name=-1234 => Int(1234)
    // name=1234  => Int(1234)
    //
    // name=3.14  => Float(12.34)
    // name=+3.14 => Float(3.14)
    // name=-3.14 => Float(-3.14)
    //
    // name=hello => String("hello")
    // name='hello' => String("hello")
    // name="hello" => String("hello")
    //
    // name=[1, 2, 3, 4] => List([Int(1), Int(2), Int(3), Int(4)])

    println!("Parse result: {:?}", parse_bool(opt));

    parse_bool(opt).or(None).ok_or_eyre(eyre!("Invalid option: '{opt}'"))
}

pub struct Package {
    pub name: String,
    pub version: (),
    pub spec: PackageSpec,
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    pub fn test_parse_bool() {
        let args: Vec<(&'static str, Option<(String, SpecOption)>)> =
            Vec::from([
                ("+static", Some(("static".into(), SpecOption::Bool(true)))),
                ("-static", Some(("static".into(), SpecOption::Bool(false)))),
                ("~static", Some(("static".into(), SpecOption::Bool(false)))),
                (
                    "+my option",
                    Some(("my option".into(), SpecOption::Bool(true))),
                ),
                (
                    "static=true",
                    Some(("static".into(), SpecOption::Bool(true))),
                ),
                (
                    "static= true",
                    Some(("static".into(), SpecOption::Bool(true))),
                ),
                (
                    "static =true",
                    Some(("static".into(), SpecOption::Bool(true))),
                ),
                (
                    "static = true",
                    Some(("static".into(), SpecOption::Bool(true))),
                ),
                (
                    "static=false",
                    Some(("static".into(), SpecOption::Bool(false))),
                ),
                (
                    "static= false",
                    Some(("static".into(), SpecOption::Bool(false))),
                ),
                (
                    "static =false",
                    Some(("static".into(), SpecOption::Bool(false))),
                ),
                (
                    "static = false",
                    Some(("static".into(), SpecOption::Bool(false))),
                ),
            ]);

        for (input, output) in &args {
            assert_eq!(parse_bool(input), *output);
        }
    }
}

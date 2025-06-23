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
/// - my_option=true  => my_option = True
/// - my_option=false => my_option = False
/// - +my_option      => my_option = True
/// - '-my_option'    => my_option = False
/// - ~my_option      => my_option = False
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

/// Parses an integer option.
///
/// Valid syntaxes are:
/// - my_option=123  => my_option = 123
/// - my_option=+123 => my_option = 123
/// - my_option=-123 => my_option = -123
fn parse_int(opt: &str) -> Option<(String, SpecOption)> {
    let eq = opt.rfind("=")?;
    let first = opt.split_at(eq).0.trim();
    let last = opt.split_at(eq + 1).1.trim();

    let val = last.parse::<i64>().ok()?;

    Some((first.into(), SpecOption::Int(val)))
}

/// Parse a string option.
///
/// Valid syntaxes are:
/// - "my_option='Hello, World!'"
/// - "my_option="Hello, World!""
/// - "my_option="This \"contains\" quotes!""
fn parse_str(opt: &str) -> Option<(String, SpecOption)> {
    let eq = opt.rfind("=")?;
    let first = opt.split_at(eq).0.trim();
    let last = opt.split_at(eq + 1).1.trim();

    let mut iter = last.bytes();

    let quote_type = match iter.next()? {
        c if c == b'"' || c == b'\'' => c,
        _ => return None,
    };

    let mut prev = b'\0';
    let mut res = String::new();

    for b in iter.by_ref() {
        if prev == b'\\' {
            res += &String::from(b as char);
        } else {
            if b == quote_type {
                // End of string
                prev = b;
                break;
            }

            if b != b'\\' {
                res += &String::from(b as char);
            }
        }

        prev = b;
    }

    if iter.count() == 0 && prev == quote_type {
        Some((first.into(), SpecOption::String(res)))
    } else {
        None
    }
}

/// Parse a list of values.
///
/// Valid syntaxes include:
/// - my_list = [1, 2, 3]
/// - my_list = ["hello", 123, ["a", 'nested', "list"], true]
fn parse_list(opt: &str) -> Option<(String, SpecOption)> {
    let eq = opt.rfind("=")?;
    let first = opt.split_at(eq).0.trim();
    let last = opt.split_at(eq + 1).1.trim();

    let mut iter = last.bytes();

    // Opening '['
    match iter.next() {
        Some(b'[') => (),
        _ => return None,
    }

    let mut current = String::new();

    // while let Some(next) = iter.next() {
    //     match next {
    //         // 'b'
    //     }
    // }

    todo!()
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

    [parse_bool, parse_int, parse_str]
        .iter()
        .find_map(|f| f(opt))
        .ok_or_eyre(eyre!("Invalid option: '{opt}'"))
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

            let Ok(parsed) = parse_spec_option(input) else {
                panic!("Failed to parse")
            };

            assert_eq!(Some(parsed), *output);
        }
    }

    #[test]
    pub fn test_parse_int() {
        let args: Vec<(&'static str, Option<(String, SpecOption)>)> =
            Vec::from([
                ("num=123", Some(("num".into(), SpecOption::Int(123)))),
                ("num=+123", Some(("num".into(), SpecOption::Int(123)))),
                ("num=-123", Some(("num".into(), SpecOption::Int(-123)))),
                ("num= +123", Some(("num".into(), SpecOption::Int(123)))),
                ("num= -123", Some(("num".into(), SpecOption::Int(-123)))),
                ("num =+123", Some(("num".into(), SpecOption::Int(123)))),
                ("num =-123", Some(("num".into(), SpecOption::Int(-123)))),
                ("num = +123", Some(("num".into(), SpecOption::Int(123)))),
                ("num = -123", Some(("num".into(), SpecOption::Int(-123)))),
            ]);

        for (input, output) in &args {
            assert_eq!(parse_int(input), *output, "Input: {input}");

            let Ok(parsed) = parse_spec_option(input) else {
                panic!("Failed to parse")
            };

            assert_eq!(Some(parsed), *output);
        }
    }

    #[test]
    pub fn test_parse_str() {
        let args: Vec<(&'static str, Option<(String, SpecOption)>)> =
            Vec::from([
                (
                    "str='Hello, World!'",
                    Some((
                        "str".into(),
                        SpecOption::String("Hello, World!".into()),
                    )),
                ),
                (
                    "str=\"Hello, World!\"",
                    Some((
                        "str".into(),
                        SpecOption::String("Hello, World!".into()),
                    )),
                ),
                (
                    "str=\"'''\"",
                    Some(("str".into(), SpecOption::String("'''".into()))),
                ),
                (
                    "str=\"This \\\"is\\\" quoted\"",
                    Some((
                        "str".into(),
                        SpecOption::String("This \"is\" quoted".into()),
                    )),
                ),
            ]);

        for (input, output) in &args {
            assert_eq!(parse_str(input), *output, "Input: {input}");

            let Ok(parsed) = parse_spec_option(input) else {
                panic!("Failed to parse")
            };

            assert_eq!(Some(parsed), *output);
        }
    }
}

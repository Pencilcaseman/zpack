use std::{
    collections::HashMap,
    iter::{Iterator, Peekable},
};

use color_eyre::{
    Result, Section,
    eyre::{OptionExt, eyre},
};

use crate::util::num;

/// The valid data types a configuration option can have in a package
/// description.
///
/// Each data type supports a syntax in YAML for assigning a value of that type.
#[derive(Debug, Clone, PartialEq)]
pub enum SpecOption {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<SpecOption>),
}

pub struct PackageSpec {
    pub downloader: (),
    pub compiler: (),
    pub builder: (),
    pub options: HashMap<String, SpecOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionToken {
    Space, // _
    Plus,  // +
    Minus, // -
    Tilde, // ~
    Equal, // =
    // SingleQuote,   // ' // Replaced by String
    // DoubleQuote,   // "
    // OpenBracket,   // ( // Unnecessary
    // CloseBracket,  // )
    OpenSquare,    // [
    CloseSquare,   // ]
    Comma,         // ,
    Bool(bool),    // true/false
    Int(i64),      // Integer value
    Float(f64),    // Floating point value
    Str(String),   // String literal
    Named(String), // Named literal
}

pub fn tokenize_option(opt: &str) -> Result<Vec<OptionToken>> {
    let bytes = opt.as_bytes();

    let mut res = Vec::new();
    let mut idx = 0;

    while idx < opt.len() {
        use OptionToken::*;

        let value = match bytes[idx] {
            b' ' => Space,
            b'+' => Plus,
            b'-' => Minus,
            b'~' => Tilde,
            b'=' => Equal,
            b'[' => OpenSquare,
            b']' => CloseSquare,
            b',' => Comma,

            b'\'' | b'"' => {
                let start_idx = idx;
                let quote_type = bytes[idx];
                let mut escaped = Vec::new();

                while idx < bytes.len() {
                    idx += 1;

                    match bytes[idx] {
                        b'\\' => {
                            escaped.push((idx - 1, None));
                            escaped.push((idx, None));
                            idx += 1;

                            if idx >= bytes.len() {
                                return Err(eyre!("Unexpected end of string"));
                            }

                            let popped = escaped.pop().unwrap().0;

                            match bytes[idx] {
                                b'\\' => escaped.push((popped, Some('\\'))),
                                b'\'' => escaped.push((popped, Some('\''))),
                                b'\"' => escaped.push((popped, Some('\"'))),
                                b't' => escaped.push((popped, Some('\t'))),
                                b'n' => escaped.push((popped, Some('\n'))),

                                unknown => {
                                    return Err(eyre!(
                                        "Invalid escape sequence: '\\{}'",
                                        unknown as char
                                    ));
                                }
                            }
                        }

                        matching if matching == quote_type => break,

                        _ => (),
                    }
                }

                Str(bytes[start_idx + 1..idx]
                    .iter()
                    .enumerate()
                    .filter_map(|(i, b)| {
                        match escaped.iter().find(|(j, _)| i == *j) {
                            Some((_, c)) => *c,
                            None => Some(*b as char),
                        }
                    })
                    .collect())
            }

            _ if bytes[idx..(idx + 4).min(opt.len())]
                .iter()
                .map(|b| b.to_ascii_lowercase() as char)
                .collect::<String>()
                == "true" =>
            {
                idx += 3;
                Bool(true)
            }

            _ if bytes[idx..(idx + 5).min(opt.len())]
                .iter()
                .map(|b| b.to_ascii_lowercase() as char)
                .collect::<String>()
                == "false" =>
            {
                idx += 4;
                Bool(false)
            }

            _ if bytes[idx].is_ascii_digit() => {
                let literal = bytes
                    .iter()
                    .skip(idx)
                    .take_while(|&&b| {
                        b.is_ascii_digit()
                            || b == b'.' // 3.14
                            || b == b'_' // 123_456
                            || b == b'e' // 1e5
                            || b == b'+' // 1e+5 or +123
                            || b == b'-' // 1e-5 or -123
                    })
                    .map(|&b| b as char)
                    .collect::<String>();

                if literal.is_empty() {
                    return Err(eyre!("Invalid spec option: {opt:?}")
                        .with_section(move || {
                            format!(
                                "Unexpected token at index {}: {:?}",
                                idx, bytes[idx] as char
                            )
                        }));
                }

                let result = match num::parse_num(&literal)? {
                    num::Number::Integer(int) => Int(int),
                    num::Number::Float(float) => Float(float),
                };

                idx += literal.len() - 1;

                result
            }

            _ => {
                let literal = bytes
                    .iter()
                    .skip(idx)
                    .take_while(|&&b| {
                        b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
                    })
                    .map(|&b| b as char)
                    .collect::<String>();

                if literal.is_empty() {
                    return Err(eyre!("Invalid spec option: {opt:?}")
                        .with_section(move || {
                            format!(
                                "Unexpected token at index {}: {:?}",
                                idx, bytes[idx] as char
                            )
                        }));
                }

                idx += literal.len() - 1;

                Named(literal)
            }
        };

        idx += 1;

        res.push(value);
    }

    Ok(res)
}

#[derive(Debug)]
pub struct ConsumeResult {
    pub name: Option<String>,
    pub value: SpecOption,
}

/// Consume a boolean value.
///
/// Valid syntaxes are:
/// - `+my_option`      => my_option = True
/// - `'-my_option'`    => my_option = False
/// - `~my_option`      => my_option = False
/// - `true`            => True
/// - `false`           => False
fn consume_bool(
    tokens: &[OptionToken],
) -> (Result<ConsumeResult>, &[OptionToken]) {
    use OptionToken::*;

    if tokens.is_empty() {
        return (
            Err(eyre!("Expected Bool. Received empty token stream.")),
            tokens,
        );
    }

    if matches!(tokens[0], Plus | Minus | Tilde) {
        if let Named(name) = &tokens[1] {
            (
                Ok(ConsumeResult {
                    name: Some(name.to_string()),
                    value: SpecOption::Bool(match tokens[0] {
                        Plus => true,
                        Minus | Tilde => false,
                        _ => unreachable!(),
                    }),
                }),
                &tokens[2..],
            )
        } else {
            (
                Err(eyre!(
                    "Invalid syntax. Expected `+option`, `-option` or `~option`"
                )),
                tokens,
            )
        }
    } else if let Bool(value) = tokens[0] {
        (
            Ok(ConsumeResult { name: None, value: SpecOption::Bool(value) }),
            &tokens[1..],
        )
    } else if let Named(name) = &tokens[0]
        && matches!(tokens[1], Equal)
        && let Bool(value) = tokens[2]
    {
        (
            Ok(ConsumeResult {
                name: Some(name.to_string()),
                value: SpecOption::Bool(value),
            }),
            &tokens[3..],
        )
    } else {
        (
            Err(eyre!(
                "Invalid syntax. Expected `+option`, `-option` or `~option`"
            )),
            tokens,
        )
    }
}

fn consume_num(
    tokens: &[OptionToken],
) -> (Result<ConsumeResult>, &[OptionToken]) {
    use OptionToken::*;

    if tokens.is_empty() {
        return (
            Err(eyre!("Expected Number. Received empty token stream")),
            tokens,
        );
    }

    if let Int(num) = tokens[0] {
        (
            Ok(ConsumeResult { name: None, value: SpecOption::Int(num) }),
            &tokens[1..],
        )
    } else if matches!(tokens[0], Plus | Minus) {
        let num = consume_num(&tokens[1..]);

        if let Ok(mut thing) = num.0 {
            if let SpecOption::Int(num) = thing.value {
                thing.value = SpecOption::Int(match tokens[0] {
                    Plus => num,
                    Minus => -num,
                    _ => unreachable!(),
                });
            } else if let SpecOption::Float(num) = thing.value {
                thing.value = SpecOption::Float(match tokens[0] {
                    Plus => num,
                    Minus => -num,
                    _ => unreachable!(),
                });
            }

            (Ok(ConsumeResult { name: None, value: thing.value }), num.1)
        } else {
            (Err(eyre!("Unknown syntax error.")), tokens)
        }
    } else {
        (Err(eyre!("Expected Number.")), tokens)
    }
}

fn consume_str(
    tokens: &[OptionToken],
) -> (Result<ConsumeResult>, &[OptionToken]) {
    use OptionToken::*;

    if tokens.is_empty() {
        return (
            Err(eyre!("Expected String. Received empty token stream")),
            tokens,
        );
    }

    if let Str(txt) = &tokens[0] {
        (
            Ok(ConsumeResult {
                name: None,
                value: SpecOption::String(txt.clone()),
            }),
            &tokens[1..],
        )
    } else {
        (Err(eyre!("Unknown syntax error.")), tokens)
    }
}

pub fn consume_spec_option(
    tokens: &[OptionToken],
) -> (Result<ConsumeResult>, &[OptionToken]) {
    {
        let bool_result = consume_bool(tokens);
        if let Ok(result) = bool_result.0 {
            return (Ok(result), bool_result.1);
        }
    }

    {
        let num_result = consume_num(tokens);
        if num_result.0.is_ok() {
            return num_result;
        }
    }

    {
        let str_result = consume_str(tokens);
        if str_result.0.is_ok() {
            return str_result;
        }
    }

    todo!()
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

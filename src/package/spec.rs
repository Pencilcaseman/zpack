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
    Space,         // _
    Plus,          // +
    Minus,         // -
    Tilde,         // ~
    Equal,         // =
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

        if value != Space {
            res.push(value);
        }
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

fn consume_list(
    tokens: &[OptionToken],
) -> (Result<ConsumeResult>, &[OptionToken]) {
    use OptionToken::*;

    if tokens.is_empty() {
        return (
            Err(eyre!("Expected String. Received empty token stream")),
            tokens,
        );
    }

    if tokens[0] != OpenSquare {
        println!("Returning here because tokens = {:?}", tokens);
        return (Err(eyre!("Expected open square bracket ('[')")), tokens);
    }

    let mut idx = 1;
    let mut values = Vec::new();

    while tokens[idx] != CloseSquare {
        if tokens[idx] == Comma && !values.is_empty() {
            idx += 1;
        }

        if idx >= tokens.len() {
            return (
                Err(eyre!(
                    "Unexpected end of string. Expected closing square bracket (']')"
                )),
                tokens,
            );
        }

        {
            let (res, rem) = consume_spec_option(&tokens[idx..]);

            if let Ok(val) = res {
                values.push(val.value);
            } else {
                println!("Failed: {res:?}");
                return (res, tokens);
            }
        }

        idx += 1;
    }

    idx += 1;

    (
        Ok(ConsumeResult { name: None, value: SpecOption::List(values) }),
        &tokens[idx..],
    )
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

    {
        let list_result = consume_list(tokens);
        if list_result.0.is_ok() {
            return list_result;
        }
    }

    todo!()
}

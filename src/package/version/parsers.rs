use chumsky::prelude::*;

use crate::util::error::ParserErrorType;

pub fn ident<'a>()
-> impl Parser<'a, &'a str, char, extra::Err<ParserErrorType<'a>>> {
    one_of(
        ('0'..='9')
            .chain('a'..='z')
            .chain('A'..='Z')
            .chain(['-'])
            .collect::<String>(),
    )
    .labelled("alphanumeric or '-'")
}

pub fn dot_sep_idents<'a>()
-> impl Parser<'a, &'a str, Vec<String>, extra::Err<ParserErrorType<'a>>> {
    ident()
        .repeated()
        .at_least(1)
        .collect::<String>()
        .separated_by(just('.'))
        .collect::<Vec<_>>()
        .labelled("dot-separated list")
}

pub fn int<'a>()
-> impl Parser<'a, &'a str, u32, extra::Err<ParserErrorType<'a>>> {
    one_of('0'..='9')
        .labelled("digit")
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(|s| s.parse::<u32>().unwrap_or(0))
        .labelled("integer")
}

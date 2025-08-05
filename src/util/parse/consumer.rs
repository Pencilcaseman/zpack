use super::cursor::Cursor;

use color_eyre::{Result, eyre::eyre};

pub trait Consumer: std::fmt::Debug {
    type Output;

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)>;
}

#[derive(Debug)]
pub struct LiteralConsumer<'a> {
    target: &'a str,
}

impl<'a> LiteralConsumer<'a> {
    pub fn new(target: &'a str) -> Self {
        Self { target }
    }
}

impl<'a> Consumer for LiteralConsumer<'a> {
    type Output = ();

    fn consume<'b>(
        &self,
        cursor: Cursor<'b>,
    ) -> Result<(Self::Output, Cursor<'b>)> {
        let target_len = self.target.len();
        let (extract, cursor) = cursor.step(target_len)?;

        if extract == self.target {
            Ok(((), cursor))
        } else {
            Err(eyre!("Expected '{}'; received '{extract}'", self.target))
        }
    }
}

#[derive(Debug)]
pub struct OptionalConsumer<T>
where
    T: Consumer,
{
    opt: T,
}

impl<T> OptionalConsumer<T>
where
    T: Consumer,
{
    pub fn new(opt: T) -> Self {
        Self { opt }
    }
}

impl<T> Consumer for OptionalConsumer<T>
where
    T: Consumer,
{
    type Output = Option<<T as Consumer>::Output>;

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)> {
        if let Ok((result, c)) = self.opt.consume(cursor) {
            Ok((Some(result), c))
        } else {
            Ok((None, cursor))
        }
    }
}

#[derive(Debug)]
pub struct BoundedConsumer<T>
where
    T: Consumer,
{
    min: Option<usize>,
    max: Option<usize>,
    parser: T,
}

impl<T> BoundedConsumer<T>
where
    T: Consumer,
{
    pub fn new(min: Option<usize>, max: Option<usize>, parser: T) -> Self {
        Self { min, max, parser }
    }

    pub fn zero_or_more(parser: T) -> Self {
        Self { min: Some(0), max: None, parser }
    }

    pub fn one_or_more(parser: T) -> Self {
        Self { min: Some(1), max: None, parser }
    }
}

impl<T> Consumer for BoundedConsumer<T>
where
    T: Consumer + std::fmt::Debug,
    <T as Consumer>::Output: std::fmt::Debug,
{
    type Output = Vec<T::Output>;

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)> {
        let mut res = Vec::new();
        let mut cur = cursor;

        while let Ok((parsed, new_cursor)) = self.parser.consume(cur) {
            res.push(parsed);
            cur = new_cursor;
        }

        let min = self.min.unwrap_or(0);
        let max = self.max.unwrap_or(usize::MAX);

        if (min..max).contains(&res.len()) {
            Ok((res, cur))
        } else {
            let desc = match (self.min, self.max) {
                (Some(min), Some(max)) => format!("between {min} and {max}"),
                (Some(min), None) => format!("{min}..."),
                (None, Some(max)) => format!("...{max}"),
                (None, None) => "0...".to_string(),
            };

            Err(eyre!(
                "Expected {desc} instances of {:?}; found {}",
                self.parser,
                res.len()
            ))
        }
    }
}

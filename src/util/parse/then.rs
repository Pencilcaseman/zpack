use super::{consumer::Consumer, cursor::Cursor};
use anyhow::Result;

#[derive(Debug)]
pub enum ThenError<First, Second> {
    First(First),
    Second(Second),
}

pub struct Then<First, Second>
where
    First: Consumer,
    Second: Consumer,
{
    first: First,
    second: Second,
}

impl<First, Second> Then<First, Second>
where
    First: Consumer,
    Second: Consumer,
{
    pub fn new(first: First, second: Second) -> Self {
        Self { first, second }
    }
}

impl<First, Second> Consumer for Then<First, Second>
where
    First: Consumer,
    Second: Consumer,
{
    type Output = (First::Output, Second::Output);
    type Error = ThenError<First::Error, Second::Error>;

    fn info(&self) -> String {
        format!("{} then {}", self.first.info(), self.second.info())
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>), Self::Error> {
        let (first, cur) = match self.first.consume(cursor) {
            Ok((f, c)) => (f, c),
            Err(e) => return Err(ThenError::First(e)),
        };

        let (second, cur) = match self.second.consume(cur) {
            Ok((s, c)) => (s, c),
            Err(e) => return Err(ThenError::Second(e)),
        };

        Ok(((first, second), cur))
    }
}

pub struct IgnoreThen<First, Second>
where
    First: Consumer,
    Second: Consumer,
{
    first: First,
    second: Second,
}

impl<First, Second> IgnoreThen<First, Second>
where
    First: Consumer,
    Second: Consumer,
{
    pub fn new(first: First, second: Second) -> Self {
        Self { first, second }
    }
}

impl<First, Second> Consumer for IgnoreThen<First, Second>
where
    First: Consumer,
    Second: Consumer,
{
    type Output = Second::Output;
    type Error = ThenError<First::Error, Second::Error>;

    fn info(&self) -> String {
        format!("{} then {}", self.first.info(), self.second.info())
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>), Self::Error> {
        let cur = match self.first.consume(cursor) {
            Ok((_, c)) => c,
            Err(e) => return Err(ThenError::First(e)),
        };

        let (second, cur) = match self.second.consume(cur) {
            Ok((s, c)) => (s, c),
            Err(e) => return Err(ThenError::Second(e)),
        };

        Ok((second, cur))
    }
}

pub struct ThenIgnore<First, Second>
where
    First: Consumer,
    Second: Consumer,
{
    first: First,
    second: Second,
}

impl<First, Second> ThenIgnore<First, Second>
where
    First: Consumer,
    Second: Consumer,
{
    pub fn new(first: First, second: Second) -> Self {
        Self { first, second }
    }
}

impl<First, Second> Consumer for ThenIgnore<First, Second>
where
    First: Consumer,
    Second: Consumer,
{
    type Output = First::Output;
    type Error = ThenError<First::Error, Second::Error>;

    fn info(&self) -> String {
        format!("{} then {}", self.first.info(), self.second.info())
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>), Self::Error> {
        let (first, cur) = match self.first.consume(cursor) {
            Ok((f, c)) => (f, c),
            Err(e) => return Err(ThenError::First(e)),
        };

        let cur = match self.second.consume(cur) {
            Ok((_, c)) => c,
            Err(e) => return Err(ThenError::Second(e)),
        };

        Ok((first, cur))
    }
}

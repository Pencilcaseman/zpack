use super::{consumer::Consumer, cursor::Cursor};
use color_eyre::Result;

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

    fn info(&self) -> String {
        format!("{} then {}", self.first.info(), self.second.info())
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)> {
        let (first, cur) = self.first.consume(cursor)?;
        let (second, cur) = self.second.consume(cur)?;
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

    fn info(&self) -> String {
        format!("{} then {}", self.first.info(), self.second.info())
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)> {
        let (_, cur) = self.first.consume(cursor)?;
        let (second, cur) = self.second.consume(cur)?;
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

    fn info(&self) -> String {
        format!("{} then {}", self.first.info(), self.second.info())
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)> {
        let (first, cur) = self.first.consume(cursor)?;
        let (_, cur) = self.second.consume(cur)?;
        Ok((first, cur))
    }
}

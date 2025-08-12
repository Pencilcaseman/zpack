use std::marker::PhantomData;

use super::{consumer::Consumer, cursor::Cursor};
use color_eyre::{Result, eyre::eyre};

#[derive(Copy, Clone)]
pub struct RawConsumer<P, Out>
where
    P: for<'a> Fn(Cursor<'a>) -> Result<(Out, Cursor<'a>)>,
    Out: 'static,
{
    predicate: P,
    phantom: PhantomData<Out>,
}

impl<P, Out> RawConsumer<P, Out>
where
    P: for<'a> Fn(Cursor<'a>) -> Result<(Out, Cursor<'a>)>,
{
    pub fn new(predicate: P) -> Self {
        Self { predicate, phantom: PhantomData }
    }
}

impl<P, Out> Consumer for RawConsumer<P, Out>
where
    P: for<'a> Fn(Cursor<'a>) -> Result<(Out, Cursor<'a>)>,
{
    type Output = Out;

    fn info(&self) -> String {
        "raw".into()
    }

    fn consume<'b>(
        &self,
        cursor: Cursor<'b>,
    ) -> Result<(Self::Output, Cursor<'b>)> {
        (self.predicate)(cursor)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_raw() {
        let raw = RawConsumer::new(|cursor| {
            cursor
                .take_while(|c| c.is_ascii_alphanumeric())
                .map(|(s, c)| (s.to_owned(), c))
        });
    }
}

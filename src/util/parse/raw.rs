use std::marker::PhantomData;

use anyhow::{Result, anyhow};

use super::{consumer::Consumer, cursor::Cursor};

#[derive(Debug, Copy, Clone)]
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
    fn test_raw() -> Result<()> {
        let raw = RawConsumer::new(|cursor| {
            cursor
                .take_while(|c| c.is_ascii_alphanumeric())
                .map(|(s, c)| (s.to_owned(), c))
                .ok_or(anyhow!("Stepped past end of string"))
        });

        let sample_text = "Hello, World!";
        let cursor = Cursor::new(sample_text);

        let (res, cur) = raw.consume(cursor)?;

        assert_eq!(res, "Hello");
        assert_eq!(cur.remaining(), ", World!");

        Ok(())
    }
}

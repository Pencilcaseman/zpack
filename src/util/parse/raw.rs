use std::marker::PhantomData;

use super::{consumer::Consumer, cursor::Cursor};

#[derive(Copy, Clone)]
pub struct RawConsumer<P, Out, Err>
where
    P: for<'a> Fn(Cursor<'a>) -> Result<(Out, Cursor<'a>), Err>,
    Out: 'static,
{
    predicate: P,
    phantom: PhantomData<Out>,
}

impl<P, Out, Err> RawConsumer<P, Out, Err>
where
    P: for<'a> Fn(Cursor<'a>) -> Result<(Out, Cursor<'a>), Err>,
{
    pub fn new(predicate: P) -> Self {
        Self { predicate, phantom: PhantomData }
    }
}

impl<P, Out, Err> Consumer for RawConsumer<P, Out, Err>
where
    P: for<'a> Fn(Cursor<'a>) -> Result<(Out, Cursor<'a>), Err>,
{
    type Output = Out;
    type Error = Err;

    fn info(&self) -> String {
        "raw".into()
    }

    fn consume<'b>(
        &self,
        cursor: Cursor<'b>,
    ) -> Result<(Self::Output, Cursor<'b>), Err> {
        (self.predicate)(cursor)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_raw() -> Result<(), ()> {
        let raw = RawConsumer::new(|cursor| {
            cursor
                .take_while(|c| c.is_ascii_alphanumeric())
                .map(|(s, c)| (s.to_owned(), c))
                .ok_or(())
        });

        let sample_text = "Hello, World!";
        let cursor = Cursor::new(sample_text);

        let (res, cur) = raw.consume(cursor)?;

        assert_eq!(res, "Hello");
        assert_eq!(cur.remaining(), ", World!");

        Ok(())
    }
}

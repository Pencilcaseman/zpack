use super::{consumer::Consumer, cursor::Cursor};
use color_eyre::{Result, eyre::eyre};

pub struct Map<T, F, R>
where
    T: Consumer,
    F: Fn(<T as Consumer>::Output) -> Result<R>,
    R: 'static,
{
    consumer: T,
    function: F,
}

impl<T, F, R> Map<T, F, R>
where
    T: Consumer,
    F: Fn(<T as Consumer>::Output) -> Result<R>,
{
    pub fn new(input: T, function: F) -> Self {
        Self { consumer: input, function }
    }
}

impl<T, F, R> Consumer for Map<T, F, R>
where
    T: Consumer,
    F: Fn(<T as Consumer>::Output) -> Result<R>,
{
    type Output = R;

    fn info(&self) -> String {
        format!("f({})", self.consumer.info())
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)> {
        let (res, cur) = self.consumer.consume(cursor)?;
        Ok(((self.function)(res)?, cur))
    }
}

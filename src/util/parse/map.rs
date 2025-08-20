use super::{consumer::Consumer, cursor::Cursor};

pub enum MapError<C, F> {
    ConsumeError(C),
    FunctionError(F),
}

pub struct Map<T, F, R, E>
where
    T: Consumer,
    F: Fn(<T as Consumer>::Output) -> Result<R, E>,
    R: 'static,
{
    consumer: T,
    function: F,
}

impl<T, F, R, E> Map<T, F, R, E>
where
    T: Consumer,
    F: Fn(<T as Consumer>::Output) -> Result<R, E>,
{
    pub fn new(input: T, function: F) -> Self {
        Self { consumer: input, function }
    }
}

impl<T, F, R, E> Consumer for Map<T, F, R, E>
where
    T: Consumer,
    F: Fn(<T as Consumer>::Output) -> Result<R, E>,
{
    type Output = R;
    type Error = MapError<<T as Consumer>::Error, E>;

    fn info(&self) -> String {
        format!("f({})", self.consumer.info())
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>), Self::Error> {
        let (res, cur) =
            self.consumer.consume(cursor).map_err(MapError::ConsumeError)?;
        Ok(((self.function)(res).map_err(MapError::FunctionError)?, cur))
    }
}

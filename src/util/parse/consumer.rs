use super::cursor::Cursor;

pub trait Consumer {
    type Output: 'static;
    type Error;

    fn info(&self) -> String;

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>), Self::Error>;
}

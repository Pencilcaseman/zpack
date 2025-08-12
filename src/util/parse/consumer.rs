use super::cursor::Cursor;

use color_eyre::Result;

pub trait Consumer {
    type Output: 'static;

    fn info(&self) -> String;

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)>;
}

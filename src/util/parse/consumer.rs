use anyhow::Result;

use super::cursor::Cursor;

pub trait Consumer {
    type Output: 'static;

    fn info(&self) -> String;

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)>;
}

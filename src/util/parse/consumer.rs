use super::cursor::Cursor;

use color_eyre::Result;

pub trait Consumer: std::fmt::Debug {
    type Output;

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)>;
}

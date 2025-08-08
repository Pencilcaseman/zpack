use super::{consumer::Consumer, cursor::Cursor};
use color_eyre::{Result, eyre::eyre, owo_colors::colors::White};

#[derive(Default, Debug)]
pub struct WhitespaceConsumer {}

impl Consumer for WhitespaceConsumer {
    type Output = ();

    fn consume<'b>(
        &self,
        mut cursor: Cursor<'b>,
    ) -> Result<(Self::Output, Cursor<'b>)> {
        while cursor.peek().is_ok_and(|c| c.is_ascii_whitespace()) {
            let _ = cursor.step_mut(1)?;
        }

        Ok(((), cursor))
    }
}

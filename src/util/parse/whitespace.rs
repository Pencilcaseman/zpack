use super::{consumer::Consumer, cursor::Cursor};
use color_eyre::{Result, eyre::eyre, owo_colors::colors::White};

#[derive(Default, Debug)]
pub struct WhitespaceConsumer {}

impl Consumer for WhitespaceConsumer {
    type Output = ();

    fn info(&self) -> String {
        "whitespace".into()
    }

    fn consume<'b>(
        &self,
        cursor: Cursor<'b>,
    ) -> Result<(Self::Output, Cursor<'b>)> {
        if let Ok((white, cur)) = cursor.take_while(|c| c.is_ascii_whitespace())
            && !white.is_empty()
        {
            Ok(((), cur))
        } else {
            Err(eyre!("Expected whitespace"))
        }
    }
}

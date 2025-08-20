use super::{consumer::Consumer, cursor::Cursor};

#[derive(Default, Debug)]
pub struct WhitespaceConsumer {}

impl Consumer for WhitespaceConsumer {
    type Output = ();
    type Error = &'static str;

    fn info(&self) -> String {
        "whitespace".into()
    }

    fn consume<'b>(
        &self,
        cursor: Cursor<'b>,
    ) -> Result<(Self::Output, Cursor<'b>), Self::Error> {
        if let Some((white, cur)) =
            cursor.take_while(|c| c.is_ascii_whitespace())
            && !white.is_empty()
        {
            Ok(((), cur))
        } else {
            Err("Expected whitespace")
        }
    }
}

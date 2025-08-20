use super::{consumer::Consumer, cursor::Cursor};

#[derive(Copy, Clone)]
pub struct IntegerConsumer {}

impl IntegerConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for IntegerConsumer {
    fn default() -> Self {
        Self::new()
    }
}

impl Consumer for IntegerConsumer {
    type Output = i128;
    type Error = ();

    fn info(&self) -> String {
        "integer".into()
    }

    fn consume<'b>(
        &self,
        cursor: Cursor<'b>,
    ) -> Result<(Self::Output, Cursor<'b>), Self::Error> {
        match cursor.take_while(|c| c.is_ascii_digit()) {
            Some((p, c)) if !p.is_empty() => match p.parse::<Self::Output>() {
                Ok(v) => Ok((v, c)),
                Err(_) => Err(()),
            },
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::util::parse::ConsumerExt;
    use crate::util::parse::WhitespaceConsumer;

    use super::*;

    #[test]
    fn test_literal() {
        let parser = IntegerConsumer::default()
            .then_ignore(WhitespaceConsumer::default());

        let sample_text = "12345Hello";
        let sample_cursor = Cursor::new(sample_text);

        match parser.consume(sample_cursor) {
            Ok((val, cur)) => {
                assert_eq!(val, 12345);
                assert_eq!(cur.remaining(), "Hello");
            }
            Err(e) => panic!("Failed to parse: {e:?}"),
            // Err(e) => panic!("Failed to parse"),
        }
    }
}

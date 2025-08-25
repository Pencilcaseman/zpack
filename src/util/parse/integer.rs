use anyhow::{Result, anyhow};

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

    fn info(&self) -> String {
        "integer".into()
    }

    fn consume<'b>(
        &self,
        cursor: Cursor<'b>,
    ) -> Result<(Self::Output, Cursor<'b>)> {
        match cursor.take_while(|c| c.is_ascii_digit()) {
            Some((p, c)) if !p.is_empty() => match p.parse::<Self::Output>() {
                Ok(v) => Ok((v, c)),
                Err(e) => Err(anyhow!(
                    "Expected valid integer. Failed because: {e:?}"
                )),
            },
            _ => Err(anyhow!("Expected integer. Received empty string")),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util::parse::{ConsumerExt, WhitespaceConsumer};

    #[test]
    fn test_literal() {
        let parser = IntegerConsumer::default()
            .then_ignore(WhitespaceConsumer::default());

        let sample_text = "12345 Hello";
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

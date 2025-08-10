use super::{consumer::Consumer, cursor::Cursor};
use color_eyre::{Result, eyre::eyre};

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
        let (p, c) = cursor.take_while(|c| c.is_ascii_digit())?;
        Ok((p.parse()?, c))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_literal() {
        let parser = IntegerConsumer::default();

        let sample_text = "12345Hello";
        let sample_cursor = Cursor::new(sample_text);

        match parser.consume(sample_cursor) {
            Ok((val, cur)) => {
                assert_eq!(val, 12345);
                assert_eq!(cur.remaining(), "Hello");
            }
            Err(e) => panic!("Failed to parse: {e:?}"),
        }
    }
}

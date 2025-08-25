use anyhow::{Result, anyhow};

use super::{consumer::Consumer, cursor::Cursor};

#[derive(Debug, Copy, Clone)]
pub struct AlphanumericConsumer {}

impl AlphanumericConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AlphanumericConsumer {
    fn default() -> Self {
        Self::new()
    }
}

impl Consumer for AlphanumericConsumer {
    type Output = String;

    fn info(&self) -> String {
        "alphanumeric".into()
    }

    fn consume<'b>(
        &self,
        cursor: Cursor<'b>,
    ) -> Result<(Self::Output, Cursor<'b>)> {
        match cursor.take_while(|c| c.is_alphanumeric()) {
            Some((txt, c)) if !txt.is_empty() => Ok((txt.to_string(), c)),
            _ => Err(anyhow!("Expected alphanumeric string")),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_alphanumeric() {
        let parser = AlphanumericConsumer::default();

        let sample_text = "HelloWorld123.Hello";
        let sample_cursor = Cursor::new(sample_text);

        match parser.consume(sample_cursor) {
            Ok((val, cur)) => {
                assert_eq!(val, "HelloWorld123");
                assert_eq!(cur.remaining(), ".Hello");
            }
            Err(e) => panic!("Failed to parse: {e:?}"),
        }
    }
}

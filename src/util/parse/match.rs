use super::{consumer::Consumer, cursor::Cursor};
use anyhow::Result;

#[derive(Debug)]
pub struct MatchConsumerError<'a> {
    expected: &'a str,
    received: String,
}

#[derive(Debug, Copy, Clone)]
pub struct MatchConsumer<'a> {
    target: &'a str,
}

impl<'a> MatchConsumer<'a> {
    pub fn new(target: &'a str) -> Self {
        Self { target }
    }
}

impl<'a> Consumer for MatchConsumer<'a> {
    type Output = ();
    type Error = MatchConsumerError<'a>;

    fn info(&self) -> String {
        format!("matching '{}'", self.target)
    }

    fn consume<'b>(
        &self,
        cursor: Cursor<'b>,
    ) -> Result<(Self::Output, Cursor<'b>), Self::Error> {
        let target_len = self.target.len();
        let remaining = cursor.remaining();
        if let Some((extract, cursor)) = cursor.step(target_len)
            && extract == self.target
        {
            Ok(((), cursor))
        } else {
            Err(MatchConsumerError {
                expected: self.target,
                received: remaining.into(),
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_literal() {
        let parser = MatchConsumer::new("Hello");

        let sample_text = "Hello, World!";
        let sample_cursor = Cursor::new(sample_text);

        match parser.consume(sample_cursor) {
            Ok((_, cur)) => {
                assert_eq!(cur.remaining(), ", World!");
            }
            Err(e) => panic!("Failed to parse: {e:?}"),
        }
    }
}

use super::{consumer::Consumer, cursor::Cursor};
use color_eyre::{Result, eyre::eyre};

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

    fn info(&self) -> String {
        format!("matching '{}'", self.target)
    }

    fn consume<'b>(
        &self,
        cursor: Cursor<'b>,
    ) -> Result<(Self::Output, Cursor<'b>)> {
        let target_len = self.target.len();
        let (extract, cursor) = cursor.step(target_len)?;

        if extract == self.target {
            Ok(((), cursor))
        } else {
            Err(eyre!("Expected '{}'; received '{extract}", self.target))
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

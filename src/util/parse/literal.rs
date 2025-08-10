use super::{consumer::Consumer, cursor::Cursor};
use color_eyre::{Result, eyre::eyre};

#[derive(Debug, Copy, Clone)]
pub struct LiteralConsumer<'a> {
    target: &'a str,
}

impl<'a> LiteralConsumer<'a> {
    pub fn new(target: &'a str) -> Self {
        Self { target }
    }
}

impl<'a> Consumer for LiteralConsumer<'a> {
    type Output = ();

    fn info(&self) -> String {
        format!("literal '{}'", self.target)
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
            Err(eyre!("literal '{extract}'"))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_literal() {
        let parser = LiteralConsumer::new("Hello");

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

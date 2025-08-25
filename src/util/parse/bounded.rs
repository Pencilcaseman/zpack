use anyhow::{Result, anyhow};

use super::{consumer::Consumer, cursor::Cursor};

#[derive(Debug)]
pub struct BoundedConsumer<T>
where
    T: Consumer,
{
    min: Option<usize>,
    max: Option<usize>,
    parser: T,
}

impl<T> BoundedConsumer<T>
where
    T: Consumer,
{
    pub fn new(min: Option<usize>, max: Option<usize>, parser: T) -> Self {
        Self { min, max, parser }
    }

    pub fn zero_or_more(parser: T) -> Self {
        Self { min: Some(0), max: None, parser }
    }

    pub fn one_or_more(parser: T) -> Self {
        Self { min: Some(1), max: None, parser }
    }
}

impl<T> Consumer for BoundedConsumer<T>
where
    T: Consumer + std::fmt::Debug,
    <T as Consumer>::Output: std::fmt::Debug,
{
    type Output = Vec<T::Output>;

    fn info(&self) -> String {
        match (self.min, self.max) {
            (Some(min), Some(max)) => {
                format!(
                    "Between {min} and {max} instances of {}",
                    self.parser.info(),
                )
            }
            (Some(min), None) => {
                format!(
                    "Expected at least {min} instances of {}",
                    self.parser.info(),
                )
            }
            (None, Some(max)) => {
                format!(
                    "Expected at most {max} instances of {}",
                    self.parser.info(),
                )
            }
            (None, None) => {
                format!("Expected at least one {}", self.parser.info(),)
            }
        }
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)> {
        let mut res = Vec::new();
        let mut cur = cursor;

        while let Ok((parsed, new_cursor)) = self.parser.consume(cur) {
            res.push(parsed);
            cur = new_cursor;
        }

        let min = self.min.unwrap_or(0);
        let max = self.max.unwrap_or(usize::MAX);

        if (min..max).contains(&res.len()) {
            Ok((res, cur))
        } else {
            Err(anyhow!("Expected {}. Received {}", self.info(), res.len()))
        }
    }
}

#[cfg(test)]
mod test {
    use super::{super::MatchConsumer, *};

    #[test]
    fn test_bounded_single() {
        let a = MatchConsumer::new("A");
        let aaa = BoundedConsumer::new(Some(0), Some(5), a);

        let sample_text = "AAAAhello";
        let sample_cursor = Cursor::new(sample_text);

        match aaa.consume(sample_cursor) {
            Ok((found, cur)) => {
                assert_eq!(found.len(), 4);
                assert_eq!(cur.remaining(), "hello");
            }
            Err(e) => panic!("Failed to parse: {e:?}"),
        }
    }

    #[test]
    fn test_bounded_multi() {
        let a = MatchConsumer::new("void");
        let aaa = BoundedConsumer::new(Some(0), None, a);

        let sample_text = "voidvoidvoidvoidvoidvoid123456void";
        let sample_cursor = Cursor::new(sample_text);

        match aaa.consume(sample_cursor) {
            Ok((found, cur)) => {
                assert_eq!(found.len(), 6);
                assert_eq!(cur.remaining(), "123456void");
            }
            Err(e) => panic!("Failed to parse: {e:?}"),
        }
    }
}

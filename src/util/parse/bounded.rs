use super::{consumer::Consumer, cursor::Cursor};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BoundedConsumerError {
    ExpectedAtLeast(usize),
    ExpectedAtMost(usize),
    ExpectedBetween(usize, usize),
}

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
    type Error = BoundedConsumerError;

    fn info(&self) -> String {
        format!("BoundedConsumerParser")
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>), Self::Error> {
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
            Err(match (self.min, self.max) {
                (Some(min), Some(max)) => {
                    BoundedConsumerError::ExpectedBetween(min, max)
                }
                (Some(min), None) => BoundedConsumerError::ExpectedAtLeast(min),
                (None, Some(max)) => BoundedConsumerError::ExpectedAtMost(max),
                (None, None) => {
                    BoundedConsumerError::ExpectedBetween(0, usize::MAX)
                }
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::MatchConsumer;
    use super::*;

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

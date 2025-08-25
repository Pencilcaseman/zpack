use anyhow::Result;

use super::{consumer::Consumer, cursor::Cursor};

#[derive(Debug, Copy, Clone)]
pub struct OptionalConsumer<T>
where
    T: Consumer,
{
    opt: T,
}

impl<T> OptionalConsumer<T>
where
    T: Consumer,
{
    pub fn new(opt: T) -> Self {
        Self { opt }
    }
}

impl<T> Consumer for OptionalConsumer<T>
where
    T: Consumer,
{
    type Output = Option<<T as Consumer>::Output>;

    fn info(&self) -> String {
        format!("optional[{}]", self.opt.info())
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)> {
        if let Ok((result, c)) = self.opt.consume(cursor) {
            Ok((Some(result), c))
        } else {
            Ok((None, cursor))
        }
    }
}

#[cfg(test)]
mod test {
    use super::{super::MatchConsumer, *};

    #[test]
    fn test_optional_matching() {
        let lit = MatchConsumer::new("Optional");
        let optional_consumer = OptionalConsumer::new(lit);

        let matching = "OptionalString";
        let matching_cursor = Cursor::new(matching);

        match optional_consumer.consume(matching_cursor) {
            Ok((matched, cursor)) => {
                assert!(matched.is_some());
                assert_eq!(cursor.remaining(), "String");
            }
            Err(e) => panic!("Failed to parse: {e:?}"),
        }
    }

    #[test]
    fn test_optional_not_matching() {
        let lit = MatchConsumer::new("Optional");
        let optional_consumer = OptionalConsumer::new(lit);

        let not_matching = "StringOptional";
        let not_matching_cursor = Cursor::new(not_matching);

        match optional_consumer.consume(not_matching_cursor) {
            Ok((matched, cursor)) => {
                assert!(matched.is_none());
                assert_eq!(cursor.remaining(), "StringOptional");
            }
            Err(e) => {
                panic!("Something has gone wrong: {e:?}");
            }
        }
    }
}

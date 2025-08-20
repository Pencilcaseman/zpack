use super::{consumer::Consumer, cursor::Cursor};
use anyhow::{Result, anyhow};
use itertools::Itertools;

pub struct MultiConsumer<O, E>
where
    O: 'static,
{
    consumers: Vec<Box<dyn Consumer<Output = O, Error = E>>>,
}

impl<O, E> MultiConsumer<O, E>
where
    O: 'static,
{
    pub fn new() -> Self {
        Self { consumers: Default::default() }
    }

    pub fn new_with(
        consumers: Vec<Box<dyn Consumer<Output = O, Error = E>>>,
    ) -> Self {
        Self { consumers }
    }

    pub fn push<C>(&mut self, consumer: C)
    where
        C: Consumer<Output = O, Error = E> + 'static,
    {
        self.consumers.push(Box::new(consumer));
    }
}

impl<O, E> Default for MultiConsumer<O, E> {
    fn default() -> Self {
        Self { consumers: Vec::new() }
    }
}

impl<O, E> Consumer for MultiConsumer<O, E>
where
    O: 'static + std::fmt::Debug,
{
    type Output = O;
    type Error = ();

    fn info(&self) -> String {
        "one of: ".to_owned()
            + &self.consumers.iter().map(|c| c.info()).join(", ")
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>), Self::Error> {
        self.consumers
            .iter()
            .find_map(|consumer| {
                if let Ok((res, cur)) = consumer.consume(cursor) {
                    Some((res, cur))
                } else {
                    None
                }
            })
            .ok_or(()) // Nothing matched -- no extra information necessary
    }
}

#[cfg(test)]
mod test {
    use super::super::{EnumConsumer, MatchConsumer};
    use super::*;

    #[test]
    fn test_multi_consumer() {
        #[derive(Debug, PartialEq)]
        enum TestEnum {
            Class,
            Function,
        }

        let class_lit = MatchConsumer::new("class");
        let class_enum =
            EnumConsumer::new(class_lit, |_| Some(TestEnum::Class));

        let function_lit = MatchConsumer::new("function");
        let function_enum =
            EnumConsumer::new(function_lit, |_| Some(TestEnum::Function));

        let mut multi_consumer = MultiConsumer::default();
        multi_consumer.push(class_enum);
        multi_consumer.push(function_enum);

        let text = "classfunction;";
        let mut cur = Cursor::new(text);

        match multi_consumer.consume(cur) {
            Ok((matched, cursor)) => {
                assert_eq!(matched, TestEnum::Class);
                assert_eq!(cursor.remaining(), "function;");

                cur = cursor;
            }
            Err(e) => {
                panic!("Something has gone wrong: {e:?}");
            }
        }

        match multi_consumer.consume(cur) {
            Ok((matched, cursor)) => {
                assert_eq!(matched, TestEnum::Function);
                assert_eq!(cursor.remaining(), ";");
            }
            Err(e) => {
                panic!("Something has gone wrong: {e:?}");
            }
        }
    }
}

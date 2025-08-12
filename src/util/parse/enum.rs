use super::{consumer::Consumer, cursor::Cursor};
use color_eyre::{Result, eyre::eyre};

#[derive(Debug)]
pub struct EnumConsumer<E, T>
where
    E: std::fmt::Debug + 'static,
    T: Consumer,
{
    consumer: T,
    conv: fn(<T as Consumer>::Output) -> Option<<Self as Consumer>::Output>,
}

impl<E, T> EnumConsumer<E, T>
where
    E: std::fmt::Debug + 'static,
    T: Consumer,
{
    pub fn new(
        consumer: T,
        conv: fn(<T as Consumer>::Output) -> Option<<Self as Consumer>::Output>,
    ) -> Self {
        Self { consumer, conv }
    }
}

impl<E, T> Consumer for EnumConsumer<E, T>
where
    T: Consumer,
    E: std::fmt::Debug + 'static,
{
    type Output = E;

    fn info(&self) -> String {
        self.consumer.info().to_string()
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)> {
        match self.consumer.consume(cursor) {
            Ok((res, cur)) => match (self.conv)(res) {
                Some(val) => Ok((val, cur)),
                None => Err(eyre!("Failed to parse")),
            },
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::MatchConsumer;
    use super::*;

    #[test]
    fn test_enum_consumer() {
        #[derive(Debug, PartialEq)]
        enum TestEnum {
            Class,
        }

        let class_lit = MatchConsumer::new("class");
        let class_enum =
            EnumConsumer::new(class_lit, |_| Some(TestEnum::Class));

        let text = "class MyClass;";
        let cur = Cursor::new(text);

        match class_enum.consume(cur) {
            Ok((matched, cursor)) => {
                assert_eq!(matched, TestEnum::Class);
                assert_eq!(cursor.remaining(), " MyClass;");
            }
            Err(e) => {
                panic!("Something has gone wrong: {e:?}");
            }
        }
    }
}

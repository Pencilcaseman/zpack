use anyhow::{Result, anyhow};

use super::{consumer::Consumer, cursor::Cursor};

#[derive(Debug)]
pub struct EnumConsumer<T, E>
where
    T: Consumer,
{
    consumer: T,
    conv: fn(<T as Consumer>::Output) -> Result<E>,
}

impl<T, E> EnumConsumer<T, E>
where
    T: Consumer,
{
    pub fn new(
        consumer: T,
        conv: fn(<T as Consumer>::Output) -> Result<E>,
    ) -> Self {
        Self { consumer, conv }
    }
}

impl<T, E> Consumer for EnumConsumer<T, E>
where
    T: Consumer,
    E: 'static,
{
    type Output = E;

    fn info(&self) -> String {
        self.consumer.info().to_string()
    }

    fn consume<'a>(
        &self,
        cursor: Cursor<'a>,
    ) -> Result<(Self::Output, Cursor<'a>)> {
        let (out, cur) = self.consumer.consume(cursor)?;
        Ok(((self.conv)(out)?, cur))
    }
}

#[cfg(test)]
mod test {
    use super::{super::MatchConsumer, *};

    #[test]
    fn test_enum_consumer() {
        #[derive(Debug, PartialEq)]
        enum TestEnum {
            Class,
        }

        let class_lit = MatchConsumer::new("class");
        let class_enum = EnumConsumer::new(class_lit, |_| Ok(TestEnum::Class));

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

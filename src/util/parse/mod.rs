pub mod consumer;
pub mod cursor;

#[cfg(test)]
mod test {
    use super::consumer::Consumer;
    use super::*;

    #[test]
    fn test_cursor() {
        let sample_text = "12345";
        let mut sample_cursor = cursor::Cursor::new(sample_text);

        assert!(sample_cursor.step(4).is_ok());
    }

    #[test]
    fn test_literal() {
        let parser = consumer::LiteralConsumer::new("Hello");

        let sample_text = "Hello, World!";
        let sample_cursor = cursor::Cursor::new(sample_text);

        match parser.consume(sample_cursor) {
            Ok((_, cur)) => {
                assert_eq!(cur.remaining(), ", World!");
            }
            Err(e) => panic!("Failed to parse: {e:?}"),
        }
    }

    #[test]
    fn test_bounded_single() {
        let a = consumer::LiteralConsumer::new("A");
        let aaa = consumer::BoundedConsumer::new(Some(0), Some(5), a);

        let sample_text = "AAAAhello";
        let sample_cursor = cursor::Cursor::new(sample_text);

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
        let a = consumer::LiteralConsumer::new("void");
        let aaa = consumer::BoundedConsumer::new(Some(0), None, a);

        let sample_text = "voidvoidvoidvoidvoidvoid123456void";
        let sample_cursor = cursor::Cursor::new(sample_text);

        match aaa.consume(sample_cursor) {
            Ok((found, cur)) => {
                assert_eq!(found.len(), 6);
                assert_eq!(cur.remaining(), "123456void");
            }
            Err(e) => panic!("Failed to parse: {e:?}"),
        }
    }

    #[test]
    fn test_optional_matching() {
        let lit = consumer::LiteralConsumer::new("Optional");
        let optional_consumer = consumer::OptionalConsumer::new(lit);

        let matching = "OptionalString";
        let matching_cursor = cursor::Cursor::new(matching);

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
        let lit = consumer::LiteralConsumer::new("Optional");
        let optional_consumer = consumer::OptionalConsumer::new(lit);

        let not_matching = "StringOptional";
        let not_matching_cursor = cursor::Cursor::new(not_matching);

        match optional_consumer.consume(not_matching_cursor) {
            Ok((matched, cursor)) => {
                assert!(matched.is_none());
                assert_eq!(cursor.remaining(), "StringOptional");
            }
            Err(e) => {
                panic!("Something has gone wrong");
            }
        }
    }
}

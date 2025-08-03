pub mod consumer;
pub mod cursor;

#[cfg(test)]
mod test {
    use super::consumer::Consumer;
    use super::*;

    #[test]
    fn test_literal() {
        let parser = consumer::LiteralParser::new("Hello");

        let sample_text = "Hello, World!";
        let cur = cursor::Cursor::new(sample_text);

        match parser.consume(cur) {
            Ok((_, cur)) => {
                assert_eq!(cur.remaining(), ", World!");
            }
            Err(e) => panic!("Failed to parse: {e:?}"),
        }
    }
}

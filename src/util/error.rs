use ariadne::{ColorGenerator, Label};
#[cfg(feature = "cheap_errors")]
use chumsky::error::Cheap;
#[cfg(not(feature = "cheap_errors"))]
use chumsky::error::Rich;

#[cfg(not(feature = "cheap_errors"))]
pub type ParserErrorType<'a> = Rich<'a, char>;
#[cfg(feature = "cheap_errors")]
pub type ParserErrorType<'a> = Cheap;

#[derive(Debug, Clone)]
pub struct ParserErrorWrapper<'a, SourceType> {
    pub name: &'a str,
    pub source: SourceType,
    pub errors: Vec<ParserErrorType<'a>>,
}

#[derive(Debug)]
pub struct FinalizedParserError<'a, SourceType> {
    pub name: &'a str,
    pub source: SourceType,
    pub report: ariadne::Report<'a, (&'a str, std::ops::Range<usize>)>,
}

impl<'a, SourceType> ParserErrorWrapper<'a, SourceType> {
    pub fn new(
        name: &'a str,
        source: SourceType,
        errors: Vec<ParserErrorType<'a>>,
    ) -> Self {
        Self { name, source, errors }
    }

    pub fn push(&mut self, errs: Vec<ParserErrorType<'a>>) {
        self.errors.extend(errs);
    }

    pub fn build(
        self,
    ) -> std::option::Option<FinalizedParserError<'a, SourceType>> {
        if self.errors.is_empty() {
            return None;
        }

        let mut colors = ColorGenerator::new();

        let mut result = ariadne::Report::build(
            ariadne::ReportKind::Error,
            (self.name, self.errors[0].span().into_range()),
        );

        for err in self.errors.into_iter() {
            let c = colors.next();

            #[cfg(not(feature = "cheap_errors"))]
            let label = Label::new((self.name, err.span().into_range()))
                .with_message(err.reason())
                .with_color(c);

            #[cfg(feature = "cheap_errors")]
            let label = Label::new((self.name, err.span().into_range()))
                .with_message(
                    "Error. Remove feature `cheap_errors` for more detail",
                )
                .with_color(c);

            result = result.with_label(label);
        }

        Some(FinalizedParserError {
            name: self.name,
            source: self.source,
            report: result.finish(),
        })
    }
}

impl<'a, SourceType> FinalizedParserError<'a, SourceType>
where
    (&'a str, SourceType): ariadne::Cache<&'a str>,
{
    pub fn eprint(self) -> std::io::Result<()> {
        self.report.eprint((self.name, self.source))
    }

    pub fn to_string(self) -> anyhow::Result<String, String> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        self.report
            .write((self.name, self.source), &mut cursor)
            .map_err(|e| format!("Failed to write error to string: {e:?}"))?;
        String::from_utf8(cursor.into_inner())
            .map_err(|e| format!("Invalid UTF-8 string: {e:?}"))
    }
}

// Copyright 2023 Salesforce, Inc. All rights reserved.
use pel::{parser::ParsingError, runtime::RuntimeError, Location};

#[derive(thiserror::Error, Debug)]
pub enum ExpressionError {
    #[error("Already resolved")]
    AlreadyResolved,

    #[error("Incomplete evaluation")]
    IncompleteEvaluation,

    #[error("Parsing error: {0}")]
    ParsingError(ParsingError),

    #[error("Runtime error: {0}")]
    RuntimeError(RuntimeError),

    #[error("{0}")]
    DetailedRuntimeError(DetailedRuntimeError),
}

impl ExpressionError {
    pub(crate) fn with_optional_source(cause: RuntimeError, source: Option<&str>) -> Self {
        if let Some(source) = source {
            Self::DetailedRuntimeError(DetailedRuntimeError::new(source, cause))
        } else {
            Self::RuntimeError(cause)
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Span {
    line: usize,
    column: usize,
}

impl Span {
    fn new(source: &str, index: usize) -> Self {
        assert!(source.len() > index, "source len is lower than index");

        let mut line = 1usize;
        let mut column = 1usize;

        for (pos, c) in source.chars().enumerate() {
            if pos == index {
                break;
            }

            if c == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }

        Self { line, column }
    }
}

#[derive(Debug)]
pub struct Snippet {
    span: Span,
    text: String,
}

impl Snippet {
    fn new(source: &str, location: Location) -> Self {
        Self {
            span: Span::new(source, location.start),
            text: source[location.start..location.end].to_string(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub struct DetailedRuntimeError {
    snippet: Snippet,
    cause: RuntimeError,
}

impl DetailedRuntimeError {
    fn new(source: &str, cause: RuntimeError) -> Self {
        Self {
            snippet: Snippet::new(source, cause.location()),
            cause,
        }
    }
}

impl std::fmt::Display for DetailedRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Runtime error:")?;
        writeln!(f, "\t{}", self.cause)?;
        writeln!(f, "Location:")?;
        writeln!(
            f,
            "\tline: {}, column: {}",
            self.snippet.span.line, self.snippet.span.column
        )?;
        writeln!(f, "{}| {}", self.snippet.span.line, self.snippet.text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_span() {
        let source = "first line\nsecond line\nthird line";

        let span = Span::new(source, 0);
        assert_eq!(span.line, 1);
        assert_eq!(span.column, 1);

        let span = Span::new(source, 9);
        assert_eq!(span.line, 1);
        assert_eq!(span.column, 10);

        let span = Span::new(source, 10);
        assert_eq!(span.line, 1);
        assert_eq!(span.column, 11);

        // new line character is located at column zero (not printable)
        let span = Span::new(source, 11);
        assert_eq!(span.line, 2);
        assert_eq!(span.column, 0);

        let span = Span::new(source, 12);
        assert_eq!(span.line, 2);
        assert_eq!(span.column, 1);

        let span = Span::new(source, 22);
        assert_eq!(span.line, 2);
        assert_eq!(span.column, 11);

        // new line character is located at column zero (not printable)
        let span = Span::new(source, 23);
        assert_eq!(span.line, 3);
        assert_eq!(span.column, 0);

        let span = Span::new(source, 24);
        assert_eq!(span.line, 3);
        assert_eq!(span.column, 1);
    }

    #[test]
    fn new_snippet() {
        let source = r#"
        (payload.message 
            + 
                (
                    1 * 'hello'
                )
        )
        "#;

        let snippet = Snippet::new(source, Location::new(80, 91));

        assert_eq!(snippet.text, "1 * 'hello'");
        assert_eq!(snippet.span, Span::new(source, 80));
    }

    #[test]
    fn display_detailed_error() {
        let location = Location::new(80, 91);
        let source = r#"
        (payload.message 
            + 
                (
                    1 * 'hello'
                )
        )
        "#;

        let cause = RuntimeError::new(location, pel::runtime::RuntimeErrorKind::TypeMismatch);
        let pel_error = ExpressionError::with_optional_source(cause, Some(source));

        let actual = format!("{pel_error}");
        let expected =
            "Runtime error:\n\tType mismatch\nLocation:\n\tline: 5, column: 20\n5| 1 * 'hello'\n";
        assert_eq!(actual, expected);
    }
}

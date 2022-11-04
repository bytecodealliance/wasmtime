//! miette-specific trait implementations. This is kept separate so
//! that we can have a very lightweight build of the ISLE compiler as
//! part of the Cranelift build process without pulling in any
//! dependencies.

use crate::error::{Error, Source, Span};
use miette::{SourceCode, SourceSpan};

impl From<Span> for SourceSpan {
    fn from(span: Span) -> Self {
        SourceSpan::new(span.from.offset.into(), span.to.offset.into())
    }
}

impl SourceCode for Source {
    fn read_span<'a>(
        &'a self,
        span: &SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> std::result::Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        let contents = self
            .text
            .read_span(span, context_lines_before, context_lines_after)?;
        Ok(Box::new(miette::MietteSpanContents::new_named(
            self.name.to_string(),
            contents.data(),
            contents.span().clone(),
            contents.line(),
            contents.column(),
            contents.line_count(),
        )))
    }
}

impl miette::Diagnostic for Error {
    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        match self {
            Self::ParseError { msg, span, .. }
            | Self::TypeError { msg, span, .. }
            | Self::UnmatchableError { msg, span, .. } => Some(Box::new(std::iter::once(
                miette::LabeledSpan::new_with_span(Some(msg.clone()), span.clone()),
            ))),
            Self::ShadowedError { shadowed, .. } => Some(Box::new(std::iter::once(
                miette::LabeledSpan::new_with_span(
                    Some("rule shadowed by more general higher-priority rule".to_string()),
                    shadowed.1.clone(),
                ),
            ))),
            _ => None,
        }
    }
    fn source_code(&self) -> std::option::Option<&dyn miette::SourceCode> {
        match self {
            Self::ParseError { src, .. }
            | Self::TypeError { src, .. }
            | Self::UnmatchableError { src, .. } => Some(src),
            Self::ShadowedError { shadowed, .. } => Some(&shadowed.0),
            _ => None,
        }
    }
    fn related(&self) -> Option<Box<dyn Iterator<Item = &dyn miette::Diagnostic> + '_>> {
        match self {
            Self::Errors(errors) => Some(Box::new(
                errors.iter().map(|x| x as &dyn miette::Diagnostic),
            )),
            _ => None,
        }
    }
}

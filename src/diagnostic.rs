// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::{
    Category, Document, FormatError, InteropError, LexerError, Limits, Position, Reader,
    ReaderError, ValidatorError,
};
use core::ops::Range;
use thiserror::Error;

fn offset(byte: usize) -> Option<Position> {
    Some(Position {
        offset: Some(u64::try_from(byte).ok()?),
        line: None,
        column: None,
    })
}

/// A reader error together with the source span supplied by the streaming parser.
#[derive(Debug, Error)]
#[error("{error}")]
pub struct LocatedError {
    /// The input-independent diagnosis.
    #[source]
    pub error: ReaderError,
    /// Half-open byte span. An EOF span can be empty.
    pub span: Range<usize>,
}

impl From<(ReaderError, Range<usize>)> for LocatedError {
    fn from((error, span): (ReaderError, Range<usize>)) -> Self {
        Self { error, span }
    }
}
impl FormatError for LocatedError {
    fn category(&self) -> Category {
        self.error.category()
    }
    fn position(&self) -> Option<Position> {
        self.error.position().or_else(|| offset(self.span.start))
    }
}
impl FormatError for LexerError {
    fn category(&self) -> Category {
        match self {
            #[cfg(feature = "std")]
            Self::Io(_) => Category::Io,
            #[cfg(not(feature = "std"))]
            Self::Io(()) => Category::Io,
            Self::UnexpectedEof(_) => Category::Eof,
            Self::DocumentLimit(_)
            | Self::TokenLimit(_)
            | Self::DepthLimit(_)
            | Self::CommentDepth(_)
            | Self::ExpansionLimit(_) => Category::Limit,
            _ => Category::Syntax,
        }
    }
    fn position(&self) -> Option<Position> {
        match *self {
            Self::InvalidUtf8(n)
            | Self::InvalidCharacter(n)
            | Self::UnexpectedEof(n)
            | Self::BadEscline(n)
            | Self::InvalidEscape(n)
            | Self::BadUnicodeEscape(n)
            | Self::UnexpectedStringNewline(n)
            | Self::BadEndString(n)
            | Self::BadIndent(Some(n))
            | Self::DocumentLimit(n)
            | Self::TokenLimit(n)
            | Self::DepthLimit(n)
            | Self::CommentDepth(n)
            | Self::ExpansionLimit(n) => offset(n),
            _ => None,
        }
    }
}
impl FormatError for ReaderError {
    fn category(&self) -> Category {
        match self {
            Self::Lexer(e) => e.category(),
            Self::DepthLimit(_) => Category::Limit,
            Self::UnexpectedEof | Self::UnclosedOpen => Category::Eof,
            _ => Category::Syntax,
        }
    }
    fn position(&self) -> Option<Position> {
        match self {
            Self::Lexer(e) => e.position(),
            Self::DepthLimit(n) => offset(*n),
            _ => None,
        }
    }
}
impl FormatError for ValidatorError {
    fn category(&self) -> Category {
        Category::Data
    }
    fn position(&self) -> Option<Position> {
        None
    }
}
impl FormatError for InteropError {
    fn category(&self) -> Category {
        Category::Data
    }
    fn position(&self) -> Option<Position> {
        None
    }
}

// Owned trees and the adapter conversions recursively traverse and drop nodes.
// The event reader has an explicit stack and can use a caller's higher depth.
pub(crate) fn tree_limits(mut limits: Limits) -> Limits {
    limits.max_depth = limits.max_depth.min(128);
    limits
}

impl Document {
    /// Parse a single KDL document with inclusive resource bounds.
    ///
    /// # Errors
    /// Returns the first syntax, input or resource error with a byte position.
    pub fn parse_with_limits(bytes: &[u8], limits: Limits) -> Result<Self, LocatedError> {
        if let Ok(end) = usize::try_from(limits.max_document_bytes)
            && bytes.len() > end
        {
            return Err((ReaderError::Lexer(LexerError::DocumentLimit(end)), end..end).into());
        }
        Ok(Reader::with_limits(bytes, tree_limits(limits)).collect::<Result<Self, _>>()?)
    }
}

/// Safe category and coordinates captured from an adapter's upstream error.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
#[error("format adapter reported a {category:?} error")]
pub struct AdapterDiagnostic {
    category: Category,
    position: Option<Position>,
}
#[cfg(any(
    feature = "json-interop",
    feature = "toml-interop",
    feature = "yaml-interop"
))]
impl AdapterDiagnostic {
    pub(crate) fn capture(error: &impl FormatError) -> Self {
        Self {
            category: error.category(),
            position: error.position(),
        }
    }
}
impl FormatError for AdapterDiagnostic {
    fn category(&self) -> Category {
        self.category
    }
    fn position(&self) -> Option<Position> {
        self.position
    }
}

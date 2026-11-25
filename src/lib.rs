// SPDX-License-Identifier: MIT OR Apache-2.0
#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
//! `rs-kdl` is a small pure-Rust KDL v2 parser and writer with a streaming reader,
//! a DOM/document model, a validator, and a formatter-style writer.
//!
//! The crate is optimized for parse/write workflows rather than
//! formatting-preserving document editing. That makes it a good fit for config
//! loading, transformation pipelines, and future format interop layers.
//!
//! # Examples
//!
//! Parse a full document:
//! ```
//! use rs_kdl::{Document, Reader};
//!
//! let source: &[u8] = b"app \"demo\" enabled=#true";
//! let document = Reader::new(source)
//!     .collect::<Result<Document, _>>()
//!     .expect("valid KDL");
//!
//! assert_eq!(document.nodes.len(), 1);
//! assert_eq!(document.nodes[0].name(), "app");
//! ```
//!
//! Stream events directly:
//! ```
//! use rs_kdl::{Event, Reader};
//!
//! let source: &[u8] = b"node value";
//! let events = Reader::new(source)
//!     .collect::<Result<Vec<_>, _>>()
//!     .expect("valid KDL");
//!
//! assert!(matches!(events[0].0, Event::Node { .. }));
//! ```
//!
//! Write a document back out:
//! ```
//! use rs_kdl::{Document, Reader, Writer};
//!
//! let source: &[u8] = b"root { child }";
//! let document = Reader::new(source)
//!     .collect::<Result<Document, _>>()
//!     .expect("valid KDL");
//!
//! let mut output = String::new();
//! let mut writer = Writer::new(&mut output);
//! for event in &document {
//!     writer.push(&event).expect("write succeeds");
//! }
//!
//! assert_eq!(output, "root {\n    child\n}");
//! ```
//!
//! Validate an event stream:
//! ```
//! use rs_kdl::{Reader, Validator};
//!
//! let mut validator = Validator::new();
//! let source: &[u8] = b"node key=\"value\"";
//! for event in Reader::new(source) {
//!     let (event, _) = event.expect("valid KDL");
//!     validator.push(&event).expect("valid event stream");
//! }
//! validator.done().expect("closed document");
//! ```
//!
//! [KDL]: <https://kdl.dev>

extern crate alloc;

use core::fmt;

pub mod dom;
pub mod interop;
pub mod lexer;
pub mod reader;
mod ssb2;
pub mod validator;
pub mod writer;

pub use dom::{Document, Entry, Event, Node, Value};
#[cfg(feature = "json-interop")]
#[cfg_attr(docsrs, doc(cfg(feature = "json-interop")))]
pub use interop::JsonInteropError;
#[cfg(feature = "toml-interop")]
#[cfg_attr(docsrs, doc(cfg(feature = "toml-interop")))]
pub use interop::TomlInteropError;
#[cfg(feature = "yaml-interop")]
#[cfg_attr(docsrs, doc(cfg(feature = "yaml-interop")))]
pub use interop::YamlInteropError;
pub use interop::{
    InteropDocument, InteropError, InteropField, InteropMapEntry, InteropNode, InteropNumber,
    InteropProperty, InteropRoot, InteropTemporal, InteropTemporalKind, InteropTypedValue,
    InteropValue,
};
#[cfg(feature = "std")]
pub use lexer::ReadInput;
pub use lexer::{Input, Lexer, LexerError, Token};
pub use reader::{Reader, ReaderError, Spanned};
pub use validator::{Validator, ValidatorError};
#[cfg(feature = "std")]
pub use writer::WriteOutput;
pub use writer::Writer;

/// if an ident can be misinterpreted as a number
fn is_ambiguous_ident(text: &str) -> bool {
    let text = text.strip_prefix(['+', '-']).unwrap_or(text);
    let text = text.strip_prefix('.').unwrap_or(text);
    matches!(text.chars().next(), Some('0'..='9'))
}

/// kdl-compatible text formatting
struct IdentDisplay<'text>(&'text str);
impl fmt::Display for IdentDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = self.0;
        if text.is_empty()
            || is_ambiguous_ident(text)
            || text.contains([
                '\u{0}', '\u{1}', '\u{2}', '\u{3}', '\u{4}', '\u{5}', '\u{6}', '\u{7}', '\u{8}',
                '\u{E}', '\u{F}', '\u{10}', '\u{11}', '\u{12}', '\u{13}', '\u{14}', '\u{15}',
                '\u{16}', '\u{17}', '\u{18}', '\u{19}', '\u{1A}', '\u{1B}', '\u{1C}', '\u{1D}',
                '\u{1E}', '\u{1F}', '\u{7F}', '\u{200E}', '\u{200F}', '\u{202A}', '\u{202B}',
                '\u{202C}', '\u{202D}', '\u{202E}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
                '\u{FEFF}', '\\', '/', '(', ')', '{', '}', ';', '[', ']', '"', '#', '=', '\u{9}',
                '\u{20}', '\u{A0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}',
                '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}',
                '\u{202F}', '\u{205F}', '\u{3000}', '\u{A}', '\u{B}', '\u{C}', '\u{D}', '\u{85}',
                '\u{2028}', '\u{2029}',
            ])
        {
            f.write_str("\"")?;
            for ch in text.chars() {
                match ch {
                    '\u{8}' => f.write_str("\\b"),
                    '\u{C}' => f.write_str("\\f"),
                    '\'' => f.write_str("'"),
                    _ => fmt::Display::fmt(&ch.escape_debug(), f),
                }?;
            }
            f.write_str("\"")
        } else {
            fmt::Display::fmt(&text, f)
        }
    }
}

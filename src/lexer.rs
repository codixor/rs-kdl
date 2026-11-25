// SPDX-License-Identifier: MIT OR Apache-2.0
//! Read raw tokens out of a file.
//!
//! Possibly useful if you want to implement syntax highlighting.
//!
//! You probably want to start at [`Lexer`].

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::iter::repeat_n;
use core::mem::discriminant;
use core::num::NonZeroUsize;
use core::ops::Range;

use rs_format::Limits;
use smol_str::SmolStr;
use thiserror::Error;

use crate::dom::Number;
use crate::dom::number::NumberBuilder;
use crate::ssb2::SmolStrBuilder2;

/// A successful token of text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    /// `\u{FEFF}` at position 0 (can only be the first token).
    Bom,
    /// End of file.
    Eof,
    /// Some vertical gap.
    Lines,
    /// Some horizontal gap.
    Spaces,
    /// Any textual value.
    String(SmolStr),
    /// A numeric value, including `#inf`, `#-inf`, and `#-nan`.
    Number(Number),
    /// `String` without any value.
    SkippedString,
    /// `Number` without any value.
    SkippedNumber,
    /// `/-`
    SlashDash,
    /// `;`
    SemiColon,
    /// `=`
    Equals,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `{`
    OpenCurly,
    /// `}`
    CloseCurly,
    /// `#true` or `#false`
    Bool(bool),
    /// `#null`
    Null,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Bom => f.write_str("byte order mark"),
            Token::Eof => f.write_str("end of file"),
            Token::Lines => f.write_str("'\\n'"),
            Token::Spaces => f.write_str("' '"),
            Token::String(value) => fmt::Debug::fmt(value, f),
            Token::Number(value) => fmt::Display::fmt(value, f),
            Token::SkippedString => f.write_str("a string"),
            Token::SkippedNumber => f.write_str("a number"),
            Token::SlashDash => f.write_str("'/-'"),
            Token::SemiColon => f.write_str("';'"),
            Token::Equals => f.write_str("'='"),
            Token::OpenParen => f.write_str("'('"),
            Token::CloseParen => f.write_str("')'"),
            Token::OpenCurly => f.write_str("'{'"),
            Token::CloseCurly => f.write_str("'}'"),
            &Token::Bool(value) => f.write_str(if value { "#true" } else { "#false" }),
            Token::Null => f.write_str("#null"),
        }
    }
}

// error terminology
// invalid = not one of many possible choices (branches)
// unexpected = valid in other places but not here
// missing = opposite of unexpected (not expected!)
// bad = not one of possible options (for one thing)

/// An error while lexing.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LexerError {
    #[cfg(feature = "std")]
    #[expect(clippy::absolute_paths, reason = "feature-gated")]
    #[error("{0}")]
    Io(std::io::ErrorKind),
    #[cfg(not(feature = "std"))]
    #[error("io error")]
    Io(()),
    #[error("Invalid UTF-8 text at {0}")]
    InvalidUtf8(usize),
    #[error("invalid document character at {0}")]
    InvalidCharacter(usize),
    #[error("Unexpected end-of-file at {0}")]
    UnexpectedEof(usize),
    #[error("Bad escline body at {0}")]
    BadEscline(usize),
    #[error("Unexpected plain keyword")]
    UnexpectedKeyword,
    #[error("Invalid string escape at {0}")]
    InvalidEscape(usize),
    #[error("Invalid number value")]
    InvalidNumber,
    #[error("Bad unicode string escape at {0}")]
    BadUnicodeEscape(usize),
    #[error("Unexpected newline in single-line string at {0}")]
    UnexpectedStringNewline(usize),
    #[error("Bad raw string start")]
    BadRawString,
    #[error("Missing newline after multi-line string start")]
    MissingStringNewline,
    #[error("Text before multi-line string end at {0}")]
    BadEndString(usize),
    #[error("Bad multi-line string indent at {0:?}")]
    BadIndent(Option<usize>),
    #[error("Invalid operator")]
    InvalidOperator,
    #[error("Missing expected text")]
    MissingText,
    #[error("Block comment nesting limit exceeded at {0}")]
    CommentDepth(usize),
    #[error("Document byte limit exceeded at {0}")]
    DocumentLimit(usize),
    #[error("Token byte limit exceeded at {0}")]
    TokenLimit(usize),
    #[error("Nested comment limit exceeded at {0}")]
    DepthLimit(usize),
    #[error("Slashdash limit exceeded at {0}")]
    ExpansionLimit(usize),
}

#[cfg(feature = "std")]
impl From<std::io::Error> for LexerError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.kind())
    }
}

/// Don't trust this impl :)
impl PartialEq for LexerError {
    fn eq(&self, other: &Self) -> bool {
        discriminant(self) == discriminant(other)
    }
}

type LexerResult<T> = Result<T, LexerError>;

// stored state between lexer calls
#[derive(Debug, Clone, Copy)]
enum NextSkip {
    None,
    Spaces,
    Lines,
    RecoverLineComment,
    RecoverBlockComment(usize),
    RecoverString {
        multiline: bool,
        hashes: Option<NonZeroUsize>,
    },
    /// An IO error was encountered, which could infinitely repeat without
    /// advancing the cursor
    IrrecoverableError,
}

/// Abstract lexer input trait, essentially [`BufRead`] with better ergonomics.
///
/// Notably implemented for <code>&\[[u8]\]</code> and [`ReadInput`].
///
/// [`BufRead`]: std::io::BufRead
pub trait Input {
    /// Peek at least `n` bytes, any less means end-of-file,
    /// Must work for `n` in `1..=char::MAX_LEN_UTF8`.
    ///
    /// # Errors
    /// On IO error.
    fn peek(&mut self, n: usize) -> LexerResult<&[u8]>;
    /// Advance n bytes, always called after at least `n` peek.
    fn advance(&mut self, n: usize);
}

impl Input for &[u8] {
    fn peek(&mut self, _n: usize) -> LexerResult<&[u8]> {
        Ok(self)
    }
    fn advance(&mut self, n: usize) {
        *self = &self[n..];
    }
}

#[cfg(feature = "std")]
const MAX_PEEK: usize = char::MAX_LEN_UTF8;

/// Input from a [`std::io::Read`].
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[derive(Debug)]
pub struct ReadInput<T> {
    reader: T,
    // TODO/perf: able to use BufRead directly somehow?
    buffer: [u8; MAX_PEEK],
    buffer_len: u8,
}
#[cfg(feature = "std")]
impl<T> ReadInput<T> {
    /// Create a new instance.
    pub fn new(reader: T) -> Self {
        Self {
            reader,
            buffer: [0; MAX_PEEK],
            buffer_len: 0,
        }
    }
}

#[cfg(feature = "std")]
#[expect(clippy::absolute_paths, reason = "feature-gated")]
#[expect(clippy::panic_in_result_fn, reason = "precondition validation")]
#[expect(
    clippy::cast_possible_truncation,
    reason = "start <= request <= MAX_PEEK"
)]
impl<T: std::io::Read> Input for ReadInput<T> {
    fn peek(&mut self, request: usize) -> LexerResult<&[u8]> {
        assert!(request <= MAX_PEEK, "target length too long");
        // manual impl of Read::read_exact, to correctly handle EOF
        let mut start = usize::from(self.buffer_len);
        while start < request {
            // allow reading past requested length, that data will be kept after advance
            match self.reader.read(&mut self.buffer[start..]) {
                Ok(0) => break,
                Ok(n) => start += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e.into()),
            }
        }
        self.buffer_len = start as u8;
        Ok(&self.buffer[..start])
    }
    fn advance(&mut self, request: usize) {
        assert!(
            request <= usize::from(self.buffer_len),
            "target length larger than buffer"
        );
        self.buffer =
            (u32::from_le_bytes(self.buffer).unbounded_shr(8 * request as u32)).to_le_bytes();
        self.buffer_len -= request as u8;
    }
}

/// return a matcher for a `&[u8; utf8]`
// use `printf '\u____' | xxd` to calculate these :)
macro_rules! utf8_class {
	// FEFF
	(bom) => {[0xEF, 0xBB, 0xBF]};
	// TODO/perf: consider including invalid utf8 encodings / surrogates
	(invalid) => {
		// 0..=8, E..=1F, 7F
		[0x00..=0x08 | 0x0E..=0x1F | 0x7F]
		// 200E, 200F, 202A..=202E
		| [0xE2, 0x80, 0x8E | 0x8F | 0xAA..=0xAE]
		// 2066..=2069
		| [0xE2, 0x81, 0xA6..=0xA9]
		// D800..=DFFF
		| [0xED, 0xA0..=0xBF, _]
		| utf8_class!(bom)
	};
	(line) => {
		// A..=D
		[0x0A..=0x0D]
		// 85
		| [0xC2, 0x85]
		// 2028, 2029
		| [0xE2, 0x80, 0xA8 | 0xA9]
	};
	(space) => {
		// 9, 20
		[0x09 | 0x20]
		// A0
		| [0xC2, 0xA0]
		// 1680
		| [0xE1, 0x9A, 0x80]
		// 2000..=200A, 202F
		| [0xE2, 0x80, 0x80..=0x8A | 0xAF]
		// 205F
		| [0xE2, 0x81, 0x9F]
		// 3000
		| [0xE3, 0x80, 0x80]
	};
	// invalid, line, space, \/(){}[];"#=
	(not_ident) => {
		utf8_class!(invalid)
		| utf8_class!(line)
		| utf8_class!(space)
		| b"/" | b"\\" | b"(" | b")" | b"{" | b"}" | b"[" | b"]" | b";" | b"\"" | b"#" | b"="
	};
}

fn utf8_len(first: u8) -> usize {
    // TODO/perf: compare with leading_ones-based solution?
    const LUT: [u8; 16] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 3, 4];
    LUT[(first >> 4) as usize] as usize
}

// Called with a single scalar from peek_table(utf8_len). Copy it before advancing
// the input so allocation can happen only after the byte budget accepts it.
fn codepoint(bytes: &[u8]) -> Option<char> {
    str::from_utf8(bytes).ok()?.chars().next()
}

/// Generic "buffer" to abstract over allocation type
// TODO/perf: try dynamic dispatching this instead of mono
pub(crate) trait StringOutput {
    fn so_push_str(&mut self, text: &str);
    fn so_push_char(&mut self, c: char);
    fn so_push_close(&mut self, hashes: usize);
    fn so_finish(self) -> Token;
    fn so_finish_num(self, first: u8) -> Option<SmolStr>;
}
//impl RawStringOutput for Vec<u8> {}
impl StringOutput for SmolStrBuilder2 {
    fn so_push_str(&mut self, text: &str) {
        self.push_str(text);
    }
    fn so_push_char(&mut self, c: char) {
        self.push(c);
    }
    fn so_push_close(&mut self, hashes: usize) {
        self.push_str("\"");
        self.push_repeated(b'#', hashes);
    }
    fn so_finish(self) -> Token {
        Token::String(self.finish())
    }
    fn so_finish_num(mut self, first: u8) -> Option<SmolStr> {
        self.swap0(first);
        Some(self.finish())
    }
}
impl StringOutput for String {
    fn so_push_str(&mut self, text: &str) {
        self.push_str(text);
    }
    fn so_push_char(&mut self, c: char) {
        self.push(c);
    }
    fn so_push_close(&mut self, hashes: usize) {
        self.push('"');
        self.extend(repeat_n('#', hashes));
    }
    fn so_finish(self) -> Token {
        Token::String(self.into())
    }
    fn so_finish_num(mut self, first: u8) -> Option<SmolStr> {
        let first = char::from(first);
        self.replace_range(..1, first.encode_utf8(&mut [0; 4]));
        Some(self.into())
    }
}
impl StringOutput for () {
    fn so_push_str(&mut self, _text: &str) {}
    fn so_push_char(&mut self, _c: char) {}
    fn so_push_close(&mut self, _hashes: usize) {}
    fn so_finish(self) -> Token {
        Token::SkippedString
    }
    fn so_finish_num(self, _first: u8) -> Option<SmolStr> {
        None
    }
}

/// Lexer to turn a text stream into tokens.
#[derive(Debug)]
pub struct Lexer<T> {
    reader: T,
    limits: Limits,
    token_end: Option<usize>,
    slashdashes: u64,
    cursor: usize,
    next_skip: NextSkip,
}

// TODO/perf: we now only allow utf-8 text again,
// consider rewriting parser to take advantage of that.
// it might not be worth it to use `char`s, but using strings instead of
// byte-strings. this is best explored as a whole-lexer rewrite.

// TODO/perf: attempt to use a pure state machine, for better error recovery
#[expect(clippy::unnested_or_patterns, reason = "does not respect utf8_class")]
impl<T: Input> Lexer<T> {
    /// Create a new lexer from the input.
    pub fn new(input: T) -> Self {
        Self::with_limits(input, Limits::default())
    }
    /// Create a lexer with inclusive per-document limits.
    pub const fn with_limits(input: T, limits: Limits) -> Self {
        Self {
            limits,
            token_end: None,
            slashdashes: 0,
            reader: input,
            cursor: 0,
            next_skip: NextSkip::None,
        }
    }
    pub(crate) const fn limits(&self) -> Limits {
        self.limits
    }

    fn start_body(&mut self) {
        self.token_end = Some(self.cursor.saturating_add(self.limits.max_token_bytes));
    }
    fn check_body(&self, position: usize) -> LexerResult<()> {
        if let Some(end) = self.token_end
            && position > end
        {
            return Err(LexerError::TokenLimit(end));
        }
        Ok(())
    }

    fn peek(&mut self, n: Range<usize>) -> LexerResult<&[u8]> {
        // Several scanners process an entire slice at once. Limit that window
        // before handing it to a builder, retaining four bytes for UTF-8 lookahead.
        let doc_end = usize::try_from(self.limits.max_document_bytes).unwrap_or(usize::MAX);
        let end = doc_end.min(self.token_end.unwrap_or(usize::MAX));
        let window = end
            .saturating_sub(self.cursor)
            .saturating_add(char::MAX_LEN_UTF8);
        match self.reader.peek(n.start) {
            Ok(result) if !result.is_empty() && self.cursor >= doc_end => {
                Err(LexerError::DocumentLimit(doc_end))
            }
            Ok(result) => Ok(&result[..result.len().min(n.end).min(window)]),
            Err(err) => {
                // TODO/perf: i think it's better to have it here (instead of in next_token)
                self.next_skip = NextSkip::IrrecoverableError;
                Err(err)
            }
        }
    }
    // TODO/perf: replace f with some specific solution like a 256-LUT
    fn peek_table(&mut self, f: impl FnOnce(u8) -> usize) -> LexerResult<&[u8]> {
        let &[first] = self.peek(1..1)? else {
            // TODO/perf: return peek result directly?
            return Ok(&[]);
        };
        // fucked up lifetime things means that i need to do 2 peeks :(
        let size = f(first);
        self.peek(size..size)
    }
    fn advance(&mut self, n: usize) -> LexerResult<()> {
        let end = self
            .cursor
            .checked_add(n)
            .ok_or(LexerError::DocumentLimit(usize::MAX))?;
        let document_end = usize::try_from(self.limits.max_document_bytes).unwrap_or(usize::MAX);
        if end > document_end {
            return Err(LexerError::DocumentLimit(document_end));
        }
        self.check_body(end)?;
        self.cursor = end;
        self.reader.advance(n);
        Ok(())
    }
    fn adv_certain(&mut self, text: &[u8]) -> LexerResult<()> {
        debug_assert_eq!(
            self.peek(text.len()..text.len())?,
            text,
            "adv_certain was certainly wrong"
        );
        self.advance(text.len())
    }
    fn adv_uncertain(&mut self, text: &[u8]) -> LexerResult<()> {
        if self.peek(text.len()..text.len())? == text {
            self.advance(text.len())?;
            Ok(())
        } else {
            Err(LexerError::MissingText)
        }
    }
    fn begin_skip(&mut self, size: usize, skip: NextSkip) -> LexerResult<Token> {
        self.advance(size)?;
        self.next_skip = skip;
        Ok(match skip {
            NextSkip::Spaces => Token::Spaces,
            NextSkip::Lines => Token::Lines,
            _ => unreachable!(),
        })
    }
    fn just(&mut self, size: usize, token: Token) -> LexerResult<Token> {
        self.advance(size)?;
        Ok(token)
    }
    // TODO/perf: check that head never appears in release builds
    fn keyword(&mut self, head: &[u8], tail: &[u8], token: Token) -> LexerResult<Token> {
        self.start_body();
        self.adv_certain(head)?;
        self.adv_uncertain(tail)?;
        Ok(token)
    }
    fn keyword_number(
        &mut self,
        head: &[u8],
        tail: &[u8],
        skip: bool,
        number: Number,
    ) -> LexerResult<Token> {
        let token = if skip {
            Token::SkippedNumber
        } else {
            Token::Number(number)
        };
        self.keyword(head, tail, token)
    }
    fn block_comment(&mut self) -> LexerResult<()> {
        self.adv_certain(b"/*")?;
        let start = self.cursor - 2;
        if self.limits.max_depth == 0 {
            return Err(LexerError::DepthLimit(start));
        }
        self.start_body();
        let mut depth = 0_usize;
        loop {
            self.next_skip = NextSkip::RecoverBlockComment(depth);
            let peek = self.peek_table(|first| match first {
                b'/' | b'*' => 2,
                _ => utf8_len(first),
            })?;
            let size = peek.len();
            match peek {
                [] => return Err(LexerError::UnexpectedEof(self.cursor)),
                utf8_class!(invalid) => {
                    return Err(LexerError::InvalidCharacter(self.cursor));
                }
                b"/*" => {
                    self.advance(2)?;
                    if depth
                        >= usize::try_from(self.limits.max_depth.saturating_sub(1))
                            .unwrap_or(usize::MAX)
                    {
                        return Err(LexerError::DepthLimit(self.cursor - 2));
                    }
                    let Some(next) = depth.checked_add(1) else {
                        self.next_skip = NextSkip::IrrecoverableError;
                        return Err(LexerError::CommentDepth(self.cursor - 2));
                    };
                    depth = next;
                }
                b"*/" => {
                    if depth == 0 {
                        self.token_end = None;
                    }
                    self.advance(2)?;
                    match depth.checked_sub(1) {
                        Some(new) => depth = new,
                        None => break,
                    }
                }
                [b'/' | b'*', ..] => self.advance(1)?,
                text if str::from_utf8(text).is_ok() => self.advance(size)?,
                _ => return Err(LexerError::InvalidUtf8(self.cursor)),
            }
        }
        self.next_skip = NextSkip::None;
        Ok(())
    }
    fn line_comment(&mut self) -> LexerResult<()> {
        self.adv_certain(b"//")?;
        self.start_body();
        self.next_skip = NextSkip::RecoverLineComment;
        loop {
            let peek = self.peek_table(utf8_len)?;
            let size = peek.len();
            match peek {
                [] => break,
                utf8_class!(line) => {
                    self.token_end = None;
                    // consume newline
                    self.advance(size)?;
                    break;
                }
                utf8_class!(invalid) => {
                    return Err(LexerError::InvalidCharacter(self.cursor));
                }
                text if str::from_utf8(text).is_ok() => self.advance(size)?,
                _ => return Err(LexerError::InvalidUtf8(self.cursor)),
            }
        }
        self.next_skip = NextSkip::None;
        self.token_end = None;
        Ok(())
    }
    fn escline(&mut self) -> LexerResult<()> {
        self.adv_certain(b"\\")?;
        loop {
            let peek = self.peek_table(|first| match first {
                b'/' => 2,
                _ => utf8_len(first),
            })?;
            let size = peek.len();
            match peek {
                [] => break,
                utf8_class!(space) => self.advance(size)?,
                utf8_class!(line) => break self.advance(size)?,
                b"/*" => self.block_comment()?,
                b"//" => break self.line_comment()?,
                // /_ fallthrough
                _ => return Err(LexerError::BadEscline(self.cursor)),
            }
        }
        Ok(())
    }
    // advances the entire escape, including the \
    fn string_escape(&mut self) -> LexerResult<Option<char>> {
        let start_of_escape = self.cursor;
        self.adv_certain(b"\\")?;
        let peek = self.peek_table(utf8_len)?;
        let size = peek.len();
        let ch = match peek {
            utf8_class!(space) | utf8_class!(line) => {
                let mut size = size;
                loop {
                    self.advance(size)?;
                    let peek_space = self.peek_table(utf8_len)?;
                    size = peek_space.len();
                    if !matches!(peek_space, utf8_class!(space) | utf8_class!(line)) {
                        return Ok(None);
                    }
                }
            }
            b"\"" => '\"',
            b"\\" => '\\',
            b"b" => '\x08',
            b"f" => '\x0C',
            b"n" => '\n',
            b"r" => '\r',
            b"t" => '\t',
            b"s" => ' ',
            b"u" => {
                #[expect(
                    clippy::cast_possible_wrap,
                    clippy::cast_sign_loss,
                    reason = "cursed anyways"
                )]
                /// thanks "needsmoreestrogen" from RPLCS
                fn hex(v: u8) -> u32 {
                    ((((v + v) as i8 >> 7) & 9) as u8 + (v & 15)).into()
                }
                macro_rules! hex { ($id:ident) => {$id @ (b'0'..=b'9' | b'A'..=b'F' | b'a'..=b'f')} }
                self.advance(1)?;
                self.adv_uncertain(b"{")?;
                // TODO/style: improve how this looks? This uses fixed width peeks
                // but it could be possible with variable width ones
                // {^0}"
                let value = match *self.peek(3..3)? {
                    [hex!(c2), hex!(c1), hex!(c0)] => {
                        self.advance(3)?;
                        let base = hex(c2) << 8 | hex(c1) << 4 | hex(c0);
                        // {000^}"
                        match *self.peek(2..2)? {
                            [hex!(c1), hex!(c0)] => {
                                let base = base << 8 | hex(c1) << 4 | hex(c0);
                                self.advance(2)?;
                                // {00000^}"
                                match *self.peek(2..2)? {
                                    [hex!(c0), b'}'] => {
                                        self.advance(2)?;
                                        base << 4 | hex(c0)
                                    }
                                    [b'}', ..] => {
                                        self.advance(1)?;
                                        base
                                    }
                                    _ => return Err(LexerError::BadUnicodeEscape(start_of_escape)),
                                }
                            }
                            [hex!(c0), b'}'] => {
                                self.advance(2)?;
                                base << 4 | hex(c0)
                            }
                            [b'}', ..] => {
                                self.advance(1)?;
                                base
                            }
                            _ => return Err(LexerError::BadUnicodeEscape(start_of_escape)),
                        }
                    }
                    [hex!(c1), hex!(c0), b'}'] => {
                        self.advance(3)?;
                        hex(c1) << 4 | hex(c0)
                    }
                    [hex!(c0), b'}', ..] => {
                        self.advance(2)?;
                        hex(c0)
                    }
                    _ => return Err(LexerError::BadUnicodeEscape(start_of_escape)),
                };
                return Ok(Some(
                    char::from_u32(value).ok_or(LexerError::BadUnicodeEscape(start_of_escape))?,
                ));
            }
            _ => return Err(LexerError::InvalidEscape(start_of_escape)),
        };
        self.advance(size)?;
        Ok(Some(ch))
    }
    fn spaces(&mut self) -> LexerResult<()> {
        loop {
            let peek = self.peek_table(|first| match first {
                b'/' => 2,
                _ => utf8_len(first),
            })?;
            let size = peek.len();
            match peek {
                [] => break,
                utf8_class!(space) => self.advance(size)?,
                b"\\" => self.escline()?,
                b"/*" => self.block_comment()?,
                // /_ fallthrough
                _ => break,
            }
        }
        Ok(())
    }
    fn lines(&mut self) -> LexerResult<()> {
        loop {
            let peek = self.peek_table(|first| match first {
                b'/' => 2,
                _ => utf8_len(first),
            })?;
            let size = peek.len();
            match peek {
                [] => break,
                utf8_class!(space) | utf8_class!(line) => self.advance(size)?,
                b"\\" => self.escline()?,
                b"/*" => self.block_comment()?,
                b"//" => self.line_comment()?,
                // /_ fallthrough
                _ => break,
            }
        }
        Ok(())
    }
    fn ident_inner(&mut self, number: bool, mut text: impl StringOutput) -> LexerResult<Token> {
        if number {
            let mut builder = NumberBuilder::new(text);
            'text: loop {
                let cursor = self.cursor;
                let token_end = self.token_end.unwrap_or(usize::MAX);
                let document_end =
                    usize::try_from(self.limits.max_document_bytes).unwrap_or(usize::MAX);
                let peek = self.peek(1..usize::MAX)?;
                if peek.is_empty() {
                    break 'text;
                }
                for (i, &byte) in peek.iter().enumerate() {
                    // subset of valid identifiers that could be valid numeric characters
                    // TODO/perf: compare codegen with making these ranges more specific
                    // or use a u128 lookup table :)
                    if !matches!(byte, b'+'..=b'9' | b'A'..=b'Z' | b'_' | b'a'..=b'z') {
                        self.advance(i)?;
                        break 'text;
                    }
                    if i >= token_end.saturating_sub(cursor) {
                        return Err(LexerError::TokenLimit(token_end));
                    }
                    if i >= document_end.saturating_sub(cursor) {
                        return Err(LexerError::DocumentLimit(document_end));
                    }
                    if !builder.step(byte) {
                        self.advance(i)?;
                        return Err(LexerError::InvalidNumber);
                    }
                }
                let size = peek.len();
                self.advance(size)?;
            }
            if matches!(self.peek_table(utf8_len)?, [] | utf8_class!(not_ident)) {
                match builder.finish() {
                    Some(Some(value)) => Ok(Token::Number(value)),
                    Some(None) => Ok(Token::SkippedNumber),
                    None => Err(LexerError::InvalidNumber),
                }
            } else {
                // number didn't consume the entire ident (e.g. high-unicode tail)
                Err(LexerError::InvalidNumber)
            }
        } else {
            let debug_start = self.cursor;
            loop {
                let cursor = self.cursor;
                let peek = self.peek_table(utf8_len)?;
                let size = peek.len();
                match peek {
                    [] | utf8_class!(not_ident) => {
                        debug_assert_ne!(debug_start, cursor, "empty ident!");
                        break;
                    }
                    ch => {
                        if let Some(ch) = codepoint(ch) {
                            self.advance(size)?;
                            text.so_push_char(ch);
                        } else {
                            self.advance(1)?;
                            return Err(LexerError::InvalidUtf8(cursor));
                        }
                    }
                }
            }
            Ok(text.so_finish())
        }
    }
    fn ident(&mut self, skip: bool) -> LexerResult<Token> {
        self.start_body();
        #[derive(Clone, Copy, PartialEq)]
        enum Preview {
            Regular,
            Number,
            Keyword,
        }
        // check for number-like value or bad keyword
        let preview = match self.peek(2..2)? {
            [b'0'..=b'9', ..] | [b'+' | b'-', b'0'..=b'9'] => Preview::Number,
            [b'.', b'0'..=b'9'] => {
                self.advance(1)?;
                return Err(LexerError::InvalidNumber);
            }
            // peek further to see if it's an error (since both bytes would now be known)
            [b'+' | b'-', b'.'] => match self.peek(3..3)? {
                [b'+' | b'-', b'.', b'0'..=b'9'] => {
                    self.advance(2)?;
                    return Err(LexerError::InvalidNumber);
                }
                _ => Preview::Regular,
            },
            b"tr" | b"fa" | b"nu" | b"in" | b"-i" | b"na" => Preview::Keyword,
            _ => Preview::Regular,
        };
        match (skip, preview) {
            (skip, Preview::Keyword) => {
                // since we need the full text, always allocate for this case
                // it could be done without but it's not worth the complexity
                let Token::String(text) = self.ident_inner(false, SmolStrBuilder2::new())? else {
                    unreachable!()
                };
                if matches!(&*text, "true" | "false" | "null" | "inf" | "-inf" | "nan") {
                    Err(LexerError::UnexpectedKeyword)
                } else if skip {
                    Ok(Token::SkippedString)
                } else {
                    Ok(Token::String(text))
                }
            }
            (true, _) => self.ident_inner(preview == Preview::Number, ()),
            (false, _) => self.ident_inner(preview == Preview::Number, SmolStrBuilder2::new()),
        }
    }
    // A quote followed by too few raw-string hashes belongs to the body.
    // Return the number of consumed hashes when the tentative delimiter fails.
    fn closing_hashes(
        &mut self,
        quotes: usize,
        hashes: Option<NonZeroUsize>,
    ) -> LexerResult<Option<usize>> {
        let end = self.token_end.take();
        self.advance(quotes)?;
        let count = hashes.map_or(0, NonZeroUsize::get);
        let mut consumed = 0;
        while consumed < count {
            let tail = self.peek(1..count - consumed)?;
            if tail.is_empty() {
                return Err(LexerError::UnexpectedEof(self.cursor));
            }
            let matched = tail.iter().take_while(|&&byte| byte == b'#').count();
            let mismatch = matched < tail.len();
            self.advance(matched)?;
            consumed += matched;
            if mismatch {
                self.token_end = end;
                self.check_body(self.cursor)?;
                return Ok(Some(consumed));
            }
        }
        Ok(None)
    }
    fn singleline_string(
        &mut self,
        hashes: Option<NonZeroUsize>,
        mut text: impl StringOutput,
    ) -> LexerResult<Token> {
        'text: loop {
            let cursor = self.cursor;
            let peek = self.peek_table(utf8_len)?;
            let size = peek.len();
            match peek {
                [] => return Err(LexerError::UnexpectedEof(cursor)),
                utf8_class!(invalid) => return Err(LexerError::InvalidCharacter(cursor)),
                utf8_class!(line) => return Err(LexerError::UnexpectedStringNewline(cursor)),
                b"\"" => {
                    if let Some(count) = self.closing_hashes(1, hashes)? {
                        text.so_push_close(count);
                        continue 'text;
                    }

                    self.next_skip = NextSkip::None;
                    break Ok(text.so_finish());
                }
                b"\\" if hashes.is_none() => {
                    if let Some(ch) = self.string_escape()? {
                        text.so_push_char(ch);
                    }
                }
                ch => {
                    let ch = codepoint(ch).ok_or(LexerError::InvalidUtf8(cursor))?;
                    self.advance(size)?;
                    text.so_push_char(ch);
                }
            }
        }
    }
    // return advance distance for newline (if there is any)
    fn newline_crlf(&mut self) -> LexerResult<Option<NonZeroUsize>> {
        let peek = self.peek_table(|first| match first {
            b'\r' => 2,
            first => utf8_len(first),
        })?;
        Ok(NonZeroUsize::new(match peek {
            b"\r\n" => 2,
            [b'\r', ..] => 1,
            utf8_class!(line) => peek.len(),
            _ => 0,
        }))
    }
    fn multiline_string_regular(&mut self, hashes: Option<NonZeroUsize>) -> LexerResult<Token> {
        // buffer of contents, escaped & normalized but not aligned
        let mut full_text = String::new();
        // (line_cursor, line_start, text_start, line_end)
        let mut lines = Vec::<Option<(usize, usize, usize, usize)>>::new();
        let tail = 'line: loop {
            let line_cursor = self.cursor;
            let line_start = full_text.len();
            let mut peek_indent = self.peek_table(utf8_len)?;
            // indent
            while matches!(peek_indent, utf8_class!(space)) {
                let ch = codepoint(peek_indent).expect("known UTF-8 whitespace");
                let size = peek_indent.len();
                self.advance(size)?;
                full_text.push(ch);
                peek_indent = self.peek_table(utf8_len)?;
            }
            let text_start = full_text.len();
            let newline = self.newline_crlf()?;
            if let Some(size) = newline {
                self.advance(size.get())?;
                lines.push(None);
                continue;
            }
            'text: loop {
                let cursor = self.cursor;
                let peek = self.peek_table(|first| match first {
                    b'"' => 3,
                    b'\r' => 2,
                    first => utf8_len(first),
                })?;
                let size = peek.len();
                match peek {
                    [] => return Err(LexerError::UnexpectedEof(cursor)),
                    utf8_class!(invalid) => return Err(LexerError::InvalidCharacter(cursor)),
                    [b'\r', ..] | utf8_class!(line) => {
                        let size = match peek {
                            b"\r\n" => 2,
                            [b'\r', ..] => 1,
                            _ => size,
                        };
                        self.advance(size)?;
                        lines.push(Some((line_cursor, line_start, text_start, full_text.len())));
                        break;
                    }
                    b"\"\"\"" => {
                        if let Some(count) = self.closing_hashes(3, hashes)? {
                            full_text.push_str("\"\"\"");
                            full_text.extend(repeat_n('#', count));
                            continue 'text;
                        }

                        self.next_skip = NextSkip::None;
                        if full_text.len() > text_start {
                            return Err(LexerError::BadEndString(cursor));
                        }
                        break 'line line_start..text_start;
                    }
                    [b'"', ..] => {
                        self.advance(1)?;
                        full_text.push('"');
                    }
                    b"\\" if hashes.is_none() => {
                        if let Some(ch) = self.string_escape()? {
                            full_text.push(ch);
                        }
                    }
                    ch => {
                        let ch = codepoint(ch).ok_or(LexerError::InvalidUtf8(cursor))?;
                        self.advance(size)?;
                        full_text.push(ch);
                    }
                }
            }
        };
        let tail_len = tail.end - tail.start;
        // create final text
        let mut text = SmolStrBuilder2::new();
        let mut pre_newline = false;
        for line in lines {
            if pre_newline {
                text.push('\n');
            }
            pre_newline = true;
            if let Some((line_cursor, line_start, text_start, line_end)) = line {
                if text_start - line_start < tail_len
                    || full_text.as_bytes()[tail.clone()]
                        != full_text.as_bytes()[line_start..line_start + tail_len]
                {
                    return Err(LexerError::BadIndent(Some(line_cursor)));
                }
                text.push_str(&full_text[line_start + tail_len..line_end]);
            }
        }
        Ok(Token::String(text.finish()))
    }
    // TODO/style: merge inner loop with multiline_string_regular
    fn multiline_string_skip(&mut self, hashes: Option<NonZeroUsize>) -> LexerResult<Token> {
        // it's impossible to make a multiline string reader that doesn't allocate
        // instead, notice that for the close quote (and thus the string) to be valid,
        // it must have no body text. by only reading the indent as the smallest common
        // one, this validates indents while only allocating one line
        let mut indent = SmolStrBuilder2::new();
        let mut peek_indent = self.peek_table(utf8_len)?;
        // get first line indent
        while matches!(peek_indent, utf8_class!(space)) {
            let ch = codepoint(peek_indent).expect("known UTF-8 whitespace");
            let size = peek_indent.len();
            self.advance(size)?;
            indent.push(ch);
            peek_indent = self.peek_table(utf8_len)?;
        }
        let mut next_truncate_length = indent.len();
        'line: loop {
            // if there's any leftover indent it's more indented than minimum (or empty)
            let has_leading_space = matches!(self.peek_table(utf8_len)?, utf8_class!(space));
            // line body / end of string
            let mut has_body = false;
            'text: loop {
                let cursor = self.cursor;
                let peek = self.peek_table(|first| match first {
                    b'"' => 3,
                    b'\r' => 2,
                    first => utf8_len(first),
                })?;
                let size = peek.len();
                match peek {
                    [] => return Err(LexerError::UnexpectedEof(cursor)),
                    utf8_class!(invalid) => return Err(LexerError::InvalidCharacter(cursor)),
                    utf8_class!(space) => {
                        // doesn't set body flag
                        self.advance(size)?;
                    }
                    [b'\r', ..] | utf8_class!(line) => {
                        let size = match peek {
                            b"\r\n" => 2,
                            [b'\r', ..] => 1,
                            _ => size,
                        };
                        self.advance(size)?;
                        break;
                    }
                    b"\"\"\"" => {
                        if self.closing_hashes(3, hashes)?.is_some() {
                            has_body = true;
                            continue 'text;
                        }

                        self.next_skip = NextSkip::None;
                        if has_body {
                            return Err(LexerError::BadEndString(cursor));
                        } else if has_leading_space {
                            return Err(LexerError::BadIndent(None));
                        }
                        break 'line;
                    }
                    [b'"', ..] => {
                        has_body = true;
                        self.advance(1)?;
                    }
                    // TODO/perf: duplicate loop with outer check? probably not worth it
                    b"\\" if hashes.is_none() => has_body |= self.string_escape()?.is_some(),
                    ch => {
                        _ = str::from_utf8(ch).map_err(|_| LexerError::InvalidUtf8(cursor))?;
                        has_body = true;
                        self.advance(size)?;
                    }
                }
            }
            // truncate if line isn't empty
            if has_body {
                indent.truncate_floor(next_truncate_length);
            }
            // take longest matching indent
            next_truncate_length = indent.len();
            let mut matched_bytes = 0;
            while matched_bytes < indent.len() {
                let ch = self.peek_table(utf8_len)?;
                if ch.is_empty() {
                    return Err(LexerError::UnexpectedEof(self.cursor));
                }
                if !indent.as_bytes()[matched_bytes..].starts_with(ch) {
                    next_truncate_length = matched_bytes;
                    break;
                }
                let size = ch.len();
                self.advance(size)?;
                matched_bytes += size;
            }
        }
        Ok(Token::SkippedString)
    }
    // all strings entry point
    fn string(&mut self, skip: bool) -> LexerResult<Token> {
        let mut hashes = 0_usize;
        'count: loop {
            let mut advance = 0;
            for &byte in self.peek(1..usize::MAX)? {
                if byte != b'#' {
                    self.advance(advance)?;
                    hashes = hashes.checked_add(advance).unwrap();
                    break 'count;
                }
                advance += 1;
            }
            if advance == 0 {
                return Err(LexerError::UnexpectedEof(self.cursor));
            }
            self.advance(advance)?;
            // if this panics you've got bigger issues
            hashes = hashes.checked_add(advance).unwrap();
        }
        let hashes = NonZeroUsize::new(hashes);
        match self.peek(3..3)? {
            b"\"\"\"" => {
                self.next_skip = NextSkip::RecoverString {
                    multiline: true,
                    hashes,
                };
                self.advance(3)?;
                self.start_body();
                let Some(size) = self.newline_crlf()? else {
                    return Err(LexerError::MissingStringNewline);
                };
                let size = size.get();
                self.advance(size)?;
                if skip {
                    self.multiline_string_skip(hashes)
                } else {
                    self.multiline_string_regular(hashes)
                }
            }
            [b'"', ..] => {
                self.next_skip = NextSkip::RecoverString {
                    multiline: false,
                    hashes,
                };
                self.advance(1)?;
                self.start_body();
                if skip {
                    self.singleline_string(hashes, ())
                } else {
                    self.singleline_string(hashes, SmolStrBuilder2::new())
                }
            }
            _ => Err(LexerError::BadRawString),
        }
    }
    fn advance_err(&mut self, n: usize, err: LexerError) -> LexerError {
        self.advance(n).err().unwrap_or(err)
    }
    fn recover_block(&mut self, mut depth: usize) -> LexerResult<()> {
        loop {
            match self.peek(2..2)? {
                [] => break,
                b"*/" if depth == 0 => {
                    self.token_end = None;
                    self.advance(2)?;
                    break;
                }
                b"*/" => {
                    depth -= 1;
                    self.advance(2)?;
                }
                b"/*" => {
                    if depth
                        >= usize::try_from(self.limits.max_depth.saturating_sub(1))
                            .unwrap_or(usize::MAX)
                    {
                        return Err(LexerError::DepthLimit(self.cursor));
                    }
                    depth += 1;
                    self.advance(2)?;
                }
                _ => self.advance(1)?,
            }
        }
        Ok(())
    }

    fn recover_string(&mut self, multiline: bool, hashes: Option<NonZeroUsize>) -> LexerResult<()> {
        let quotes: &[u8] = if multiline { b"\"\"\"" } else { b"\"" };
        loop {
            let next = self.peek(quotes.len()..quotes.len())?;
            if next.is_empty() {
                break;
            }
            if next == quotes {
                if self.closing_hashes(quotes.len(), hashes)?.is_none() {
                    break;
                }
            } else {
                self.advance(1)?;
            }
        }
        Ok(())
    }
    fn next_token_value(&mut self, skip: bool, out_cursor: &mut usize) -> LexerResult<Token> {
        match self.next_skip {
            NextSkip::None => {}
            NextSkip::Spaces => self.spaces()?,
            NextSkip::Lines => self.lines()?,
            NextSkip::RecoverLineComment => loop {
                let next = self.peek_table(utf8_len)?;
                if next.is_empty() {
                    break;
                }
                if matches!(next, utf8_class!(line)) {
                    let size = next.len();
                    self.token_end = None;
                    self.advance(size)?;
                    break;
                }
                self.advance(1)?;
            },
            NextSkip::RecoverBlockComment(depth) => self.recover_block(depth)?,
            NextSkip::RecoverString { multiline, hashes } => {
                self.recover_string(multiline, hashes)?
            }
            NextSkip::IrrecoverableError => return Ok(Token::Eof),
        }
        self.next_skip = NextSkip::None;
        self.token_end = None;
        let start = self.cursor;
        *out_cursor = start;
        // TODO/perf: ideas for general parsing improvements
        // - splitting match by result length (0/1/2/3/4 bytes)
        // - match keywords with 4 bytes instead of 3 (u32?)
        // - check on byte-slice matching codegen (in general)
        let peek = self.peek_table(|first: u8| match first {
            b'/' => 2,
            b'#' => 3,
            _ => utf8_len(first),
        })?;
        let size = peek.len();
        Ok(match peek {
            [] => Token::Eof,
            utf8_class!(bom) if start == 0 => self.just(3, Token::Bom)?,
            utf8_class!(invalid) => {
                return Err(self.advance_err(size, LexerError::InvalidCharacter(self.cursor)));
            }
            utf8_class!(line) => self.begin_skip(size, NextSkip::Lines)?,
            utf8_class!(space) => self.begin_skip(size, NextSkip::Spaces)?,
            b"\\" => self.begin_skip(0, NextSkip::Spaces)?,
            b";" => self.just(1, Token::SemiColon)?,
            b"=" => self.just(1, Token::Equals)?,
            b"(" => self.just(1, Token::OpenParen)?,
            b")" => self.just(1, Token::CloseParen)?,
            b"{" => self.just(1, Token::OpenCurly)?,
            b"}" => self.just(1, Token::CloseCurly)?,
            b"[" | b"]" => return Err(self.advance_err(1, LexerError::InvalidOperator)),
            // silly trick I'm taking from serde_json
            b"#tr" => self.keyword(b"#tr", b"ue", Token::Bool(true))?,
            b"#fa" => self.keyword(b"#fa", b"lse", Token::Bool(false))?,
            b"#nu" => self.keyword(b"#nu", b"ll", Token::Null)?,
            b"#in" => self.keyword_number(b"#in", b"f", skip, Number::INFINITY)?,
            b"#-i" => self.keyword_number(b"#-i", b"nf", skip, Number::NEG_INFINITY)?,
            b"#na" => self.keyword_number(b"#na", b"n", skip, Number::NAN)?,
            [b'#', ..] | b"\"" => self.string(skip)?,
            b"/-" => {
                if self.slashdashes >= self.limits.max_expansion {
                    return Err(LexerError::ExpansionLimit(start));
                }
                self.slashdashes += 1;
                self.just(2, Token::SlashDash)?
            }
            b"/*" => self.begin_skip(0, NextSkip::Spaces)?,
            b"//" => self.begin_skip(0, NextSkip::Lines)?,
            [b'/', ..] => return Err(self.advance_err(1, LexerError::InvalidOperator)),
            _ => self.ident(skip)?,
        })
    }
    /// Read one token from input, returns value and starting position. This can
    /// safely be resumed after an error, for attempted recovery information.
    pub fn next_token(&mut self, skip: bool) -> (LexerResult<Token>, usize) {
        // TODO/style: this whole mut ref thing is kinda jank
        let mut pos = self.cursor;
        let token = self.next_token_value(skip, &mut pos);
        if token.is_ok() {
            self.token_end = None;
        }
        if matches!(
            token,
            Err(LexerError::DocumentLimit(_)
                | LexerError::DepthLimit(_)
                | LexerError::TokenLimit(_)
                | LexerError::ExpansionLimit(_))
        ) {
            self.next_skip = NextSkip::IrrecoverableError;
        }
        (token, pos)
    }
    /// Get the current position, this is not the same as the next
    /// [`next_token`] position if the last token read was Spaces, Lines, or an
    /// error.
    ///
    /// [`next_token`]: Self::next_token
    pub fn current_position(&mut self) -> usize {
        self.cursor
    }
}

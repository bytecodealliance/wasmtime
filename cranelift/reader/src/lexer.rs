//! Lexical analysis for .clif files.

use crate::error::Location;
use cranelift_codegen::ir::types;
use cranelift_codegen::ir::{Block, Value};
#[allow(unused_imports, deprecated)]
use std::ascii::AsciiExt;
use std::str::CharIndices;
use std::u16;

/// A Token returned from the `Lexer`.
///
/// Some variants may contains references to the original source text, so the `Token` has the same
/// lifetime as the source.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Token<'a> {
    Comment(&'a str),
    LPar,                 // '('
    RPar,                 // ')'
    LBrace,               // '{'
    RBrace,               // '}'
    LBracket,             // '['
    RBracket,             // ']'
    Minus,                // '-'
    Plus,                 // '+'
    Comma,                // ','
    Dot,                  // '.'
    Colon,                // ':'
    Equal,                // '='
    Not,                  // '!'
    Arrow,                // '->'
    Float(&'a str),       // Floating point immediate
    Integer(&'a str),     // Integer immediate
    Type(types::Type),    // i32, f32, b32x4, ...
    Value(Value),         // v12, v7
    Block(Block),         // block3
    StackSlot(u32),       // ss3
    GlobalValue(u32),     // gv3
    Heap(u32),            // heap2
    Table(u32),           // table2
    JumpTable(u32),       // jt2
    Constant(u32),        // const2
    FuncRef(u32),         // fn2
    SigRef(u32),          // sig2
    UserRef(u32),         // u345
    Name(&'a str),        // %9arbitrary_alphanum, %x3, %0, %function ...
    String(&'a str),      // "arbitrary quoted string with no escape" ...
    HexSequence(&'a str), // #89AF
    Identifier(&'a str),  // Unrecognized identifier (opcode, enumerator, ...)
    SourceLoc(&'a str),   // @00c7
}

/// A `Token` with an associated location.
#[derive(Debug, PartialEq, Eq)]
pub struct LocatedToken<'a> {
    pub token: Token<'a>,
    pub location: Location,
}

/// Wrap up a `Token` with the given location.
fn token(token: Token, loc: Location) -> Result<LocatedToken, LocatedError> {
    Ok(LocatedToken {
        token,
        location: loc,
    })
}

/// An error from the lexical analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexError {
    InvalidChar,
}

/// A `LexError` with an associated Location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocatedError {
    pub error: LexError,
    pub location: Location,
}

/// Wrap up a `LexError` with the given location.
fn error<'a>(error: LexError, loc: Location) -> Result<LocatedToken<'a>, LocatedError> {
    Err(LocatedError {
        error,
        location: loc,
    })
}

/// Get the number of decimal digits at the end of `s`.
fn trailing_digits(s: &str) -> usize {
    // It's faster to iterate backwards over bytes, and we're only counting ASCII digits.
    s.as_bytes()
        .iter()
        .rev()
        .take_while(|&&b| b'0' <= b && b <= b'9')
        .count()
}

/// Pre-parse a supposed entity name by splitting it into two parts: A head of lowercase ASCII
/// letters and numeric tail.
pub fn split_entity_name(name: &str) -> Option<(&str, u32)> {
    let (head, tail) = name.split_at(name.len() - trailing_digits(name));
    if tail.len() > 1 && tail.starts_with('0') {
        None
    } else {
        tail.parse().ok().map(|n| (head, n))
    }
}

/// Lexical analysis.
///
/// A `Lexer` reads text from a `&str` and provides a sequence of tokens.
///
/// Also keep track of a line number for error reporting.
///
pub struct Lexer<'a> {
    // Complete source being processed.
    source: &'a str,

    // Iterator into `source`.
    chars: CharIndices<'a>,

    // Next character to be processed, or `None` at the end.
    lookahead: Option<char>,

    // Index into `source` of lookahead character.
    pos: usize,

    // Current line number.
    line_number: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(s: &'a str) -> Self {
        let mut lex = Self {
            source: s,
            chars: s.char_indices(),
            lookahead: None,
            pos: 0,
            line_number: 1,
        };
        // Advance to the first char.
        lex.next_ch();
        lex
    }

    // Advance to the next character.
    // Return the next lookahead character, or None when the end is encountered.
    // Always update cur_ch to reflect
    fn next_ch(&mut self) -> Option<char> {
        if self.lookahead == Some('\n') {
            self.line_number += 1;
        }
        match self.chars.next() {
            Some((idx, ch)) => {
                self.pos = idx;
                self.lookahead = Some(ch);
            }
            None => {
                self.pos = self.source.len();
                self.lookahead = None;
            }
        }
        self.lookahead
    }

    // Get the location corresponding to `lookahead`.
    fn loc(&self) -> Location {
        Location {
            line_number: self.line_number,
        }
    }

    // Starting from `lookahead`, are we looking at `prefix`?
    fn looking_at(&self, prefix: &str) -> bool {
        self.source[self.pos..].starts_with(prefix)
    }

    // Starting from `lookahead`, are we looking at a number?
    fn looking_at_numeric(&self) -> bool {
        if let Some(c) = self.lookahead {
            if c.is_digit(10) {
                return true;
            }
            match c {
                '-' => return true,
                '+' => return true,
                '.' => return true,
                _ => {}
            }
            if self.looking_at("NaN") || self.looking_at("Inf") || self.looking_at("sNaN") {
                return true;
            }
        }
        false
    }

    // Scan a single-char token.
    fn scan_char(&mut self, tok: Token<'a>) -> Result<LocatedToken<'a>, LocatedError> {
        assert_ne!(self.lookahead, None);
        let loc = self.loc();
        self.next_ch();
        token(tok, loc)
    }

    // Scan a multi-char token.
    fn scan_chars(
        &mut self,
        count: usize,
        tok: Token<'a>,
    ) -> Result<LocatedToken<'a>, LocatedError> {
        let loc = self.loc();
        for _ in 0..count {
            assert_ne!(self.lookahead, None);
            self.next_ch();
        }
        token(tok, loc)
    }

    /// Get the rest of the current line.
    /// The next token returned by `next()` will be from the following lines.
    pub fn rest_of_line(&mut self) -> &'a str {
        let begin = self.pos;
        loop {
            match self.next_ch() {
                None | Some('\n') => return &self.source[begin..self.pos],
                _ => {}
            }
        }
    }

    // Scan a comment extending to the end of the current line.
    fn scan_comment(&mut self) -> Result<LocatedToken<'a>, LocatedError> {
        let loc = self.loc();
        let text = self.rest_of_line();
        token(Token::Comment(text), loc)
    }

    // Scan a number token which can represent either an integer or floating point number.
    //
    // Accept the following forms:
    //
    // - `10`: Integer
    // - `-10`: Integer
    // - `0xff_00`: Integer
    // - `0.0`: Float
    // - `0x1.f`: Float
    // - `-0x2.4`: Float
    // - `0x0.4p-34`: Float
    //
    // This function does not filter out all invalid numbers. It depends in the context-sensitive
    // decoding of the text for that. For example, the number of allowed digits in an `Ieee32` and
    // an `Ieee64` constant are different.
    fn scan_number(&mut self) -> Result<LocatedToken<'a>, LocatedError> {
        let begin = self.pos;
        let loc = self.loc();
        let mut is_float = false;

        // Skip a leading sign.
        match self.lookahead {
            Some('-') => {
                self.next_ch();
                if !self.looking_at_numeric() {
                    // If the next characters won't parse as a number, we return Token::Minus
                    return token(Token::Minus, loc);
                }
            }
            Some('+') => {
                self.next_ch();
                if !self.looking_at_numeric() {
                    // If the next characters won't parse as a number, we return Token::Plus
                    return token(Token::Plus, loc);
                }
            }
            _ => {}
        }

        // Check for NaNs with payloads.
        if self.looking_at("NaN:") || self.looking_at("sNaN:") {
            // Skip the `NaN:` prefix, the loop below won't accept it.
            // We expect a hexadecimal number to follow the colon.
            while self.next_ch() != Some(':') {}
            is_float = true;
        } else if self.looking_at("NaN") || self.looking_at("Inf") {
            // This is Inf or a default quiet NaN.
            is_float = true;
        }

        // Look for the end of this number. Detect the radix point if there is one.
        loop {
            match self.next_ch() {
                Some('-') | Some('_') => {}
                Some('.') => is_float = true,
                Some(ch) if ch.is_alphanumeric() => {}
                _ => break,
            }
        }
        let text = &self.source[begin..self.pos];
        if is_float {
            token(Token::Float(text), loc)
        } else {
            token(Token::Integer(text), loc)
        }
    }

    // Scan a 'word', which is an identifier-like sequence of characters beginning with '_' or an
    // alphabetic char, followed by zero or more alphanumeric or '_' characters.
    fn scan_word(&mut self) -> Result<LocatedToken<'a>, LocatedError> {
        let begin = self.pos;
        let loc = self.loc();

        assert!(self.lookahead == Some('_') || self.lookahead.unwrap().is_alphabetic());
        loop {
            match self.next_ch() {
                Some('_') => {}
                Some(ch) if ch.is_alphanumeric() => {}
                _ => break,
            }
        }
        let text = &self.source[begin..self.pos];

        // Look for numbered well-known entities like block15, v45, ...
        token(
            split_entity_name(text)
                .and_then(|(prefix, number)| {
                    Self::numbered_entity(prefix, number)
                        .or_else(|| Self::value_type(text, prefix, number))
                })
                .unwrap_or_else(|| match text {
                    "iflags" => Token::Type(types::IFLAGS),
                    "fflags" => Token::Type(types::FFLAGS),
                    "sarg_t" => Token::Type(types::SARG_T),
                    _ => Token::Identifier(text),
                }),
            loc,
        )
    }

    // If prefix is a well-known entity prefix and suffix is a valid entity number, return the
    // decoded token.
    fn numbered_entity(prefix: &str, number: u32) -> Option<Token<'a>> {
        match prefix {
            "v" => Value::with_number(number).map(Token::Value),
            "block" => Block::with_number(number).map(Token::Block),
            "ss" => Some(Token::StackSlot(number)),
            "gv" => Some(Token::GlobalValue(number)),
            "heap" => Some(Token::Heap(number)),
            "table" => Some(Token::Table(number)),
            "jt" => Some(Token::JumpTable(number)),
            "const" => Some(Token::Constant(number)),
            "fn" => Some(Token::FuncRef(number)),
            "sig" => Some(Token::SigRef(number)),
            "u" => Some(Token::UserRef(number)),
            _ => None,
        }
    }

    // Recognize a scalar or vector type.
    fn value_type(text: &str, prefix: &str, number: u32) -> Option<Token<'a>> {
        let is_vector = prefix.ends_with('x');
        let scalar = if is_vector {
            &prefix[0..prefix.len() - 1]
        } else {
            text
        };
        let base_type = match scalar {
            "i8" => types::I8,
            "i16" => types::I16,
            "i32" => types::I32,
            "i64" => types::I64,
            "i128" => types::I128,
            "f32" => types::F32,
            "f64" => types::F64,
            "b1" => types::B1,
            "b8" => types::B8,
            "b16" => types::B16,
            "b32" => types::B32,
            "b64" => types::B64,
            "b128" => types::B128,
            "r32" => types::R32,
            "r64" => types::R64,
            _ => return None,
        };
        if is_vector {
            if number <= u32::from(u16::MAX) {
                base_type.by(number as u16).map(Token::Type)
            } else {
                None
            }
        } else {
            Some(Token::Type(base_type))
        }
    }

    fn scan_name(&mut self) -> Result<LocatedToken<'a>, LocatedError> {
        let loc = self.loc();
        let begin = self.pos + 1;

        assert_eq!(self.lookahead, Some('%'));

        while let Some(c) = self.next_ch() {
            if !(c.is_ascii() && c.is_alphanumeric() || c == '_') {
                break;
            }
        }

        let end = self.pos;
        token(Token::Name(&self.source[begin..end]), loc)
    }

    /// Scan for a multi-line quoted string with no escape character.
    fn scan_string(&mut self) -> Result<LocatedToken<'a>, LocatedError> {
        let loc = self.loc();
        let begin = self.pos + 1;

        assert_eq!(self.lookahead, Some('"'));

        while let Some(c) = self.next_ch() {
            if c == '"' {
                break;
            }
        }

        let end = self.pos;
        if self.lookahead != Some('"') {
            return error(LexError::InvalidChar, self.loc());
        }
        self.next_ch();
        token(Token::String(&self.source[begin..end]), loc)
    }

    fn scan_hex_sequence(&mut self) -> Result<LocatedToken<'a>, LocatedError> {
        let loc = self.loc();
        let begin = self.pos + 1;

        assert_eq!(self.lookahead, Some('#'));

        while let Some(c) = self.next_ch() {
            if !char::is_digit(c, 16) {
                break;
            }
        }

        let end = self.pos;
        token(Token::HexSequence(&self.source[begin..end]), loc)
    }

    fn scan_srcloc(&mut self) -> Result<LocatedToken<'a>, LocatedError> {
        let loc = self.loc();
        let begin = self.pos + 1;

        assert_eq!(self.lookahead, Some('@'));

        while let Some(c) = self.next_ch() {
            if !char::is_digit(c, 16) {
                break;
            }
        }

        let end = self.pos;
        token(Token::SourceLoc(&self.source[begin..end]), loc)
    }

    /// Get the next token or a lexical error.
    ///
    /// Return None when the end of the source is encountered.
    #[allow(clippy::cognitive_complexity)]
    pub fn next(&mut self) -> Option<Result<LocatedToken<'a>, LocatedError>> {
        loop {
            let loc = self.loc();
            return match self.lookahead {
                None => None,
                Some(';') => Some(self.scan_comment()),
                Some('(') => Some(self.scan_char(Token::LPar)),
                Some(')') => Some(self.scan_char(Token::RPar)),
                Some('{') => Some(self.scan_char(Token::LBrace)),
                Some('}') => Some(self.scan_char(Token::RBrace)),
                Some('[') => Some(self.scan_char(Token::LBracket)),
                Some(']') => Some(self.scan_char(Token::RBracket)),
                Some(',') => Some(self.scan_char(Token::Comma)),
                Some('.') => Some(self.scan_char(Token::Dot)),
                Some(':') => Some(self.scan_char(Token::Colon)),
                Some('=') => Some(self.scan_char(Token::Equal)),
                Some('!') => Some(self.scan_char(Token::Not)),
                Some('+') => Some(self.scan_number()),
                Some('-') => {
                    if self.looking_at("->") {
                        Some(self.scan_chars(2, Token::Arrow))
                    } else {
                        Some(self.scan_number())
                    }
                }
                Some(ch) if ch.is_digit(10) => Some(self.scan_number()),
                Some(ch) if ch.is_alphabetic() => {
                    if self.looking_at("NaN") || self.looking_at("Inf") {
                        Some(self.scan_number())
                    } else {
                        Some(self.scan_word())
                    }
                }
                Some('%') => Some(self.scan_name()),
                Some('"') => Some(self.scan_string()),
                Some('#') => Some(self.scan_hex_sequence()),
                Some('@') => Some(self.scan_srcloc()),
                Some(ch) if ch.is_whitespace() => {
                    self.next_ch();
                    continue;
                }
                _ => {
                    // Skip invalid char, return error.
                    self.next_ch();
                    Some(error(LexError::InvalidChar, loc))
                }
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::trailing_digits;
    use super::*;
    use crate::error::Location;
    use cranelift_codegen::ir::types;
    use cranelift_codegen::ir::{Block, Value};

    #[test]
    fn digits() {
        assert_eq!(trailing_digits(""), 0);
        assert_eq!(trailing_digits("x"), 0);
        assert_eq!(trailing_digits("0x"), 0);
        assert_eq!(trailing_digits("x1"), 1);
        assert_eq!(trailing_digits("1x1"), 1);
        assert_eq!(trailing_digits("1x01"), 2);
    }

    #[test]
    fn entity_name() {
        assert_eq!(split_entity_name(""), None);
        assert_eq!(split_entity_name("x"), None);
        assert_eq!(split_entity_name("x+"), None);
        assert_eq!(split_entity_name("x+1"), Some(("x+", 1)));
        assert_eq!(split_entity_name("x-1"), Some(("x-", 1)));
        assert_eq!(split_entity_name("1"), Some(("", 1)));
        assert_eq!(split_entity_name("x1"), Some(("x", 1)));
        assert_eq!(split_entity_name("xy0"), Some(("xy", 0)));
        // Reject this non-canonical form.
        assert_eq!(split_entity_name("inst01"), None);
    }

    fn token<'a>(token: Token<'a>, line: usize) -> Option<Result<LocatedToken<'a>, LocatedError>> {
        Some(super::token(token, Location { line_number: line }))
    }

    fn error<'a>(error: LexError, line: usize) -> Option<Result<LocatedToken<'a>, LocatedError>> {
        Some(super::error(error, Location { line_number: line }))
    }

    #[test]
    fn make_lexer() {
        let mut l1 = Lexer::new("");
        let mut l2 = Lexer::new(" ");
        let mut l3 = Lexer::new("\n ");

        assert_eq!(l1.next(), None);
        assert_eq!(l2.next(), None);
        assert_eq!(l3.next(), None);
    }

    #[test]
    fn lex_comment() {
        let mut lex = Lexer::new("; hello");
        assert_eq!(lex.next(), token(Token::Comment("; hello"), 1));
        assert_eq!(lex.next(), None);

        lex = Lexer::new("\n  ;hello\n;foo");
        assert_eq!(lex.next(), token(Token::Comment(";hello"), 2));
        assert_eq!(lex.next(), token(Token::Comment(";foo"), 3));
        assert_eq!(lex.next(), None);

        // Scan a comment after an invalid char.
        let mut lex = Lexer::new("$; hello");
        assert_eq!(lex.next(), error(LexError::InvalidChar, 1));
        assert_eq!(lex.next(), token(Token::Comment("; hello"), 1));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_chars() {
        let mut lex = Lexer::new("(); hello\n = :{, }.");
        assert_eq!(lex.next(), token(Token::LPar, 1));
        assert_eq!(lex.next(), token(Token::RPar, 1));
        assert_eq!(lex.next(), token(Token::Comment("; hello"), 1));
        assert_eq!(lex.next(), token(Token::Equal, 2));
        assert_eq!(lex.next(), token(Token::Colon, 2));
        assert_eq!(lex.next(), token(Token::LBrace, 2));
        assert_eq!(lex.next(), token(Token::Comma, 2));
        assert_eq!(lex.next(), token(Token::RBrace, 2));
        assert_eq!(lex.next(), token(Token::Dot, 2));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_numbers() {
        let mut lex = Lexer::new(" 0 2_000 -1,0xf -0x0 0.0 0x0.4p-34 NaN +5");
        assert_eq!(lex.next(), token(Token::Integer("0"), 1));
        assert_eq!(lex.next(), token(Token::Integer("2_000"), 1));
        assert_eq!(lex.next(), token(Token::Integer("-1"), 1));
        assert_eq!(lex.next(), token(Token::Comma, 1));
        assert_eq!(lex.next(), token(Token::Integer("0xf"), 1));
        assert_eq!(lex.next(), token(Token::Integer("-0x0"), 1));
        assert_eq!(lex.next(), token(Token::Float("0.0"), 1));
        assert_eq!(lex.next(), token(Token::Float("0x0.4p-34"), 1));
        assert_eq!(lex.next(), token(Token::Float("NaN"), 1));
        assert_eq!(lex.next(), token(Token::Integer("+5"), 1));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_identifiers() {
        let mut lex = Lexer::new(
            "v0 v00 vx01 block1234567890 block5234567890 v1x vx1 vxvx4 \
             function0 function b1 i32x4 f32x5 \
             iflags fflags sarg_t iflagss",
        );
        assert_eq!(
            lex.next(),
            token(Token::Value(Value::with_number(0).unwrap()), 1)
        );
        assert_eq!(lex.next(), token(Token::Identifier("v00"), 1));
        assert_eq!(lex.next(), token(Token::Identifier("vx01"), 1));
        assert_eq!(
            lex.next(),
            token(Token::Block(Block::with_number(1234567890).unwrap()), 1)
        );
        assert_eq!(lex.next(), token(Token::Identifier("block5234567890"), 1));
        assert_eq!(lex.next(), token(Token::Identifier("v1x"), 1));
        assert_eq!(lex.next(), token(Token::Identifier("vx1"), 1));
        assert_eq!(lex.next(), token(Token::Identifier("vxvx4"), 1));
        assert_eq!(lex.next(), token(Token::Identifier("function0"), 1));
        assert_eq!(lex.next(), token(Token::Identifier("function"), 1));
        assert_eq!(lex.next(), token(Token::Type(types::B1), 1));
        assert_eq!(lex.next(), token(Token::Type(types::I32X4), 1));
        assert_eq!(lex.next(), token(Token::Identifier("f32x5"), 1));
        assert_eq!(lex.next(), token(Token::Type(types::IFLAGS), 1));
        assert_eq!(lex.next(), token(Token::Type(types::FFLAGS), 1));
        assert_eq!(lex.next(), token(Token::Type(types::SARG_T), 1));
        assert_eq!(lex.next(), token(Token::Identifier("iflagss"), 1));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_hex_sequences() {
        let mut lex = Lexer::new("#0 #DEADbeef123 #789");

        assert_eq!(lex.next(), token(Token::HexSequence("0"), 1));
        assert_eq!(lex.next(), token(Token::HexSequence("DEADbeef123"), 1));
        assert_eq!(lex.next(), token(Token::HexSequence("789"), 1));
    }

    #[test]
    fn lex_names() {
        let mut lex = Lexer::new("%0 %x3 %function %123_abc %ss0 %v3 %block11 %const42 %_");

        assert_eq!(lex.next(), token(Token::Name("0"), 1));
        assert_eq!(lex.next(), token(Token::Name("x3"), 1));
        assert_eq!(lex.next(), token(Token::Name("function"), 1));
        assert_eq!(lex.next(), token(Token::Name("123_abc"), 1));
        assert_eq!(lex.next(), token(Token::Name("ss0"), 1));
        assert_eq!(lex.next(), token(Token::Name("v3"), 1));
        assert_eq!(lex.next(), token(Token::Name("block11"), 1));
        assert_eq!(lex.next(), token(Token::Name("const42"), 1));
        assert_eq!(lex.next(), token(Token::Name("_"), 1));
    }

    #[test]
    fn lex_strings() {
        let mut lex = Lexer::new(
            r#"""  "0" "x3""function" "123 abc" "\" "start
                    and end on
                    different lines" "#,
        );

        assert_eq!(lex.next(), token(Token::String(""), 1));
        assert_eq!(lex.next(), token(Token::String("0"), 1));
        assert_eq!(lex.next(), token(Token::String("x3"), 1));
        assert_eq!(lex.next(), token(Token::String("function"), 1));
        assert_eq!(lex.next(), token(Token::String("123 abc"), 1));
        assert_eq!(lex.next(), token(Token::String(r#"\"#), 1));
        assert_eq!(
            lex.next(),
            token(
                Token::String(
                    r#"start
                    and end on
                    different lines"#
                ),
                1
            )
        );
    }

    #[test]
    fn lex_userrefs() {
        let mut lex = Lexer::new("u0 u1 u234567890 u9:8765");

        assert_eq!(lex.next(), token(Token::UserRef(0), 1));
        assert_eq!(lex.next(), token(Token::UserRef(1), 1));
        assert_eq!(lex.next(), token(Token::UserRef(234567890), 1));
        assert_eq!(lex.next(), token(Token::UserRef(9), 1));
        assert_eq!(lex.next(), token(Token::Colon, 1));
        assert_eq!(lex.next(), token(Token::Integer("8765"), 1));
        assert_eq!(lex.next(), None);
    }
}

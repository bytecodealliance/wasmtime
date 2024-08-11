//! Lexer for the ISLE language.

use std::borrow::Cow;

use crate::error::{Error, Span};
use crate::files::Files;

type Result<T> = std::result::Result<T, Error>;

/// The lexer.
///
/// Breaks source text up into a sequence of tokens (with source positions).
#[derive(Clone, Debug)]
pub struct Lexer<'src> {
    src: &'src str,
    pos: Pos,
    lookahead: Option<(Pos, Token)>,
}

/// A source position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash, PartialOrd, Ord)]
pub struct Pos {
    /// This source position's file.
    ///
    /// Indexes into `Lexer::filenames` early in the compiler pipeline, and
    /// later into `TypeEnv::filenames` once we get into semantic analysis.
    pub file: usize,
    /// This source position's byte offset in the file.
    pub offset: usize,
}

impl Pos {
    /// Create a new `Pos`.
    pub fn new(file: usize, offset: usize) -> Self {
        Self { file, offset }
    }

    /// Print this source position as `file.isle line 12`.
    pub fn pretty_print_line(&self, files: &Files) -> String {
        format!(
            "{} line {}",
            files.file_name(self.file).unwrap(),
            files.file_line_map(self.file).unwrap().line(self.offset)
        )
    }
}

/// A token of ISLE source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// Left paren.
    LParen,
    /// Right paren.
    RParen,
    /// A symbol, e.g. `Foo`.
    Symbol(String),
    /// An integer.
    Int(i128),
    /// `@`
    At,
}

impl<'src> Lexer<'src> {
    /// Create a new lexer for the given source contents
    pub fn new(file: usize, src: &'src str) -> Result<Lexer<'src>> {
        let mut l = Lexer {
            src,
            pos: Pos::new(file, 0),
            lookahead: None,
        };
        l.reload()?;
        Ok(l)
    }

    /// Get the lexer's current source position.
    pub fn pos(&self) -> Pos {
        self.pos
    }

    fn advance_pos(&mut self) {
        self.pos.offset += 1;
    }

    fn error(&self, pos: Pos, msg: impl Into<String>) -> Error {
        Error::ParseError {
            msg: msg.into(),
            span: Span::new_single(pos),
        }
    }

    fn next_token(&mut self) -> Result<Option<(Pos, Token)>> {
        fn is_sym_first_char(c: u8) -> bool {
            match c {
                b'-' | b'0'..=b'9' | b'(' | b')' | b';' | b'<' | b'>' => false,
                c if c.is_ascii_whitespace() => false,
                _ => true,
            }
        }
        fn is_sym_other_char(c: u8) -> bool {
            match c {
                b'(' | b')' | b';' | b'@' | b'<' => false,
                c if c.is_ascii_whitespace() => false,
                _ => true,
            }
        }

        // Skip any whitespace and any comments.
        while self.pos.offset < self.src.len() {
            if self.src.as_bytes()[self.pos.offset].is_ascii_whitespace() {
                self.advance_pos();
                continue;
            }
            if self.src.as_bytes()[self.pos.offset] == b';' {
                while self.pos.offset < self.src.len()
                    && self.src.as_bytes()[self.pos.offset] != b'\n'
                {
                    self.advance_pos();
                }
                continue;
            }
            break;
        }

        let Some(c) = self.src.as_bytes().get(self.pos.offset) else {
            return Ok(None);
        };
        let char_pos = self.pos();
        match c {
            b'(' => {
                self.advance_pos();
                Ok(Some((char_pos, Token::LParen)))
            }
            b')' => {
                self.advance_pos();
                Ok(Some((char_pos, Token::RParen)))
            }
            b'@' => {
                self.advance_pos();
                Ok(Some((char_pos, Token::At)))
            }
            c if is_sym_first_char(*c) => {
                let start = self.pos.offset;
                let start_pos = self.pos();
                while self.pos.offset < self.src.len()
                    && is_sym_other_char(self.src.as_bytes()[self.pos.offset])
                {
                    self.advance_pos();
                }
                let end = self.pos.offset;
                let s = &self.src[start..end];
                debug_assert!(!s.is_empty());
                Ok(Some((start_pos, Token::Symbol(s.to_string()))))
            }
            c @ (b'0'..=b'9' | b'-') => {
                let start_pos = self.pos();
                let neg = if *c == b'-' {
                    self.advance_pos();
                    true
                } else {
                    false
                };

                let mut radix = 10;

                // Check for prefixed literals.
                match (
                    self.src.as_bytes().get(self.pos.offset),
                    self.src.as_bytes().get(self.pos.offset + 1),
                ) {
                    (Some(b'0'), Some(b'x' | b'X')) => {
                        self.advance_pos();
                        self.advance_pos();
                        radix = 16;
                    }
                    (Some(b'0'), Some(b'o' | b'O')) => {
                        self.advance_pos();
                        self.advance_pos();
                        radix = 8;
                    }
                    (Some(b'0'), Some(b'b' | b'B')) => {
                        self.advance_pos();
                        self.advance_pos();
                        radix = 2;
                    }
                    _ => {}
                }

                // Find the range in the buffer for this integer literal. We'll
                // pass this range to `i64::from_str_radix` to do the actual
                // string-to-integer conversion.
                let start = self.pos.offset;
                while self.pos.offset < self.src.len()
                    && ((radix <= 10 && self.src.as_bytes()[self.pos.offset].is_ascii_digit())
                        || (radix == 16
                            && self.src.as_bytes()[self.pos.offset].is_ascii_hexdigit())
                        || self.src.as_bytes()[self.pos.offset] == b'_')
                {
                    self.advance_pos();
                }
                let end = self.pos.offset;
                let s = &self.src[start..end];
                let s = if s.contains('_') {
                    Cow::Owned(s.replace('_', ""))
                } else {
                    Cow::Borrowed(s)
                };

                // Support either signed range (-2^127..2^127) or
                // unsigned range (0..2^128).
                let num = i128::from_str_radix(&s, radix)
                    .or_else(|_| u128::from_str_radix(&s, radix).map(|val| val as i128))
                    .map_err(|e| self.error(start_pos, e.to_string()))?;

                let tok = if neg {
                    Token::Int(num.checked_neg().ok_or_else(|| {
                        self.error(start_pos, "integer literal cannot fit in i128")
                    })?)
                } else {
                    Token::Int(num)
                };
                Ok(Some((start_pos, tok)))
            }
            c => Err(self.error(self.pos, format!("Unexpected character '{c}'"))),
        }
    }

    /// Get the next token from this lexer's token stream, if any.
    pub fn next(&mut self) -> Result<Option<(Pos, Token)>> {
        let tok = self.lookahead.take();
        self.reload()?;
        Ok(tok)
    }

    fn reload(&mut self) -> Result<()> {
        if self.lookahead.is_none() && self.pos.offset < self.src.len() {
            self.lookahead = self.next_token()?;
        }
        Ok(())
    }

    /// Peek ahead at the next token.
    pub fn peek(&self) -> Option<&(Pos, Token)> {
        self.lookahead.as_ref()
    }

    /// Are we at the end of the source input?
    pub fn eof(&self) -> bool {
        self.lookahead.is_none()
    }
}

impl Token {
    /// Is this an `Int` token?
    pub fn is_int(&self) -> bool {
        matches!(self, Token::Int(_))
    }

    /// Is this a `Sym` token?
    pub fn is_sym(&self) -> bool {
        matches!(self, Token::Symbol(_))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn lex(src: &str) -> Vec<Token> {
        let mut toks = vec![];
        let mut lexer = Lexer::new(0, src).unwrap();
        while let Some((_, tok)) = lexer.next().unwrap() {
            toks.push(tok);
        }
        toks
    }

    #[test]
    fn lexer_basic() {
        assert_eq!(
            lex(";; comment\n; another\r\n   \t(one two three 23 -568  )\n"),
            [
                Token::LParen,
                Token::Symbol("one".to_string()),
                Token::Symbol("two".to_string()),
                Token::Symbol("three".to_string()),
                Token::Int(23),
                Token::Int(-568),
                Token::RParen
            ]
        );
    }

    #[test]
    fn ends_with_sym() {
        assert_eq!(lex("asdf"), [Token::Symbol("asdf".to_string())]);
    }

    #[test]
    fn ends_with_num() {
        assert_eq!(lex("23"), [Token::Int(23)]);
    }

    #[test]
    fn weird_syms() {
        assert_eq!(
            lex("(+ [] => !! _test!;comment\n)"),
            [
                Token::LParen,
                Token::Symbol("+".to_string()),
                Token::Symbol("[]".to_string()),
                Token::Symbol("=>".to_string()),
                Token::Symbol("!!".to_string()),
                Token::Symbol("_test!".to_string()),
                Token::RParen,
            ]
        );
    }
}

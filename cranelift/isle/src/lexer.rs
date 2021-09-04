//! Lexer for the ISLE language.

#[derive(Clone, Debug)]
pub struct Lexer<'a> {
    buf: &'a [u8],
    pos: Pos,
    lookahead: Option<(Pos, Token<'a>)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pos {
    pub offset: usize,
    pub line: usize,
    pub col: usize,
}

impl Pos {
    pub fn pretty_print(&self, filename: &str) -> String {
        format!("{}:{}:{}", filename, self.line, self.col)
    }
    pub fn pretty_print_line(&self, filename: &str) -> String {
        format!("{} line {}", filename, self.line)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token<'a> {
    LParen,
    RParen,
    Symbol(&'a str),
    Int(i64),
}

impl<'a> Lexer<'a> {
    pub fn new(s: &'a str) -> Lexer<'a> {
        let mut l = Lexer {
            buf: s.as_bytes(),
            pos: Pos {
                offset: 0,
                line: 1,
                col: 0,
            },
            lookahead: None,
        };
        l.reload();
        l
    }

    pub fn offset(&self) -> usize {
        self.pos.offset
    }

    pub fn pos(&self) -> Pos {
        self.pos
    }

    fn next_token(&mut self) -> Option<(Pos, Token<'a>)> {
        fn is_sym_first_char(c: u8) -> bool {
            match c {
                b'-' | b'0'..=b'9' | b'(' | b')' | b';' => false,
                c if c.is_ascii_whitespace() => false,
                _ => true,
            }
        }
        fn is_sym_other_char(c: u8) -> bool {
            match c {
                b'(' | b')' | b';' => false,
                c if c.is_ascii_whitespace() => false,
                _ => true,
            }
        }

        // Skip any whitespace and any comments.
        while self.pos.offset < self.buf.len() {
            if self.buf[self.pos.offset].is_ascii_whitespace() {
                self.pos.col += 1;
                if self.buf[self.pos.offset] == b'\n' {
                    self.pos.line += 1;
                    self.pos.col = 0;
                }
                self.pos.offset += 1;
                continue;
            }
            if self.buf[self.pos.offset] == b';' {
                while self.pos.offset < self.buf.len() && self.buf[self.pos.offset] != b'\n' {
                    self.pos.offset += 1;
                }
                self.pos.line += 1;
                self.pos.col = 0;
                continue;
            }
            break;
        }

        if self.pos.offset == self.buf.len() {
            return None;
        }

        let char_pos = self.pos;
        match self.buf[self.pos.offset] {
            b'(' => {
                self.pos.offset += 1;
                self.pos.col += 1;
                Some((char_pos, Token::LParen))
            }
            b')' => {
                self.pos.offset += 1;
                self.pos.col += 1;
                Some((char_pos, Token::RParen))
            }
            c if is_sym_first_char(c) => {
                let start = self.pos.offset;
                let start_pos = self.pos;
                while self.pos.offset < self.buf.len()
                    && is_sym_other_char(self.buf[self.pos.offset])
                {
                    self.pos.col += 1;
                    self.pos.offset += 1;
                }
                let end = self.pos.offset;
                let s = std::str::from_utf8(&self.buf[start..end])
                    .expect("Only ASCII characters, should be UTF-8");
                Some((start_pos, Token::Symbol(s)))
            }
            c if (c >= b'0' && c <= b'9') || c == b'-' => {
                let start_pos = self.pos;
                let neg = if c == b'-' {
                    self.pos.offset += 1;
                    self.pos.col += 1;
                    true
                } else {
                    false
                };
                let mut num = 0;
                while self.pos.offset < self.buf.len()
                    && (self.buf[self.pos.offset] >= b'0' && self.buf[self.pos.offset] <= b'9')
                {
                    num = (num * 10) + (self.buf[self.pos.offset] - b'0') as i64;
                    self.pos.offset += 1;
                    self.pos.col += 1;
                }

                let tok = if neg {
                    Token::Int(-num)
                } else {
                    Token::Int(num)
                };
                Some((start_pos, tok))
            }
            c => panic!("Unexpected character '{}' at offset {}", c, self.pos.offset),
        }
    }

    fn reload(&mut self) {
        if self.lookahead.is_none() && self.pos.offset < self.buf.len() {
            self.lookahead = self.next_token();
        }
    }

    pub fn peek(&self) -> Option<(Pos, Token<'a>)> {
        self.lookahead
    }

    pub fn eof(&self) -> bool {
        self.lookahead.is_none()
    }
}

impl<'a> std::iter::Iterator for Lexer<'a> {
    type Item = (Pos, Token<'a>);

    fn next(&mut self) -> Option<(Pos, Token<'a>)> {
        let tok = self.lookahead.take();
        self.reload();
        tok
    }
}

impl<'a> Token<'a> {
    pub fn is_int(&self) -> bool {
        match self {
            Token::Int(_) => true,
            _ => false,
        }
    }

    pub fn is_sym(&self) -> bool {
        match self {
            Token::Symbol(_) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lexer_basic() {
        assert_eq!(
            Lexer::new(";; comment\n; another\r\n   \t(one two three 23 -568  )\n")
                .map(|(_, tok)| tok)
                .collect::<Vec<_>>(),
            vec![
                Token::LParen,
                Token::Symbol("one"),
                Token::Symbol("two"),
                Token::Symbol("three"),
                Token::Int(23),
                Token::Int(-568),
                Token::RParen
            ]
        );
    }

    #[test]
    fn ends_with_sym() {
        assert_eq!(
            Lexer::new("asdf").map(|(_, tok)| tok).collect::<Vec<_>>(),
            vec![Token::Symbol("asdf"),]
        );
    }

    #[test]
    fn ends_with_num() {
        assert_eq!(
            Lexer::new("23").map(|(_, tok)| tok).collect::<Vec<_>>(),
            vec![Token::Int(23)],
        );
    }

    #[test]
    fn weird_syms() {
        assert_eq!(
            Lexer::new("(+ [] => !! _test!;comment\n)")
                .map(|(_, tok)| tok)
                .collect::<Vec<_>>(),
            vec![
                Token::LParen,
                Token::Symbol("+"),
                Token::Symbol("[]"),
                Token::Symbol("=>"),
                Token::Symbol("!!"),
                Token::Symbol("_test!"),
                Token::RParen,
            ]
        );
    }
}

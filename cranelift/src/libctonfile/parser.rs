
// ====--------------------------------------------------------------------------------------====//
//
// Parser for .cton files.
//
// ====--------------------------------------------------------------------------------------====//

use std::result;
use std::fmt::{self, Display, Formatter, Write};
use lexer::{self, Lexer, Token};
use cretonne::{types, repr};

pub use lexer::Location;

/// A parse error is returned when the parse failed.
#[derive(Debug)]
pub struct Error {
    pub location: Location,
    pub message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.location.line_number, self.message)
    }
}

pub type Result<T> = result::Result<T, Error>;

pub struct Parser<'a> {
    lex: Lexer<'a>,

    lex_error: Option<lexer::Error>,

    // Current lookahead token.
    lookahead: Option<Token<'a>>,

    // Location of lookahead.
    location: Location,
}

impl<'a> Parser<'a> {
    /// Create a new `Parser` which reads `text`. The referenced text must outlive the parser.
    pub fn new(text: &'a str) -> Parser {
        Parser {
            lex: Lexer::new(text),
            lex_error: None,
            lookahead: None,
            location: Location { line_number: 0 },
        }
    }

    /// Parse the entire string into a list of functions.
    pub fn parse(text: &'a str) -> Result<Vec<repr::Function>> {
        Self::new(text).parse_function_list()
    }

    // Consume the current lookahead token and return it.
    fn consume(&mut self) -> Token<'a> {
        self.lookahead.take().expect("No token to consume")
    }

    // Get the current lookahead token, after making sure there is one.
    fn token(&mut self) -> Option<Token<'a>> {
        if self.lookahead == None {
            match self.lex.next() {
                Some(Ok(lexer::LocatedToken { token, location })) => {
                    self.lookahead = Some(token);
                    self.location = location;
                }
                Some(Err(lexer::LocatedError { error, location })) => {
                    self.lex_error = Some(error);
                    self.location = location;
                }
                None => {}
            }
        }
        return self.lookahead;
    }

    // Generate an error.
    fn error(&self, message: &str) -> Error {
        Error {
            location: self.location,
            message:
                // If we have a lexer error latched, report that.
                match self.lex_error {
                    Some(lexer::Error::InvalidChar) => "invalid character".to_string(),
                    None => message.to_string(),
                }
        }
    }

    // Match and consume a token without payload.
    fn match_token(&mut self, want: Token<'a>, err_msg: &str) -> Result<Token<'a>> {
        if self.token() == Some(want) {
            Ok(self.consume())
        } else {
            Err(self.error(err_msg))
        }
    }

    // If the next token is a `want`, consume it, otherwise do nothing.
    fn optional(&mut self, want: Token<'a>) -> bool {
        if self.token() == Some(want) {
            self.consume();
            true
        } else {
            false
        }
    }

    /// Parse a list of function definitions.
    ///
    /// This is the top-level parse function matching the whole contents of a file.
    pub fn parse_function_list(&mut self) -> Result<Vec<repr::Function>> {
        let mut list = Vec::new();
        while self.token().is_some() {
            list.push(try!(self.parse_function()));
        }
        Ok(list)
    }

    // Parse a whole function definition.
    //
    // function ::= * "function" name signature { ... }
    //
    fn parse_function(&mut self) -> Result<repr::Function> {
        try!(self.match_token(Token::Function, "expected 'function' keyword"));

        // function ::= "function" * name signature { ... }
        let name = try!(self.parse_function_name());

        // function ::= "function" name * signature { ... }
        let sig = try!(self.parse_signature());

        let mut func = repr::Function::new();

        try!(self.match_token(Token::LBrace, "expected '{' before function body"));
        try!(self.match_token(Token::RBrace, "expected '}' after function body"));

        Ok(func)
    }

    // Parse a function name.
    //
    // function ::= "function" * name signature { ... }
    //
    fn parse_function_name(&mut self) -> Result<String> {
        match self.token() {
            Some(Token::Identifier(s)) => {
                self.consume();
                Ok(s.to_string())
            }
            _ => Err(self.error("expected function name")),
        }
    }

    // Parse a function signature.
    //
    // signature ::=  * "(" [arglist] ")" ["->" retlist] [call_conv]
    //
    fn parse_signature(&mut self) -> Result<types::Signature> {
        let mut sig = types::Signature::new();

        try!(self.match_token(Token::LPar, "expected function signature: ( args... )"));
        // signature ::=  "(" * [arglist] ")" ["->" retlist] [call_conv]
        if self.token() != Some(Token::RPar) {
            sig.argument_types = try!(self.parse_argument_list());
        }
        try!(self.match_token(Token::RPar, "expected ')' after function arguments"));
        if self.optional(Token::Arrow) {
            sig.return_types = try!(self.parse_argument_list());
        }

        // TBD: calling convention.

        Ok(sig)
    }

    // Parse list of function argument / return value types.
    //
    // arglist ::= * arg { "," arg }
    //
    fn parse_argument_list(&mut self) -> Result<Vec<types::ArgumentType>> {
        let mut list = Vec::new();

        // arglist ::= * arg { "," arg }
        list.push(try!(self.parse_argument_type()));

        // arglist ::= arg * { "," arg }
        while self.optional(Token::Comma) {
            // arglist ::= arg { "," * arg }
            list.push(try!(self.parse_argument_type()));
        }

        Ok(list)
    }

    // Parse a single argument type with flags.
    fn parse_argument_type(&mut self) -> Result<types::ArgumentType> {
        // arg ::= * type { flag }
        let mut arg = if let Some(Token::Type(t)) = self.token() {
            types::ArgumentType::new(t)
        } else {
            return Err(self.error("expected argument type"));
        };
        self.consume();

        // arg ::= type * { flag }
        while let Some(Token::Identifier(s)) = self.token() {
            match s {
                "uext" => arg.extension = types::ArgumentExtension::Uext,
                "sext" => arg.extension = types::ArgumentExtension::Sext,
                "inreg" => arg.inreg = true,
                _ => break,
            }
            self.consume();
        }

        Ok(arg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cretonne::types::{self, ArgumentType, ArgumentExtension};

    #[test]
    fn argument_type() {
        let mut p = Parser::new("i32 sext");
        let arg = p.parse_argument_type().unwrap();
        assert_eq!(arg,
                   ArgumentType {
                       value_type: types::I32,
                       extension: ArgumentExtension::Sext,
                       inreg: false,
                   });
        let Error { location, message } = p.parse_argument_type().unwrap_err();
        assert_eq!(location.line_number, 1);
        assert_eq!(message, "expected argument type");
    }

    #[test]
    fn signature() {
        let sig = Parser::new("()").parse_signature().unwrap();
        assert_eq!(sig.argument_types.len(), 0);
        assert_eq!(sig.return_types.len(), 0);

        let sig2 = Parser::new("(i8 inreg uext, f32, f64) -> i32 sext, f64")
                       .parse_signature()
                       .unwrap();
        assert_eq!(format!("{}", sig2),
                   "(i8 uext inreg, f32, f64) -> i32 sext, f64");

        // `void` is not recognized as a type by the lexer. It should not appear in files.
        assert_eq!(format!("{}",
                           Parser::new("() -> void").parse_signature().unwrap_err()),
                   "1: expected argument type");
        assert_eq!(format!("{}", Parser::new("i8 -> i8").parse_signature().unwrap_err()),
                   "1: expected function signature: ( args... )");
        assert_eq!(format!("{}",
                           Parser::new("(i8 -> i8").parse_signature().unwrap_err()),
                   "1: expected ')' after function arguments");
    }
}

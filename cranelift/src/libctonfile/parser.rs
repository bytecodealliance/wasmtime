
// ====--------------------------------------------------------------------------------------====//
//
// Parser for .cton files.
//
// ====--------------------------------------------------------------------------------------====//

use std::result;
use lexer::{self, Lexer, Token};
use cretonne::{types, repr};

pub use lexer::Location;

/// A parse error is returned when the parse failed.
pub struct Error {
    pub location: Location,
    pub message: String,
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
                    Some(lexer::Error::InvalidChar) => "Invalid character".to_string(),
                    None => message.to_string(),
                }
        }
    }

    // Match and consume a token without payload.
    fn match_token(&mut self, want: Token<'a>, err_msg: &str) -> Result<Token<'a>> {
        match self.token() {
            Some(ref t) if *t == want => Ok(self.consume()),
            _ => Err(self.error(err_msg)),
        }
    }

    // if the next token is a `want`, consume it, otherwise do nothing.
    fn optional(&mut self, want: Token<'a>) -> bool {
        match self.token() {
            Some(t) if t == want => {
                self.consume();
                true
            }
            _ => false,
        }
    }

    // Parse a whole function definition.
    //
    // function ::= * "function" name signature { ... }
    //
    fn parse_function(&mut self) -> Result<repr::Function> {
        try!(self.match_token(Token::Function, "Expected 'function' keyword"));

        // function ::= "function" * name signature { ... }
        let name = try!(self.parse_function_name());

        // function ::= "function" name * signature { ... }
        let sig = try!(self.parse_signature());

        let mut func = repr::Function::new();

        try!(self.match_token(Token::LBrace, "Expected '{' before function body"));
        try!(self.match_token(Token::RBrace, "Expected '}' after function body"));

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
            _ => Err(self.error("Expected function name")),
        }
    }

    // Parse a function signature.
    //
    // signature ::=  * "(" arglist ")" ["->" retlist] [call_conv]
    // callconv  ::=  string
    //
    // function ::= "function" * name signature { ... }
    //
    fn parse_signature(&mut self) -> Result<types::Signature> {
        let mut sig = types::Signature::new();

        try!(self.match_token(Token::LPar, "Expected function signature: '(' args... ')'"));
        // signature ::=  "(" * arglist ")" ["->" retlist] [call_conv]
        sig.argument_types = try!(self.parse_argument_list());
        try!(self.match_token(Token::RPar, "Expected ')' after function arguments"));
        if self.optional(Token::Arrow) {
            sig.return_types = try!(self.parse_argument_list());
            if sig.return_types.is_empty() {
                return Err(self.error("Missing return type after '->'"));
            }
        }

        // TBD: calling convention.

        Ok(sig)
    }

    // Parse (possibly empty) list of function argument / return value types.
    //
    // arglist ::= * <empty>
    //             * arg
    //             * arglist "," arg
    fn parse_argument_list(&mut self) -> Result<Vec<types::ArgumentType>> {
        let mut list = Vec::new();
        // arglist   ::= * <empty>
        //               * arg
        match self.token() {
            Some(Token::Type(_)) => list.push(try!(self.parse_argument_type())),
            _ => return Ok(list),
        }

        // arglist ::= arg *
        //             arglist * "," arg
        while self.token() == Some(Token::Comma) {
            // arglist ::= arglist * "," arg
            self.consume();
            // arglist ::= arglist "," * arg
            list.push(try!(self.parse_argument_type()));
        }

        Ok(list)
    }

    // Parse a single argument type with flags.
    fn parse_argument_type(&mut self) -> Result<types::ArgumentType> {
        // arg ::= * type
        //         * arg flag
        let mut arg = match self.token() {
            Some(Token::Type(t)) => types::ArgumentType::new(t),
            _ => return Err(self.error("Expected argument type")),
        };
        loop {
            self.consume();
            // arg ::= arg * flag
            match self.token() {
                Some(Token::Identifier(s)) => {
                    match s {
                        "uext" => arg.extension = types::ArgumentExtension::Uext,
                        "sext" => arg.extension = types::ArgumentExtension::Sext,
                        "inreg" => arg.inreg = true,
                        _ => break,
                    }
                }
                _ => break,
            }
        }
        Ok(arg)
    }
}

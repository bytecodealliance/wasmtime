//! Parser for ISLE language.

use crate::ast::*;
use crate::error::*;
use crate::lexer::{Lexer, Pos, Token};

#[derive(Clone, Debug)]
pub struct Parser<'a> {
    lexer: Lexer<'a>,
}

pub type ParseResult<T> = std::result::Result<T, ParseError>;

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer<'a>) -> Parser<'a> {
        Parser { lexer }
    }

    pub fn error(&self, pos: Pos, msg: String) -> ParseError {
        ParseError {
            filename: self.lexer.filenames[pos.file].clone(),
            pos,
            msg,
        }
    }

    fn take<F: Fn(&Token) -> bool>(&mut self, f: F) -> ParseResult<Token> {
        if let Some(&(pos, ref peek)) = self.lexer.peek() {
            if !f(peek) {
                return Err(self.error(pos, format!("Unexpected token {:?}", peek)));
            }
            Ok(self.lexer.next().unwrap().1)
        } else {
            Err(self.error(self.lexer.pos(), "Unexpected EOF".to_string()))
        }
    }

    fn is<F: Fn(&Token) -> bool>(&self, f: F) -> bool {
        if let Some(&(_, ref peek)) = self.lexer.peek() {
            f(peek)
        } else {
            false
        }
    }

    fn pos(&self) -> Option<Pos> {
        self.lexer.peek().map(|(pos, _)| *pos)
    }

    fn is_lparen(&self) -> bool {
        self.is(|tok| *tok == Token::LParen)
    }
    fn is_rparen(&self) -> bool {
        self.is(|tok| *tok == Token::RParen)
    }
    fn is_at(&self) -> bool {
        self.is(|tok| *tok == Token::At)
    }
    fn is_lt(&self) -> bool {
        self.is(|tok| *tok == Token::Lt)
    }
    fn is_sym(&self) -> bool {
        self.is(|tok| tok.is_sym())
    }
    fn is_int(&self) -> bool {
        self.is(|tok| tok.is_int())
    }
    fn is_sym_str(&self, s: &str) -> bool {
        self.is(|tok| match tok {
            &Token::Symbol(ref tok_s) if tok_s == s => true,
            _ => false,
        })
    }

    fn lparen(&mut self) -> ParseResult<()> {
        self.take(|tok| *tok == Token::LParen).map(|_| ())
    }
    fn rparen(&mut self) -> ParseResult<()> {
        self.take(|tok| *tok == Token::RParen).map(|_| ())
    }
    fn at(&mut self) -> ParseResult<()> {
        self.take(|tok| *tok == Token::At).map(|_| ())
    }
    fn lt(&mut self) -> ParseResult<()> {
        self.take(|tok| *tok == Token::Lt).map(|_| ())
    }

    fn symbol(&mut self) -> ParseResult<String> {
        match self.take(|tok| tok.is_sym())? {
            Token::Symbol(s) => Ok(s),
            _ => unreachable!(),
        }
    }

    fn int(&mut self) -> ParseResult<i64> {
        match self.take(|tok| tok.is_int())? {
            Token::Int(i) => Ok(i),
            _ => unreachable!(),
        }
    }

    pub fn parse_defs(&mut self) -> ParseResult<Defs> {
        let mut defs = vec![];
        while !self.lexer.eof() {
            defs.push(self.parse_def()?);
        }
        Ok(Defs {
            defs,
            filenames: self.lexer.filenames.clone(),
        })
    }

    fn parse_def(&mut self) -> ParseResult<Def> {
        self.lparen()?;
        let pos = self.pos();
        let def = match &self.symbol()?[..] {
            "type" => Def::Type(self.parse_type()?),
            "decl" => Def::Decl(self.parse_decl()?),
            "rule" => Def::Rule(self.parse_rule()?),
            "extractor" => Def::Extractor(self.parse_etor()?),
            "extern" => Def::Extern(self.parse_extern()?),
            s => {
                return Err(self.error(pos.unwrap(), format!("Unexpected identifier: {}", s)));
            }
        };
        self.rparen()?;
        Ok(def)
    }

    fn str_to_ident(&self, pos: Pos, s: &str) -> ParseResult<Ident> {
        let first = s.chars().next().unwrap();
        if !first.is_alphabetic() && first != '_' {
            return Err(self.error(
                pos,
                format!("Identifier '{}' does not start with letter or _", s),
            ));
        }
        if s.chars()
            .skip(1)
            .any(|c| !c.is_alphanumeric() && c != '_' && c != '.')
        {
            return Err(self.error(
                pos,
                format!(
                    "Identifier '{}' contains invalid character (not a-z, A-Z, 0-9, _, .)",
                    s
                ),
            ));
        }
        Ok(Ident(s.to_string()))
    }

    fn parse_ident(&mut self) -> ParseResult<Ident> {
        let pos = self.pos();
        let s = self.symbol()?;
        self.str_to_ident(pos.unwrap(), &s)
    }

    fn parse_type(&mut self) -> ParseResult<Type> {
        let pos = self.pos();
        let name = self.parse_ident()?;
        let mut is_extern = false;
        if self.is_sym_str("extern") {
            self.symbol()?;
            is_extern = true;
        }
        let ty = self.parse_typevalue()?;
        Ok(Type {
            name,
            is_extern,
            ty,
            pos: pos.unwrap(),
        })
    }

    fn parse_typevalue(&mut self) -> ParseResult<TypeValue> {
        let pos = self.pos();
        self.lparen()?;
        if self.is_sym_str("primitive") {
            self.symbol()?;
            let primitive_ident = self.parse_ident()?;
            self.rparen()?;
            Ok(TypeValue::Primitive(primitive_ident))
        } else if self.is_sym_str("enum") {
            self.symbol()?;
            let mut variants = vec![];
            while !self.is_rparen() {
                let variant = self.parse_type_variant()?;
                variants.push(variant);
            }
            self.rparen()?;
            Ok(TypeValue::Enum(variants))
        } else {
            Err(self.error(pos.unwrap(), "Unknown type definition".to_string()))
        }
    }

    fn parse_type_variant(&mut self) -> ParseResult<Variant> {
        if self.is_sym() {
            let name = self.parse_ident()?;
            Ok(Variant {
                name,
                fields: vec![],
            })
        } else {
            self.lparen()?;
            let name = self.parse_ident()?;
            let mut fields = vec![];
            while !self.is_rparen() {
                fields.push(self.parse_type_field()?);
            }
            self.rparen()?;
            Ok(Variant { name, fields })
        }
    }

    fn parse_type_field(&mut self) -> ParseResult<Field> {
        self.lparen()?;
        let name = self.parse_ident()?;
        let ty = self.parse_ident()?;
        self.rparen()?;
        Ok(Field { name, ty })
    }

    fn parse_decl(&mut self) -> ParseResult<Decl> {
        let pos = self.pos();
        let term = self.parse_ident()?;

        self.lparen()?;
        let mut arg_tys = vec![];
        while !self.is_rparen() {
            arg_tys.push(self.parse_ident()?);
        }
        self.rparen()?;

        let ret_ty = self.parse_ident()?;

        Ok(Decl {
            term,
            arg_tys,
            ret_ty,
            pos: pos.unwrap(),
        })
    }

    fn parse_extern(&mut self) -> ParseResult<Extern> {
        let pos = self.pos();
        if self.is_sym_str("constructor") {
            self.symbol()?;
            let term = self.parse_ident()?;
            let func = self.parse_ident()?;
            Ok(Extern::Constructor {
                term,
                func,
                pos: pos.unwrap(),
            })
        } else if self.is_sym_str("extractor") {
            self.symbol()?;

            let infallible = if self.is_sym_str("infallible") {
                self.symbol()?;
                true
            } else {
                false
            };

            let term = self.parse_ident()?;
            let func = self.parse_ident()?;

            let arg_polarity = if self.is_lparen() {
                let mut pol = vec![];
                self.lparen()?;
                while !self.is_rparen() {
                    if self.is_sym_str("in") {
                        self.symbol()?;
                        pol.push(ArgPolarity::Input);
                    } else if self.is_sym_str("out") {
                        self.symbol()?;
                        pol.push(ArgPolarity::Output);
                    } else {
                        return Err(
                            self.error(pos.unwrap(), "Invalid argument polarity".to_string())
                        );
                    }
                }
                self.rparen()?;
                Some(pol)
            } else {
                None
            };
            Ok(Extern::Extractor {
                term,
                func,
                pos: pos.unwrap(),
                arg_polarity,
                infallible,
            })
        } else {
            Err(self.error(
                pos.unwrap(),
                "Invalid extern: must be (extern constructor ...) or (extern extractor ...)"
                    .to_string(),
            ))
        }
    }

    fn parse_etor(&mut self) -> ParseResult<Extractor> {
        let pos = self.pos();
        self.lparen()?;
        let term = self.parse_ident()?;
        let mut args = vec![];
        while !self.is_rparen() {
            args.push(self.parse_ident()?);
        }
        self.rparen()?;
        let template = self.parse_pattern()?;
        Ok(Extractor {
            term,
            args,
            template,
            pos: pos.unwrap(),
        })
    }

    fn parse_rule(&mut self) -> ParseResult<Rule> {
        let pos = self.pos();
        let prio = if self.is_int() {
            Some(self.int()?)
        } else {
            None
        };
        let pattern = self.parse_pattern()?;
        let expr = self.parse_expr()?;
        Ok(Rule {
            pattern,
            expr,
            pos: pos.unwrap(),
            prio,
        })
    }

    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        let pos = self.pos();
        if self.is_int() {
            Ok(Pattern::ConstInt { val: self.int()? })
        } else if self.is_sym_str("_") {
            self.symbol()?;
            Ok(Pattern::Wildcard)
        } else if self.is_sym() {
            let s = self.symbol()?;
            if s.starts_with("=") {
                let s = &s[1..];
                let var = self.str_to_ident(pos.unwrap(), s)?;
                Ok(Pattern::Var { var })
            } else {
                let var = self.str_to_ident(pos.unwrap(), &s)?;
                if self.is_at() {
                    self.at()?;
                    let subpat = Box::new(self.parse_pattern()?);
                    Ok(Pattern::BindPattern { var, subpat })
                } else {
                    Ok(Pattern::BindPattern {
                        var,
                        subpat: Box::new(Pattern::Wildcard),
                    })
                }
            }
        } else if self.is_lparen() {
            self.lparen()?;
            if self.is_sym_str("and") {
                self.symbol()?;
                let mut subpats = vec![];
                while !self.is_rparen() {
                    subpats.push(self.parse_pattern()?);
                }
                self.rparen()?;
                Ok(Pattern::And { subpats })
            } else {
                let sym = self.parse_ident()?;
                let mut args = vec![];
                while !self.is_rparen() {
                    args.push(self.parse_pattern_term_arg()?);
                }
                self.rparen()?;
                Ok(Pattern::Term { sym, args })
            }
        } else {
            Err(self.error(pos.unwrap(), "Unexpected pattern".into()))
        }
    }

    fn parse_pattern_term_arg(&mut self) -> ParseResult<TermArgPattern> {
        if self.is_lt() {
            self.lt()?;
            Ok(TermArgPattern::Expr(self.parse_expr()?))
        } else {
            Ok(TermArgPattern::Pattern(self.parse_pattern()?))
        }
    }

    fn parse_expr(&mut self) -> ParseResult<Expr> {
        let pos = self.pos();
        if self.is_lparen() {
            self.lparen()?;
            if self.is_sym_str("let") {
                self.symbol()?;
                self.lparen()?;
                let mut defs = vec![];
                while !self.is_rparen() {
                    let def = self.parse_letdef()?;
                    defs.push(def);
                }
                self.rparen()?;
                let body = Box::new(self.parse_expr()?);
                self.rparen()?;
                Ok(Expr::Let { defs, body })
            } else {
                let sym = self.parse_ident()?;
                let mut args = vec![];
                while !self.is_rparen() {
                    args.push(self.parse_expr()?);
                }
                self.rparen()?;
                Ok(Expr::Term { sym, args })
            }
        } else if self.is_sym_str("#t") {
            self.symbol()?;
            Ok(Expr::ConstInt { val: 1 })
        } else if self.is_sym_str("#f") {
            self.symbol()?;
            Ok(Expr::ConstInt { val: 0 })
        } else if self.is_sym() {
            let name = self.parse_ident()?;
            Ok(Expr::Var { name })
        } else if self.is_int() {
            let val = self.int()?;
            Ok(Expr::ConstInt { val })
        } else {
            Err(self.error(pos.unwrap(), "Invalid expression".into()))
        }
    }

    fn parse_letdef(&mut self) -> ParseResult<LetDef> {
        self.lparen()?;
        let var = self.parse_ident()?;
        let ty = self.parse_ident()?;
        let val = Box::new(self.parse_expr()?);
        self.rparen()?;
        Ok(LetDef { var, ty, val })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_type() {
        let text = r"
            ;; comment
            (type Inst extern (enum
              (Alu (a Reg) (b Reg) (dest Reg))
              (Load (a Reg) (dest Reg))))
            (type u32 (primitive u32))
            ";
        let defs = Parser::new(Lexer::from_str(text, "(none)"))
            .parse_defs()
            .expect("should parse");
        assert_eq!(
            defs,
            Defs {
                filenames: vec!["(none)".to_string()],
                defs: vec![
                    Def::Type(Type {
                        name: Ident("Inst".to_string()),
                        is_extern: true,
                        ty: TypeValue::Enum(vec![
                            Variant {
                                name: Ident("Alu".to_string()),
                                fields: vec![
                                    Field {
                                        name: Ident("a".to_string()),
                                        ty: Ident("Reg".to_string()),
                                    },
                                    Field {
                                        name: Ident("b".to_string()),
                                        ty: Ident("Reg".to_string()),
                                    },
                                    Field {
                                        name: Ident("dest".to_string()),
                                        ty: Ident("Reg".to_string()),
                                    },
                                ],
                            },
                            Variant {
                                name: Ident("Load".to_string()),
                                fields: vec![
                                    Field {
                                        name: Ident("a".to_string()),
                                        ty: Ident("Reg".to_string()),
                                    },
                                    Field {
                                        name: Ident("dest".to_string()),
                                        ty: Ident("Reg".to_string()),
                                    },
                                ],
                            }
                        ]),
                        pos: Pos {
                            file: 0,
                            offset: 42,
                            line: 3,
                            col: 18,
                        },
                    }),
                    Def::Type(Type {
                        name: Ident("u32".to_string()),
                        is_extern: false,
                        ty: TypeValue::Primitive(Ident("u32".to_string())),
                        pos: Pos {
                            file: 0,
                            offset: 167,
                            line: 6,
                            col: 18,
                        },
                    }),
                ]
            }
        );
    }
}

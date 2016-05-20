
// ====--------------------------------------------------------------------------------------====//
//
// Parser for .cton files.
//
// ====--------------------------------------------------------------------------------------====//

use std::collections::HashMap;
use std::result;
use std::fmt::{self, Display, Formatter, Write};
use std::u32;
use lexer::{self, Lexer, Token};
use cretonne::types::{Type, VOID, FunctionName, Signature, ArgumentType, ArgumentExtension};
use cretonne::immediates::{Imm64, Ieee32, Ieee64};
use cretonne::entities::*;
use cretonne::instructions::{Opcode, InstructionFormat, InstructionData};
use cretonne::repr::{Function, StackSlotData};

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

// Context for resolving references when parsing a single function.
//
// Many entities like values, stack slots, and function signatures are referenced in the `.cton`
// file by number. We need to map these numbers to real references.
struct Context {
    function: Function,
    stack_slots: HashMap<u32, StackSlot>, // ssNN
    ebbs: HashMap<u32, Ebb>, // ebbNN
    value_directs: HashMap<u32, Value>, // vNN
    value_tables: HashMap<u32, Value>, // vxNN
}

impl Context {
    fn new(f: Function) -> Context {
        Context {
            function: f,
            stack_slots: HashMap::new(),
            ebbs: HashMap::new(),
            value_directs: HashMap::new(),
            value_tables: HashMap::new(),
        }
    }

    // Allocate a new stack slot and add a mapping number -> StackSlot.
    fn add_ss(&mut self, number: u32, data: StackSlotData, loc: &Location) -> Result<()> {
        if self.stack_slots.insert(number, self.function.make_stack_slot(data)).is_some() {
            Err(Error {
                location: loc.clone(),
                message: format!("duplicate stack slot: ss{}", number),
            })
        } else {
            Ok(())
        }
    }

    // Allocate a new EBB and add a mapping number -> Ebb.
    fn add_ebb(&mut self, number: u32, loc: &Location) -> Result<Ebb> {
        let ebb = self.function.make_ebb();
        if self.ebbs.insert(number, ebb).is_some() {
            Err(Error {
                location: loc.clone(),
                message: format!("duplicate EBB: ebb{}", number),
            })
        } else {
            Ok(ebb)
        }
    }

    // Add a value mapping number -> data for a direct value (vNN).
    fn add_v(&mut self, number: u32, data: Value, loc: &Location) -> Result<()> {
        if self.value_directs.insert(number, data).is_some() {
            Err(Error {
                location: loc.clone(),
                message: format!("duplicate value: v{}", number),
            })
        } else {
            Ok(())
        }
    }

    // Add a value mapping number -> data for a table value (vxNN).
    fn add_vx(&mut self, number: u32, data: Value, loc: &Location) -> Result<()> {
        if self.value_tables.insert(number, data).is_some() {
            Err(Error {
                location: loc.clone(),
                message: format!("duplicate value: vx{}", number),
            })
        } else {
            Ok(())
        }
    }
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
    pub fn parse(text: &'a str) -> Result<Vec<Function>> {
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

    // Match and consume a specific identifier string.
    // Used for pseudo-keywords like "stack_slot" that only appear in certain contexts.
    fn match_identifier(&mut self, want: &'static str, err_msg: &str) -> Result<Token<'a>> {
        if self.token() == Some(Token::Identifier(want)) {
            Ok(self.consume())
        } else {
            Err(self.error(err_msg))
        }
    }

    // Match and consume a type.
    fn match_type(&mut self, err_msg: &str) -> Result<Type> {
        if let Some(Token::Type(t)) = self.token() {
            self.consume();
            Ok(t)
        } else {
            Err(self.error(err_msg))
        }
    }

    // Match and consume a stack slot reference.
    fn match_ss(&mut self, err_msg: &str) -> Result<u32> {
        if let Some(Token::StackSlot(ss)) = self.token() {
            self.consume();
            Ok(ss)
        } else {
            Err(self.error(err_msg))
        }
    }

    // Match and consume an ebb reference.
    fn match_ebb(&mut self, err_msg: &str) -> Result<u32> {
        if let Some(Token::Ebb(ebb)) = self.token() {
            self.consume();
            Ok(ebb)
        } else {
            Err(self.error(err_msg))
        }
    }

    // Match and consume a vx reference.
    fn match_vx(&mut self, err_msg: &str) -> Result<u32> {
        if let Some(Token::ValueTable(vx)) = self.token() {
            self.consume();
            Ok(vx)
        } else {
            Err(self.error(err_msg))
        }
    }

    // Match and consume a value reference, direct or vtable.
    // This does not convert from the source value numbering to our in-memory value numbering.
    fn match_value(&mut self, err_msg: &str) -> Result<Value> {
        let val = match self.token() {
            Some(Token::ValueDirect(v)) => Value::direct_from_number(v),
            Some(Token::ValueTable(vx)) => Value::new_table(vx as usize),
            _ => return Err(self.error(err_msg)),
        };
        self.consume();
        Ok(val)
    }

    // Match and consume an Imm64 immediate.
    fn match_imm64(&mut self, err_msg: &str) -> Result<Imm64> {
        if let Some(Token::Integer(text)) = self.token() {
            self.consume();
            // Lexer just gives us raw text that looks like an integer.
            // Parse it as an Imm64 to check for overflow and other issues.
            text.parse().map_err(|e| self.error(e))
        } else {
            Err(self.error(err_msg))
        }
    }

    // Match and consume an Ieee32 immediate.
    fn match_ieee32(&mut self, err_msg: &str) -> Result<Ieee32> {
        if let Some(Token::Float(text)) = self.token() {
            self.consume();
            // Lexer just gives us raw text that looks like a float.
            // Parse it as an Ieee32 to check for the right number of digits and other issues.
            text.parse().map_err(|e| self.error(e))
        } else {
            Err(self.error(err_msg))
        }
    }

    // Match and consume an Ieee64 immediate.
    fn match_ieee64(&mut self, err_msg: &str) -> Result<Ieee64> {
        if let Some(Token::Float(text)) = self.token() {
            self.consume();
            // Lexer just gives us raw text that looks like a float.
            // Parse it as an Ieee64 to check for the right number of digits and other issues.
            text.parse().map_err(|e| self.error(e))
        } else {
            Err(self.error(err_msg))
        }
    }

    /// Parse a list of function definitions.
    ///
    /// This is the top-level parse function matching the whole contents of a file.
    pub fn parse_function_list(&mut self) -> Result<Vec<Function>> {
        let mut list = Vec::new();
        while self.token().is_some() {
            list.push(try!(self.parse_function()));
        }
        Ok(list)
    }

    // Parse a whole function definition.
    //
    // function ::= * function-spec "{" preamble function-body "}"
    //
    fn parse_function(&mut self) -> Result<Function> {
        let (name, sig) = try!(self.parse_function_spec());
        let mut ctx = Context::new(Function::with_name_signature(name, sig));

        // function ::= function-spec * "{" preamble function-body "}"
        try!(self.match_token(Token::LBrace, "expected '{' before function body"));
        // function ::= function-spec "{" * preamble function-body "}"
        try!(self.parse_preamble(&mut ctx));
        // function ::= function-spec "{"  preamble * function-body "}"
        try!(self.parse_function_body(&mut ctx));
        // function ::= function-spec "{" preamble function-body * "}"
        try!(self.match_token(Token::RBrace, "expected '}' after function body"));

        Ok(ctx.function)
    }

    // Parse a function spec.
    //
    // function-spec ::= * "function" name signature
    //
    fn parse_function_spec(&mut self) -> Result<(FunctionName, Signature)> {
        try!(self.match_token(Token::Function, "expected 'function' keyword"));

        // function-spec ::= "function" * name signature
        let name = try!(self.parse_function_name());

        // function-spec ::= "function" name * signature
        let sig = try!(self.parse_signature());

        Ok((name, sig))
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
    fn parse_signature(&mut self) -> Result<Signature> {
        let mut sig = Signature::new();

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
    fn parse_argument_list(&mut self) -> Result<Vec<ArgumentType>> {
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
    fn parse_argument_type(&mut self) -> Result<ArgumentType> {
        // arg ::= * type { flag }
        let mut arg = ArgumentType::new(try!(self.match_type("expected argument type")));

        // arg ::= type * { flag }
        while let Some(Token::Identifier(s)) = self.token() {
            match s {
                "uext" => arg.extension = ArgumentExtension::Uext,
                "sext" => arg.extension = ArgumentExtension::Sext,
                "inreg" => arg.inreg = true,
                _ => break,
            }
            self.consume();
        }

        Ok(arg)
    }

    // Parse the function preamble.
    //
    // preamble      ::= * { preamble-decl }
    // preamble-decl ::= * stack-slot-decl
    //                   * function-decl
    //                   * signature-decl
    //
    // The parsed decls are added to `ctx` rather than returned.
    fn parse_preamble(&mut self, ctx: &mut Context) -> Result<()> {
        loop {
            try!(match self.token() {
                Some(Token::StackSlot(..)) => {
                    self.parse_stack_slot_decl()
                        .and_then(|(num, dat)| ctx.add_ss(num, dat, &self.location))
                }
                // More to come..
                _ => return Ok(()),
            });
        }
    }

    // Parse a stack slot decl, add to `func`.
    //
    // stack-slot-decl ::= * StackSlot(ss) "=" "stack_slot" Bytes {"," stack-slot-flag}
    fn parse_stack_slot_decl(&mut self) -> Result<(u32, StackSlotData)> {
        let number = try!(self.match_ss("expected stack slot number: ss«n»"));
        try!(self.match_token(Token::Equal, "expected '=' in stack_slot decl"));
        try!(self.match_identifier("stack_slot", "expected 'stack_slot'"));

        // stack-slot-decl ::= StackSlot(ss) "=" "stack_slot" * Bytes {"," stack-slot-flag}
        let bytes = try!(self.match_imm64("expected byte-size in stack_slot decl")).to_bits();
        if bytes > u32::MAX as u64 {
            return Err(self.error("stack slot too large"));
        }
        let data = StackSlotData::new(bytes as u32);

        // TBD: stack-slot-decl ::= StackSlot(ss) "=" "stack_slot" Bytes * {"," stack-slot-flag}
        Ok((number, data))
    }

    // Parse a function body, add contents to `ctx`.
    //
    // function-body ::= * { extended-basic-block }
    //
    fn parse_function_body(&mut self, ctx: &mut Context) -> Result<()> {
        while self.token() != Some(Token::RBrace) {
            try!(self.parse_extended_basic_block(ctx));
        }
        Ok(())
    }

    // Parse an extended basic block, add contents to `ctx`.
    //
    // extended-basic-block ::= * ebb-header { instruction }
    // ebb-header           ::= ["entry"] Ebb(ebb) [ebb-args] ":"
    //
    fn parse_extended_basic_block(&mut self, ctx: &mut Context) -> Result<()> {
        let is_entry = self.optional(Token::Entry);
        let ebb_num = try!(self.match_ebb("expected EBB header"));
        let ebb = try!(ctx.add_ebb(ebb_num, &self.location));

        if is_entry {
            if ctx.function.entry_block != NO_EBB {
                return Err(self.error("multiple entry blocks in function"));
            }
            ctx.function.entry_block = ebb;
        }

        if !self.optional(Token::Colon) {
            // ebb-header ::= ["entry"] Ebb(ebb) [ * ebb-args ] ":"
            try!(self.parse_ebb_args(ctx, ebb));
            try!(self.match_token(Token::Colon, "expected ':' after EBB arguments"));
        }

        // extended-basic-block ::= ebb-header * { instruction }
        while match self.token() {
            Some(Token::ValueDirect(_)) => true,
            Some(Token::Identifier(_)) => true,
            _ => false,
        } {
            try!(self.parse_instruction(ctx, ebb));
        }

        Ok(())
    }

    // Parse parenthesized list of EBB arguments. Returns a vector of (u32, Type) pairs with the
    // source vx numbers of the defined values and the defined types.
    //
    // ebb-args ::= * "(" ebb-arg { "," ebb-arg } ")"
    fn parse_ebb_args(&mut self, ctx: &mut Context, ebb: Ebb) -> Result<()> {
        // ebb-args ::= * "(" ebb-arg { "," ebb-arg } ")"
        try!(self.match_token(Token::LPar, "expected '(' before EBB arguments"));

        // ebb-args ::= "(" * ebb-arg { "," ebb-arg } ")"
        try!(self.parse_ebb_arg(ctx, ebb));

        // ebb-args ::= "(" ebb-arg * { "," ebb-arg } ")"
        while self.optional(Token::Comma) {
            // ebb-args ::= "(" ebb-arg { "," * ebb-arg } ")"
            try!(self.parse_ebb_arg(ctx, ebb));
        }

        // ebb-args ::= "(" ebb-arg { "," ebb-arg } * ")"
        try!(self.match_token(Token::RPar, "expected ')' after EBB arguments"));

        Ok(())
    }

    // Parse a single EBB argument declaration, and append it to `ebb`.
    //
    // ebb-arg ::= * ValueTable(vx) ":" Type(t)
    //
    fn parse_ebb_arg(&mut self, ctx: &mut Context, ebb: Ebb) -> Result<()> {
        // ebb-arg ::= * ValueTable(vx) ":" Type(t)
        let vx = try!(self.match_vx("EBB argument must be a vx value"));
        let vx_location = self.location;
        // ebb-arg ::= ValueTable(vx) * ":" Type(t)
        try!(self.match_token(Token::Colon, "expected ':' after EBB argument"));
        // ebb-arg ::= ValueTable(vx) ":" * Type(t)
        let t = try!(self.match_type("expected EBB argument type"));
        // Allocate the EBB argument and add the mapping.
        let value = ctx.function.append_ebb_arg(ebb, t);
        ctx.add_vx(vx, value, &vx_location)
    }

    // Parse an instruction, append it to `ebb`.
    //
    // instruction ::= [inst-results "="] Opcode(opc) ...
    // inst-results ::= ValueDirect(v) { "," ValueTable(vx) }
    //
    fn parse_instruction(&mut self, ctx: &mut Context, ebb: Ebb) -> Result<()> {
        // Result value numbers. First is a ValueDirect, remaining are ValueTable.
        let mut results = Vec::new();

        // instruction ::=  * [inst-results "="] Opcode(opc) ...
        if let Some(Token::ValueDirect(v)) = self.token() {
            self.consume();
            results.push(v);

            // inst-results ::= ValueDirect(v) * { "," ValueTable(vx) }
            while self.optional(Token::Comma) {
                // inst-results ::= ValueDirect(v) { "," * ValueTable(vx) }
                results.push(try!(self.match_vx("expected vx result value")));
            }

            try!(self.match_token(Token::Equal, "expected '=' before opcode"));
        }

        // instruction ::=  [inst-results "="] * Opcode(opc) ...
        let opcode = if let Some(Token::Identifier(text)) = self.token() {
            match text.parse() {
                Ok(opc) => opc,
                Err(msg) => return Err(self.error(msg)),
            }
        } else {
            return Err(self.error("expected instruction opcode"));
        };

        // instruction ::=  [inst-results "="] Opcode(opc) * ...
        let inst_data = try!(self.parse_inst_operands(opcode));
        let inst = ctx.function.make_inst(inst_data);

        // TODO: Check that results.len() matches the opcode.
        // TODO: Multiple results.
        if !results.is_empty() {
            assert!(results.len() == 1, "Multiple results not implemented");
            let result = ctx.function.first_result(inst);
            try!(ctx.add_v(results[0], result, &self.location));
        }

        ctx.function.append_inst(ebb, inst);

        Ok(())
    }

    // Parse the operands following the instruction opcode.
    // This depends on the format of the opcode.
    fn parse_inst_operands(&mut self, opcode: Opcode) -> Result<InstructionData> {
        Ok(match opcode.format().unwrap() {
            InstructionFormat::Nullary => {
                InstructionData::Nullary {
                    opcode: opcode,
                    ty: VOID,
                }
            }
            InstructionFormat::Unary => {
                InstructionData::Unary {
                    opcode: opcode,
                    ty: VOID,
                    arg: try!(self.match_value("expected SSA value operand")),
                }
            }
            InstructionFormat::UnaryImm => {
                InstructionData::UnaryImm {
                    opcode: opcode,
                    ty: VOID,
                    imm: try!(self.match_imm64("expected immediate integer operand")),
                }
            }
            InstructionFormat::UnaryIeee32 => {
                InstructionData::UnaryIeee32 {
                    opcode: opcode,
                    ty: VOID,
                    imm: try!(self.match_ieee32("expected immediate 32-bit float operand")),
                }
            }
            InstructionFormat::UnaryIeee64 => {
                InstructionData::UnaryIeee64 {
                    opcode: opcode,
                    ty: VOID,
                    imm: try!(self.match_ieee64("expected immediate 64-bit float operand")),
                }
            }
            InstructionFormat::UnaryImmVector => {
                unimplemented!();
            }
            InstructionFormat::Binary => {
                let lhs = try!(self.match_value("expected SSA value first operand"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let rhs = try!(self.match_value("expected SSA value second operand"));
                InstructionData::Binary {
                    opcode: opcode,
                    ty: VOID,
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::BinaryImm => {
                let lhs = try!(self.match_value("expected SSA value first operand"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let rhs = try!(self.match_imm64("expected immediate integer second operand"));
                InstructionData::BinaryImm {
                    opcode: opcode,
                    ty: VOID,
                    lhs: lhs,
                    rhs: rhs,
                }
            }
            InstructionFormat::BinaryImmRev => {
                let lhs = try!(self.match_imm64("expected immediate integer first operand"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let rhs = try!(self.match_value("expected SSA value second operand"));
                InstructionData::BinaryImmRev {
                    opcode: opcode,
                    ty: VOID,
                    lhs: lhs,
                    rhs: rhs,
                }
            }
            InstructionFormat::BinaryOverflow => {
                let lhs = try!(self.match_value("expected SSA value first operand"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let rhs = try!(self.match_value("expected SSA value second operand"));
                InstructionData::BinaryOverflow {
                    opcode: opcode,
                    ty: VOID,
                    second_result: NO_VALUE,
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::Select |
            InstructionFormat::InsertLane |
            InstructionFormat::ExtractLane |
            InstructionFormat::Jump |
            InstructionFormat::Branch |
            InstructionFormat::BranchTable |
            InstructionFormat::Call => {
                unimplemented!();
            }
        })
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
        assert_eq!(sig2.to_string(),
                   "(i8 uext inreg, f32, f64) -> i32 sext, f64");

        // `void` is not recognized as a type by the lexer. It should not appear in files.
        assert_eq!(Parser::new("() -> void").parse_signature().unwrap_err().to_string(),
                   "1: expected argument type");
        assert_eq!(Parser::new("i8 -> i8").parse_signature().unwrap_err().to_string(),
                   "1: expected function signature: ( args... )");
        assert_eq!(Parser::new("(i8 -> i8").parse_signature().unwrap_err().to_string(),
                   "1: expected ')' after function arguments");
    }

    #[test]
    fn stack_slot_decl() {
        let func = Parser::new("function foo() {
                                  ss3 = stack_slot 13
                                  ss1 = stack_slot 1
                                }")
                       .parse_function()
                       .unwrap();
        assert_eq!(func.name, "foo");
        let mut iter = func.stack_slot_iter();
        let ss0 = iter.next().unwrap();
        assert_eq!(ss0.to_string(), "ss0");
        assert_eq!(func[ss0].size, 13);
        let ss1 = iter.next().unwrap();
        assert_eq!(ss1.to_string(), "ss1");
        assert_eq!(func[ss1].size, 1);
        assert_eq!(iter.next(), None);

        // Catch duplicate definitions.
        assert_eq!(Parser::new("function bar() {
                                    ss1  = stack_slot 13
                                    ss1  = stack_slot 1
                                }")
                       .parse_function()
                       .unwrap_err()
                       .to_string(),
                   "3: duplicate stack slot: ss1");
    }

    #[test]
    fn ebb_header() {
        let func = Parser::new("function ebbs() {
                                ebb0:
                                ebb4(vx3: i32):
                                }")
                       .parse_function()
                       .unwrap();
        assert_eq!(func.name, "ebbs");

        let mut ebbs = func.ebbs_numerically();

        let ebb0 = ebbs.next().unwrap();
        assert_eq!(func.ebb_args(ebb0).next(), None);

        let ebb4 = ebbs.next().unwrap();
        let mut ebb4_args = func.ebb_args(ebb4);
        let arg0 = ebb4_args.next().unwrap();
        assert_eq!(func.value_type(arg0), types::I32);
        assert_eq!(ebb4_args.next(), None);
    }
}

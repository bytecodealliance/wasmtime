
// ====--------------------------------------------------------------------------------------====//
//
// Parser for .cton files.
//
// ====--------------------------------------------------------------------------------------====//

use std::collections::HashMap;
use std::result;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use std::u32;
use lexer::{self, Lexer, Token};
use cretonne::ir::{Function, Ebb, Inst, Opcode, Value, Type, FunctionName, StackSlotData,
                   JumpTable, StackSlot};
use cretonne::ir::types::{VOID, Signature, ArgumentType, ArgumentExtension};
use cretonne::ir::immediates::{Imm64, Ieee32, Ieee64};
use cretonne::ir::entities::{NO_EBB, NO_VALUE};
use cretonne::ir::instructions::{InstructionFormat, InstructionData, VariableArgs, JumpData,
                                 BranchData, ReturnData};
use cretonne::ir::jumptable::JumpTableData;

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

// Create an `Err` variant of `Result<X>` from a location and `format!` args.
macro_rules! err {
    ( $loc:expr, $msg:expr ) => {
        Err(Error {
            location: $loc.clone(),
            message: String::from($msg),
        })
    };

    ( $loc:expr, $fmt:expr, $( $arg:expr ),+ ) => {
        Err(Error {
            location: $loc.clone(),
            message: format!( $fmt, $( $arg ),+ ),
        })
    };
}

pub struct Parser<'a> {
    lex: Lexer<'a>,

    lex_error: Option<lexer::Error>,

    // Current lookahead token.
    lookahead: Option<Token<'a>>,

    // Location of lookahead.
    loc: Location,
}

// Context for resolving references when parsing a single function.
//
// Many entities like values, stack slots, and function signatures are referenced in the `.cton`
// file by number. We need to map these numbers to real references.
struct Context {
    function: Function,
    stack_slots: HashMap<u32, StackSlot>, // ssNN
    jump_tables: HashMap<u32, JumpTable>, // jtNN
    ebbs: HashMap<Ebb, Ebb>, // ebbNN
    values: HashMap<Value, Value>, // vNN, vxNN

    // Remember the location of every instruction.
    inst_locs: Vec<(Inst, Location)>,
}

impl Context {
    fn new(f: Function) -> Context {
        Context {
            function: f,
            stack_slots: HashMap::new(),
            jump_tables: HashMap::new(),
            ebbs: HashMap::new(),
            values: HashMap::new(),
            inst_locs: Vec::new(),
        }
    }

    // Allocate a new stack slot and add a mapping number -> StackSlot.
    fn add_ss(&mut self, number: u32, data: StackSlotData, loc: &Location) -> Result<()> {
        if self.stack_slots.insert(number, self.function.make_stack_slot(data)).is_some() {
            err!(loc, "duplicate stack slot: ss{}", number)
        } else {
            Ok(())
        }
    }

    // Allocate a new jump table and add a mapping number -> JumpTable.
    fn add_jt(&mut self, number: u32, data: JumpTableData, loc: &Location) -> Result<()> {
        if self.jump_tables.insert(number, self.function.jump_tables.push(data)).is_some() {
            err!(loc, "duplicate jump table: jt{}", number)
        } else {
            Ok(())
        }
    }

    // Resolve a reference to a jump table.
    fn get_jt(&self, number: u32, loc: &Location) -> Result<JumpTable> {
        match self.jump_tables.get(&number) {
            Some(&jt) => Ok(jt),
            None => err!(loc, "undefined jump table jt{}", number),
        }
    }

    // Allocate a new EBB and add a mapping src_ebb -> Ebb.
    fn add_ebb(&mut self, src_ebb: Ebb, loc: &Location) -> Result<Ebb> {
        let ebb = self.function.dfg.make_ebb();
        self.function.layout.append_ebb(ebb);
        if self.ebbs.insert(src_ebb, ebb).is_some() {
            err!(loc, "duplicate EBB: {}", src_ebb)
        } else {
            Ok(ebb)
        }
    }

    // Add a value mapping src_val -> data.
    fn add_value(&mut self, src_val: Value, data: Value, loc: &Location) -> Result<()> {
        if self.values.insert(src_val, data).is_some() {
            err!(loc, "duplicate value: {}", src_val)
        } else {
            Ok(())
        }
    }

    // Record the location of an instuction.
    fn add_inst_loc(&mut self, inst: Inst, loc: &Location) {
        self.inst_locs.push((inst, *loc));
    }

    // The parser creates all instructions with Ebb and Value references using the source file
    // numbering. These references need to be rewritten after parsing is complete since forward
    // references are allowed.

    // Rewrite an Ebb reference.
    fn rewrite_ebb(map: &HashMap<Ebb, Ebb>, ebb: &mut Ebb, loc: &Location) -> Result<()> {
        match map.get(ebb) {
            Some(&new) => {
                *ebb = new;
                Ok(())
            }
            None => err!(loc, "undefined reference: {}", ebb),
        }
    }

    // Rewrite a value reference.
    fn rewrite_value(map: &HashMap<Value, Value>, val: &mut Value, loc: &Location) -> Result<()> {
        match map.get(val) {
            Some(&new) => {
                *val = new;
                Ok(())
            }
            None => err!(loc, "undefined reference: {}", val),
        }
    }

    // Rewrite a slice of value references.
    fn rewrite_values(map: &HashMap<Value, Value>,
                      vals: &mut [Value],
                      loc: &Location)
                      -> Result<()> {
        for val in vals {
            try!(Self::rewrite_value(map, val, loc));
        }
        Ok(())
    }

    // Rewrite all EBB and value references in the function.
    fn rewrite_references(&mut self) -> Result<()> {
        for &(inst, loc) in &self.inst_locs {
            match self.function.dfg[inst] {
                InstructionData::Nullary { .. } |
                InstructionData::UnaryImm { .. } |
                InstructionData::UnaryIeee32 { .. } |
                InstructionData::UnaryIeee64 { .. } |
                InstructionData::UnaryImmVector { .. } => {}

                InstructionData::Unary { ref mut arg, .. } |
                InstructionData::BinaryImm { ref mut arg, .. } |
                InstructionData::BinaryImmRev { ref mut arg, .. } |
                InstructionData::ExtractLane { ref mut arg, .. } |
                InstructionData::BranchTable { ref mut arg, .. } => {
                    try!(Self::rewrite_value(&self.values, arg, &loc));
                }

                InstructionData::Binary { ref mut args, .. } |
                InstructionData::BinaryOverflow { ref mut args, .. } |
                InstructionData::InsertLane { ref mut args, .. } |
                InstructionData::IntCompare { ref mut args, .. } |
                InstructionData::FloatCompare { ref mut args, .. } => {
                    try!(Self::rewrite_values(&self.values, args, &loc));
                }

                InstructionData::Ternary { ref mut args, .. } => {
                    try!(Self::rewrite_values(&self.values, args, &loc));
                }

                InstructionData::Jump { ref mut data, .. } => {
                    try!(Self::rewrite_ebb(&self.ebbs, &mut data.destination, &loc));
                    try!(Self::rewrite_values(&self.values, &mut data.arguments, &loc));
                }

                InstructionData::Branch { ref mut data, .. } => {
                    try!(Self::rewrite_value(&self.values, &mut data.arg, &loc));
                    try!(Self::rewrite_ebb(&self.ebbs, &mut data.destination, &loc));
                    try!(Self::rewrite_values(&self.values, &mut data.arguments, &loc));
                }

                InstructionData::Call { ref mut data, .. } => {
                    try!(Self::rewrite_values(&self.values, &mut data.args, &loc));
                }

                InstructionData::Return { ref mut data, .. } => {
                    try!(Self::rewrite_values(&self.values, &mut data.args, &loc));
                }
            }
        }

        // Rewrite EBB references in jump tables.
        let loc = Location { line_number: 0 };
        for jt in self.function.jump_tables.keys() {
            for ebb in self.function.jump_tables[jt].as_mut_slice() {
                if *ebb != NO_EBB {
                    try!(Self::rewrite_ebb(&self.ebbs, ebb, &loc));
                }
            }
        }

        Ok(())
    }
}

impl<'a> Parser<'a> {
    /// Create a new `Parser` which reads `text`. The referenced text must outlive the parser.
    pub fn new(text: &'a str) -> Parser {
        Parser {
            lex: Lexer::new(text),
            lex_error: None,
            lookahead: None,
            loc: Location { line_number: 0 },
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
        while self.lookahead == None {
            match self.lex.next() {
                Some(Ok(lexer::LocatedToken { token, location })) => {
                    match token {
                        Token::Comment(_) => {
                            // Ignore comments.
                        }
                        _ => self.lookahead = Some(token),
                    }
                    self.loc = location;
                }
                Some(Err(lexer::LocatedError { error, location })) => {
                    self.lex_error = Some(error);
                    self.loc = location;
                    break;
                }
                None => break,
            }
        }
        return self.lookahead;
    }

    // Match and consume a token without payload.
    fn match_token(&mut self, want: Token<'a>, err_msg: &str) -> Result<Token<'a>> {
        if self.token() == Some(want) {
            Ok(self.consume())
        } else {
            err!(self.loc, err_msg)
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
            err!(self.loc, err_msg)
        }
    }

    // Match and consume a type.
    fn match_type(&mut self, err_msg: &str) -> Result<Type> {
        if let Some(Token::Type(t)) = self.token() {
            self.consume();
            Ok(t)
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume a stack slot reference.
    fn match_ss(&mut self, err_msg: &str) -> Result<u32> {
        if let Some(Token::StackSlot(ss)) = self.token() {
            self.consume();
            Ok(ss)
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume a jump table reference.
    fn match_jt(&mut self) -> Result<u32> {
        if let Some(Token::JumpTable(jt)) = self.token() {
            self.consume();
            Ok(jt)
        } else {
            err!(self.loc, "expected jump table number: jt«n»")
        }
    }

    // Match and consume an ebb reference.
    fn match_ebb(&mut self, err_msg: &str) -> Result<Ebb> {
        if let Some(Token::Ebb(ebb)) = self.token() {
            self.consume();
            Ok(ebb)
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume a value reference, direct or vtable.
    // This does not convert from the source value numbering to our in-memory value numbering.
    fn match_value(&mut self, err_msg: &str) -> Result<Value> {
        if let Some(Token::Value(v)) = self.token() {
            self.consume();
            Ok(v)
        } else {
            err!(self.loc, err_msg)
        }
    }

    fn error(&self, message: &str) -> Error {
        Error {
            location: self.loc.clone(),
            message: message.to_string(),
        }
    }

    // Match and consume an Imm64 immediate.
    fn match_imm64(&mut self, err_msg: &str) -> Result<Imm64> {
        if let Some(Token::Integer(text)) = self.token() {
            self.consume();
            // Lexer just gives us raw text that looks like an integer.
            // Parse it as an Imm64 to check for overflow and other issues.
            text.parse().map_err(|e| self.error(e))
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume a u8 immediate.
    // This is used for lane numbers in SIMD vectors.
    fn match_uimm8(&mut self, err_msg: &str) -> Result<u8> {
        if let Some(Token::Integer(text)) = self.token() {
            self.consume();
            // Lexer just gives us raw text that looks like an integer.
            // Parse it as a u8 to check for overflow and other issues.
            text.parse().map_err(|_| self.error("expected u8 decimal immediate"))
        } else {
            err!(self.loc, err_msg)
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
            err!(self.loc, err_msg)
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
            err!(self.loc, err_msg)
        }
    }

    // Match and consume an enumerated immediate, like one of the condition codes.
    fn match_enum<T: FromStr>(&mut self, err_msg: &str) -> Result<T> {
        if let Some(Token::Identifier(text)) = self.token() {
            self.consume();
            text.parse().map_err(|_| self.error(err_msg))
        } else {
            err!(self.loc, err_msg)
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

        // Rewrite references to values and EBBs after parsing everuthing to allow forward
        // references.
        try!(ctx.rewrite_references());

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
            _ => err!(self.loc, "expected function name"),
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
    //                   * jump-table-decl
    //
    // The parsed decls are added to `ctx` rather than returned.
    fn parse_preamble(&mut self, ctx: &mut Context) -> Result<()> {
        loop {
            try!(match self.token() {
                Some(Token::StackSlot(..)) => {
                    self.parse_stack_slot_decl()
                        .and_then(|(num, dat)| ctx.add_ss(num, dat, &self.loc))
                }
                Some(Token::JumpTable(..)) => {
                    self.parse_jump_table_decl()
                        .and_then(|(num, dat)| ctx.add_jt(num, dat, &self.loc))
                }
                // More to come..
                _ => return Ok(()),
            });
        }
    }

    // Parse a stack slot decl.
    //
    // stack-slot-decl ::= * StackSlot(ss) "=" "stack_slot" Bytes {"," stack-slot-flag}
    fn parse_stack_slot_decl(&mut self) -> Result<(u32, StackSlotData)> {
        let number = try!(self.match_ss("expected stack slot number: ss«n»"));
        try!(self.match_token(Token::Equal, "expected '=' in stack_slot decl"));
        try!(self.match_identifier("stack_slot", "expected 'stack_slot'"));

        // stack-slot-decl ::= StackSlot(ss) "=" "stack_slot" * Bytes {"," stack-slot-flag}
        let bytes = try!(self.match_imm64("expected byte-size in stack_slot decl")).to_bits();
        if bytes > u32::MAX as u64 {
            return err!(self.loc, "stack slot too large");
        }
        let data = StackSlotData::new(bytes as u32);

        // TBD: stack-slot-decl ::= StackSlot(ss) "=" "stack_slot" Bytes * {"," stack-slot-flag}
        Ok((number, data))
    }

    // Parse a jump table decl.
    //
    // jump-table-decl ::= * JumpTable(jt) "=" "jump_table" jt-entry {"," jt-entry}
    fn parse_jump_table_decl(&mut self) -> Result<(u32, JumpTableData)> {
        let number = try!(self.match_jt());
        try!(self.match_token(Token::Equal, "expected '=' in jump_table decl"));
        try!(self.match_identifier("jump_table", "expected 'jump_table'"));

        let mut data = JumpTableData::new();

        // jump-table-decl ::= JumpTable(jt) "=" "jump_table" * jt-entry {"," jt-entry}
        for idx in 0usize.. {
            if let Some(dest) = try!(self.parse_jump_table_entry()) {
                data.set_entry(idx, dest);
            }
            if !self.optional(Token::Comma) {
                return Ok((number, data));
            }
        }

        err!(self.loc, "jump_table too long")
    }

    // jt-entry ::= * Ebb(dest) | "0"
    fn parse_jump_table_entry(&mut self) -> Result<Option<Ebb>> {
        match self.token() {
            Some(Token::Integer(s)) => {
                if s == "0" {
                    self.consume();
                    Ok(None)
                } else {
                    err!(self.loc, "invalid jump_table entry '{}'", s)
                }
            }
            Some(Token::Ebb(dest)) => {
                self.consume();
                Ok(Some(dest))
            }
            _ => err!(self.loc, "expected jump_table entry"),
        }
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
    // ebb-header           ::= Ebb(ebb) [ebb-args] ":"
    //
    fn parse_extended_basic_block(&mut self, ctx: &mut Context) -> Result<()> {
        let ebb_num = try!(self.match_ebb("expected EBB header"));
        let ebb = try!(ctx.add_ebb(ebb_num, &self.loc));

        if !self.optional(Token::Colon) {
            // ebb-header ::= Ebb(ebb) [ * ebb-args ] ":"
            try!(self.parse_ebb_args(ctx, ebb));
            try!(self.match_token(Token::Colon, "expected ':' after EBB arguments"));
        }

        // extended-basic-block ::= ebb-header * { instruction }
        while match self.token() {
            Some(Token::Value(_)) => true,
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
    // ebb-arg ::= * Value(vx) ":" Type(t)
    //
    fn parse_ebb_arg(&mut self, ctx: &mut Context, ebb: Ebb) -> Result<()> {
        // ebb-arg ::= * Value(vx) ":" Type(t)
        let vx = try!(self.match_value("EBB argument must be a value"));
        let vx_location = self.loc;
        // ebb-arg ::= Value(vx) * ":" Type(t)
        try!(self.match_token(Token::Colon, "expected ':' after EBB argument"));
        // ebb-arg ::= Value(vx) ":" * Type(t)
        let t = try!(self.match_type("expected EBB argument type"));
        // Allocate the EBB argument and add the mapping.
        let value = ctx.function.dfg.append_ebb_arg(ebb, t);
        ctx.add_value(vx, value, &vx_location)
    }

    // Parse an instruction, append it to `ebb`.
    //
    // instruction ::= [inst-results "="] Opcode(opc) ["." Type] ...
    // inst-results ::= Value(v) { "," Value(vx) }
    //
    fn parse_instruction(&mut self, ctx: &mut Context, ebb: Ebb) -> Result<()> {
        // Result value numbers.
        let mut results = Vec::new();

        // instruction  ::=  * [inst-results "="] Opcode(opc) ["." Type] ...
        // inst-results ::= * Value(v) { "," Value(vx) }
        if let Some(Token::Value(v)) = self.token() {
            self.consume();
            results.push(v);

            // inst-results ::= Value(v) * { "," Value(vx) }
            while self.optional(Token::Comma) {
                // inst-results ::= Value(v) { "," * Value(vx) }
                results.push(try!(self.match_value("expected result value")));
            }

            try!(self.match_token(Token::Equal, "expected '=' before opcode"));
        }

        // instruction ::=  [inst-results "="] * Opcode(opc) ["." Type] ...
        let opcode = if let Some(Token::Identifier(text)) = self.token() {
            match text.parse() {
                Ok(opc) => opc,
                Err(msg) => return err!(self.loc, "{}: '{}'", msg, text),
            }
        } else {
            return err!(self.loc, "expected instruction opcode");
        };
        let opcode_loc = self.loc;
        self.consume();

        // Look for a controlling type variable annotation.
        // instruction ::=  [inst-results "="] Opcode(opc) * ["." Type] ...
        let explicit_ctrl_type = if self.optional(Token::Dot) {
            Some(try!(self.match_type("expected type after 'opcode.'")))
        } else {
            None
        };

        // instruction ::=  [inst-results "="] Opcode(opc) ["." Type] * ...
        let inst_data = try!(self.parse_inst_operands(ctx, opcode));

        // We're done parsing the instruction now.
        //
        // We still need to check that the number of result values in the source matches the opcode
        // or function call signature. We also need to create values with the right type for all
        // the instruction results.
        let ctrl_typevar = try!(self.infer_typevar(ctx, opcode, explicit_ctrl_type, &inst_data));
        let inst = ctx.function.dfg.make_inst(inst_data);
        let num_results = ctx.function.dfg.make_inst_results(inst, ctrl_typevar);
        ctx.function.layout.append_inst(inst, ebb);
        ctx.add_inst_loc(inst, &opcode_loc);

        if results.len() != num_results {
            return err!(self.loc,
                        "instruction produces {} result values, {} given",
                        num_results,
                        results.len());
        }

        // Now map the source result values to the just created instruction results.
        // Pass a reference to `ctx.values` instead of `ctx` itself since the `Values` iterator
        // holds a reference to `ctx.function`.
        self.add_values(&mut ctx.values,
                        results.into_iter(),
                        ctx.function.dfg.inst_results(inst))
    }

    // Type inference for polymorphic instructions.
    //
    // The controlling type variable can be specified explicitly as 'splat.i32x4 v5', or it can be
    // inferred from `inst_data.typevar_operand` for some opcodes.
    //
    // The value operands in `inst_data` are expected to use source numbering.
    //
    // Returns the controlling typevar for a polymorphic opcode, or `VOID` for a non-polymorphic
    // opcode.
    fn infer_typevar(&self,
                     ctx: &Context,
                     opcode: Opcode,
                     explicit_ctrl_type: Option<Type>,
                     inst_data: &InstructionData)
                     -> Result<Type> {
        let constraints = opcode.constraints();
        let ctrl_type = match explicit_ctrl_type {
            Some(t) => t,
            None => {
                if constraints.use_typevar_operand() {
                    // This is an opcode that supports type inference, AND there was no explicit
                    // type specified. Look up `ctrl_value` to see if it was defined already.
                    // TBD: If it is defined in another block, the type should have been specified
                    // explicitly. It is unfortunate that the correctness of IL depends on the
                    // layout of the blocks.
                    let ctrl_src_value = inst_data.typevar_operand()
                        .expect("Constraints <-> Format inconsistency");
                    ctx.function.dfg.value_type(match ctx.values.get(&ctrl_src_value) {
                        Some(&v) => v,
                        None => {
                            return err!(self.loc,
                                        "cannot determine type of operand {}",
                                        ctrl_src_value);
                        }
                    })
                } else if constraints.is_polymorphic() {
                    // This opcode does not support type inference, so the explicit type variable
                    // is required.
                    return err!(self.loc,
                                "type variable required for polymorphic opcode, e.g. '{}.{}'",
                                opcode,
                                constraints.ctrl_typeset().unwrap().example());
                } else {
                    // This is a non-polymorphic opcode. No typevar needed.
                    VOID
                }
            }
        };

        // Verify that `ctrl_type` is valid for the controlling type variable. We don't want to
        // attempt deriving types from an incorrect basis.
        // This is not a complete type check. The verifier does that.
        if let Some(typeset) = constraints.ctrl_typeset() {
            // This is a polymorphic opcode.
            if !typeset.contains(ctrl_type) {
                return err!(self.loc,
                            "{} is not a valid typevar for {}",
                            ctrl_type,
                            opcode);
            }
        } else {
            // Treat it as a syntax error to speficy a typevar on a non-polymorphic opcode.
            if ctrl_type != VOID {
                return err!(self.loc, "{} does not take a typevar", opcode);
            }
        }

        Ok(ctrl_type)
    }

    // Add mappings for a list of source values to their corresponding new values.
    fn add_values<S, V>(&self,
                        values: &mut HashMap<Value, Value>,
                        results: S,
                        new_results: V)
                        -> Result<()>
        where S: Iterator<Item = Value>,
              V: Iterator<Item = Value>
    {
        for (src, val) in results.zip(new_results) {
            if values.insert(src, val).is_some() {
                return err!(self.loc, "duplicate result value: {}", src);
            }
        }
        Ok(())
    }

    // Parse comma-separated value list into a VariableArgs struct.
    //
    // value_list ::= [ value { "," value } ]
    //
    fn parse_value_list(&mut self) -> Result<VariableArgs> {
        let mut args = VariableArgs::new();

        if let Some(Token::Value(v)) = self.token() {
            args.push(v);
            self.consume();
        } else {
            return Ok(args);
        }

        while self.optional(Token::Comma) {
            args.push(try!(self.match_value("expected value in argument list")));
        }

        Ok(args)
    }

    // Parse an optional value list enclosed in parantheses.
    fn parse_opt_value_list(&mut self) -> Result<VariableArgs> {
        if !self.optional(Token::LPar) {
            return Ok(VariableArgs::new());
        }

        let args = try!(self.parse_value_list());

        try!(self.match_token(Token::RPar, "expected ')' after arguments"));

        Ok(args)
    }

    // Parse the operands following the instruction opcode.
    // This depends on the format of the opcode.
    fn parse_inst_operands(&mut self, ctx: &Context, opcode: Opcode) -> Result<InstructionData> {
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
                    arg: lhs,
                    imm: rhs,
                }
            }
            InstructionFormat::BinaryImmRev => {
                let lhs = try!(self.match_imm64("expected immediate integer first operand"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let rhs = try!(self.match_value("expected SSA value second operand"));
                InstructionData::BinaryImmRev {
                    opcode: opcode,
                    ty: VOID,
                    imm: lhs,
                    arg: rhs,
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
            InstructionFormat::Ternary => {
                // Names here refer to the `select` instruction.
                // This format is also use by `fma`.
                let ctrl_arg = try!(self.match_value("expected SSA value control operand"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let true_arg = try!(self.match_value("expected SSA value true operand"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let false_arg = try!(self.match_value("expected SSA value false operand"));
                InstructionData::Ternary {
                    opcode: opcode,
                    ty: VOID,
                    args: [ctrl_arg, true_arg, false_arg],
                }
            }
            InstructionFormat::Jump => {
                // Parse the destination EBB number. Don't translate source to local numbers yet.
                let ebb_num = try!(self.match_ebb("expected jump destination EBB"));
                let args = try!(self.parse_opt_value_list());
                InstructionData::Jump {
                    opcode: opcode,
                    ty: VOID,
                    data: Box::new(JumpData {
                        destination: ebb_num,
                        arguments: args,
                    }),
                }
            }
            InstructionFormat::Branch => {
                let ctrl_arg = try!(self.match_value("expected SSA value control operand"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let ebb_num = try!(self.match_ebb("expected branch destination EBB"));
                let args = try!(self.parse_opt_value_list());
                InstructionData::Branch {
                    opcode: opcode,
                    ty: VOID,
                    data: Box::new(BranchData {
                        arg: ctrl_arg,
                        destination: ebb_num,
                        arguments: args,
                    }),
                }
            }
            InstructionFormat::InsertLane => {
                let lhs = try!(self.match_value("expected SSA value first operand"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let lane = try!(self.match_uimm8("expected lane number"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let rhs = try!(self.match_value("expected SSA value last operand"));
                InstructionData::InsertLane {
                    opcode: opcode,
                    ty: VOID,
                    lane: lane,
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::ExtractLane => {
                let arg = try!(self.match_value("expected SSA value last operand"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let lane = try!(self.match_uimm8("expected lane number"));
                InstructionData::ExtractLane {
                    opcode: opcode,
                    ty: VOID,
                    lane: lane,
                    arg: arg,
                }
            }
            InstructionFormat::IntCompare => {
                let cond = try!(self.match_enum("expected intcc condition code"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let lhs = try!(self.match_value("expected SSA value first operand"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let rhs = try!(self.match_value("expected SSA value second operand"));
                InstructionData::IntCompare {
                    opcode: opcode,
                    ty: VOID,
                    cond: cond,
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::FloatCompare => {
                let cond = try!(self.match_enum("expected floatcc condition code"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let lhs = try!(self.match_value("expected SSA value first operand"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let rhs = try!(self.match_value("expected SSA value second operand"));
                InstructionData::FloatCompare {
                    opcode: opcode,
                    ty: VOID,
                    cond: cond,
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::Return => {
                let args = try!(self.parse_value_list());
                InstructionData::Return {
                    opcode: opcode,
                    ty: VOID,
                    data: Box::new(ReturnData { args: args }),
                }
            }
            InstructionFormat::BranchTable => {
                let arg = try!(self.match_value("expected SSA value operand"));
                try!(self.match_token(Token::Comma, "expected ',' between operands"));
                let table = try!(self.match_jt().and_then(|num| ctx.get_jt(num, &self.loc)));
                InstructionData::BranchTable {
                    opcode: opcode,
                    ty: VOID,
                    arg: arg,
                    table: table,
                }
            }
            InstructionFormat::Call => {
                unimplemented!();
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cretonne::ir::types::{self, ArgumentType, ArgumentExtension};

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

        let mut ebbs = func.layout.ebbs();

        let ebb0 = ebbs.next().unwrap();
        assert_eq!(func.dfg.ebb_args(ebb0).next(), None);

        let ebb4 = ebbs.next().unwrap();
        let mut ebb4_args = func.dfg.ebb_args(ebb4);
        let arg0 = ebb4_args.next().unwrap();
        assert_eq!(func.dfg.value_type(arg0), types::I32);
        assert_eq!(ebb4_args.next(), None);
    }
}

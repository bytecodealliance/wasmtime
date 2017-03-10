
// ====--------------------------------------------------------------------------------------====//
//
// Parser for .cton files.
//
// ====--------------------------------------------------------------------------------------====//

use std::str::FromStr;
use std::{u16, u32};
use std::mem;
use cretonne::ir::{Function, Ebb, Opcode, Value, Type, FunctionName, StackSlotData, JumpTable,
                   JumpTableData, Signature, ArgumentType, ArgumentExtension, ExtFuncData, SigRef,
                   FuncRef, ValueLoc, ArgumentLoc};
use cretonne::ir::types::VOID;
use cretonne::ir::immediates::{Imm64, Ieee32, Ieee64};
use cretonne::ir::entities::AnyEntity;
use cretonne::ir::instructions::{InstructionFormat, InstructionData, VariableArgs,
                                 TernaryOverflowData};
use cretonne::isa::{self, TargetIsa, Encoding};
use cretonne::settings;
use testfile::{TestFile, Details, Comment};
use error::{Location, Error, Result};
use lexer::{self, Lexer, Token};
use testcommand::TestCommand;
use isaspec;
use sourcemap::{SourceMap, MutableSourceMap};

/// Parse the entire `text` into a list of functions.
///
/// Any test commands or ISA declarations are ignored.
pub fn parse_functions(text: &str) -> Result<Vec<Function>> {
    parse_test(text).map(|file| file.functions.into_iter().map(|(func, _)| func).collect())
}

/// Parse the entire `text` as a test case file.
///
/// The returned `TestFile` contains direct references to substrings of `text`.
pub fn parse_test<'a>(text: &'a str) -> Result<TestFile<'a>> {
    let mut parser = Parser::new(text);
    // Gather the preamble comments as 'Function'.
    parser.gather_comments(AnyEntity::Function);

    let commands = parser.parse_test_commands();
    let isa_spec = parser.parse_isa_specs()?;
    let preamble_comments = parser.take_comments();
    let functions = parser.parse_function_list(isa_spec.unique_isa())?;

    Ok(TestFile {
        commands: commands,
        isa_spec: isa_spec,
        preamble_comments: preamble_comments,
        functions: functions,
    })
}

pub struct Parser<'a> {
    lex: Lexer<'a>,

    lex_error: Option<lexer::Error>,

    // Current lookahead token.
    lookahead: Option<Token<'a>>,

    // Location of lookahead.
    loc: Location,

    // The currently active entity that should be associated with collected comments, or `None` if
    // comments are ignored.
    comment_entity: Option<AnyEntity>,

    // Comments collected so far.
    comments: Vec<Comment<'a>>,
}

// Context for resolving references when parsing a single function.
//
// Many entities like values, stack slots, and function signatures are referenced in the `.cton`
// file by number. We need to map these numbers to real references.
struct Context<'a> {
    function: Function,
    map: SourceMap,

    // Reference to the unique_isa for things like parsing ISA-specific instruction encoding
    // information. This is only `Some` if exactly one set of `isa` directives were found in the
    // prologue (it is valid to have directives for multiple different ISAs, but in that case we
    // couldn't know which ISA the provided encodings are intended for)
    unique_isa: Option<&'a TargetIsa>,
}

impl<'a> Context<'a> {
    fn new(f: Function, unique_isa: Option<&'a TargetIsa>) -> Context<'a> {
        Context {
            function: f,
            map: SourceMap::new(),
            unique_isa: unique_isa,
        }
    }

    // Get the index of a recipe name if it exists.
    fn find_recipe_index(&self, recipe_name: &str) -> Option<u16> {
        if let Some(unique_isa) = self.unique_isa {
            unique_isa.recipe_names()
                .iter()
                .position(|&name| name == recipe_name)
                .map(|idx| idx as u16)
        } else {
            None
        }
    }

    // Allocate a new stack slot and add a mapping number -> StackSlot.
    fn add_ss(&mut self, number: u32, data: StackSlotData, loc: &Location) -> Result<()> {
        self.map.def_ss(number, self.function.stack_slots.push(data), loc)
    }

    // Allocate a new signature and add a mapping number -> SigRef.
    fn add_sig(&mut self, number: u32, data: Signature, loc: &Location) -> Result<()> {
        self.map.def_sig(number, self.function.dfg.signatures.push(data), loc)
    }

    // Resolve a reference to a signature.
    fn get_sig(&self, number: u32, loc: &Location) -> Result<SigRef> {
        match self.map.get_sig(number) {
            Some(sig) => Ok(sig),
            None => err!(loc, "undefined signature sig{}", number),
        }
    }

    // Allocate a new external function and add a mapping number -> FuncRef.
    fn add_fn(&mut self, number: u32, data: ExtFuncData, loc: &Location) -> Result<()> {
        self.map.def_fn(number, self.function.dfg.ext_funcs.push(data), loc)
    }

    // Resolve a reference to a function.
    fn get_fn(&self, number: u32, loc: &Location) -> Result<FuncRef> {
        match self.map.get_fn(number) {
            Some(fnref) => Ok(fnref),
            None => err!(loc, "undefined function fn{}", number),
        }
    }

    // Allocate a new jump table and add a mapping number -> JumpTable.
    fn add_jt(&mut self, number: u32, data: JumpTableData, loc: &Location) -> Result<()> {
        self.map.def_jt(number, self.function.jump_tables.push(data), loc)
    }

    // Resolve a reference to a jump table.
    fn get_jt(&self, number: u32, loc: &Location) -> Result<JumpTable> {
        match self.map.get_jt(number) {
            Some(jt) => Ok(jt),
            None => err!(loc, "undefined jump table jt{}", number),
        }
    }

    // Allocate a new EBB and add a mapping src_ebb -> Ebb.
    fn add_ebb(&mut self, src_ebb: Ebb, loc: &Location) -> Result<Ebb> {
        let ebb = self.function.dfg.make_ebb();
        self.function.layout.append_ebb(ebb);
        self.map.def_ebb(src_ebb, ebb, loc).and(Ok(ebb))
    }

    // The parser creates all instructions with Ebb and Value references using the source file
    // numbering. These references need to be rewritten after parsing is complete since forward
    // references are allowed.
    fn rewrite_references(&mut self) -> Result<()> {
        for ebb in self.function.layout.ebbs() {
            for inst in self.function.layout.ebb_insts(ebb) {
                let loc = inst.into();
                let value_lists = &mut self.function.dfg.value_lists;
                match self.function.dfg.insts[inst] {
                    InstructionData::Nullary { .. } |
                    InstructionData::UnaryImm { .. } |
                    InstructionData::UnaryIeee32 { .. } |
                    InstructionData::UnaryIeee64 { .. } |
                    InstructionData::UnaryImmVector { .. } => {}

                    InstructionData::Unary { ref mut arg, .. } |
                    InstructionData::UnarySplit { ref mut arg, .. } |
                    InstructionData::BinaryImm { ref mut arg, .. } |
                    InstructionData::ExtractLane { ref mut arg, .. } |
                    InstructionData::BranchTable { ref mut arg, .. } => {
                        self.map.rewrite_value(arg, loc)?;
                    }

                    InstructionData::Binary { ref mut args, .. } |
                    InstructionData::BinaryOverflow { ref mut args, .. } |
                    InstructionData::InsertLane { ref mut args, .. } |
                    InstructionData::IntCompare { ref mut args, .. } |
                    InstructionData::FloatCompare { ref mut args, .. } => {
                        self.map.rewrite_values(args, loc)?;
                    }

                    InstructionData::Ternary { ref mut args, .. } => {
                        self.map.rewrite_values(args, loc)?;
                    }

                    InstructionData::TernaryOverflow { ref mut data, .. } => {
                        self.map.rewrite_values(&mut data.args, loc)?;
                    }

                    InstructionData::Jump { ref mut destination, ref mut args, .. } => {
                        self.map.rewrite_ebb(destination, loc)?;
                        self.map.rewrite_values(args.as_mut_slice(value_lists), loc)?;
                    }

                    InstructionData::Branch { ref mut destination, ref mut args, .. } => {
                        self.map.rewrite_ebb(destination, loc)?;
                        self.map.rewrite_values(args.as_mut_slice(value_lists), loc)?;
                    }

                    InstructionData::Call { ref mut args, .. } => {
                        self.map.rewrite_values(args.as_mut_slice(value_lists), loc)?;
                    }

                    InstructionData::IndirectCall { ref mut args, .. } => {
                        self.map.rewrite_values(args.as_mut_slice(value_lists), loc)?;
                    }

                    InstructionData::Return { ref mut args, .. } => {
                        self.map.rewrite_values(args.as_mut_slice(value_lists), loc)?;
                    }

                    InstructionData::ReturnReg { ref mut args, .. } => {
                        self.map.rewrite_values(args.as_mut_slice(value_lists), loc)?;
                    }
                }
            }
        }

        // Rewrite EBB references in jump tables.
        for jt in self.function.jump_tables.keys() {
            let loc = jt.into();
            for ebb_ref in self.function.jump_tables[jt].as_mut_slice() {
                if let Some(mut ebb) = ebb_ref.expand() {
                    self.map.rewrite_ebb(&mut ebb, loc)?;
                    // Convert back to a packed option.
                    *ebb_ref = ebb.into();
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
            comment_entity: None,
            comments: Vec::new(),
        }
    }

    // Consume the current lookahead token and return it.
    fn consume(&mut self) -> Token<'a> {
        self.lookahead.take().expect("No token to consume")
    }

    // Consume the whole line following the current lookahead token.
    // Return the text of the line tail.
    fn consume_line(&mut self) -> &'a str {
        let rest = self.lex.rest_of_line();
        self.consume();
        rest
    }

    // Get the current lookahead token, after making sure there is one.
    fn token(&mut self) -> Option<Token<'a>> {
        while self.lookahead == None {
            match self.lex.next() {
                Some(Ok(lexer::LocatedToken { token, location })) => {
                    match token {
                        Token::Comment(text) => {
                            // Gather comments, associate them with `comment_entity`.
                            if let Some(entity) = self.comment_entity {
                                self.comments.push(Comment {
                                    entity: entity,
                                    text: text,
                                });
                            }
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

    // Begin gathering comments associated with `entity`.
    fn gather_comments<E: Into<AnyEntity>>(&mut self, entity: E) {
        self.comment_entity = Some(entity.into());
    }

    // Get the comments gathered so far, clearing out the internal list.
    fn take_comments(&mut self) -> Vec<Comment<'a>> {
        mem::replace(&mut self.comments, Vec::new())
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

    // Match and consume a function reference.
    fn match_fn(&mut self, err_msg: &str) -> Result<u32> {
        if let Some(Token::FuncRef(fnref)) = self.token() {
            self.consume();
            Ok(fnref)
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume a signature reference.
    fn match_sig(&mut self, err_msg: &str) -> Result<u32> {
        if let Some(Token::SigRef(sigref)) = self.token() {
            self.consume();
            Ok(sigref)
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

    // Match and consume a u32 immediate.
    // This is used for stack argument byte offsets.
    fn match_uimm32(&mut self, err_msg: &str) -> Result<u32> {
        if let Some(Token::Integer(text)) = self.token() {
            self.consume();
            // Lexer just gives us raw text that looks like an integer.
            // Parse it as a u32 to check for overflow and other issues.
            text.parse().map_err(|_| self.error("expected u32 decimal immediate"))
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

    // Match and consume an identifier.
    fn match_any_identifier(&mut self, err_msg: &str) -> Result<&'a str> {
        if let Some(Token::Identifier(text)) = self.token() {
            self.consume();
            Ok(text)
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume a HexSequence that fits into a u16.
    // This is used for instruction encodings.
    fn match_hex16(&mut self, err_msg: &str) -> Result<u16> {
        if let Some(Token::HexSequence(bits_str)) = self.token() {
            self.consume();
            // The only error we anticipate from this parse is overflow, the lexer should
            // already have ensured that the string doesn't contain invalid characters, and
            // isn't empty or negative.
            u16::from_str_radix(bits_str, 16)
                .map_err(|_| self.error("the hex sequence given overflows the u16 type"))
        } else {
            err!(self.loc, err_msg)
        }
    }

    /// Parse a list of test commands.
    pub fn parse_test_commands(&mut self) -> Vec<TestCommand<'a>> {
        let mut list = Vec::new();
        while self.token() == Some(Token::Identifier("test")) {
            list.push(TestCommand::new(self.consume_line()));
        }
        list
    }

    /// Parse a list of ISA specs.
    ///
    /// Accept a mix of `isa` and `set` command lines. The `set` commands are cumulative.
    ///
    pub fn parse_isa_specs(&mut self) -> Result<isaspec::IsaSpec> {
        // Was there any `isa` commands?
        let mut seen_isa = false;
        // Location of last `set` command since the last `isa`.
        let mut last_set_loc = None;

        let mut isas = Vec::new();
        let mut flag_builder = settings::builder();

        while let Some(Token::Identifier(command)) = self.token() {
            match command {
                "set" => {
                    last_set_loc = Some(self.loc);
                    isaspec::parse_options(self.consume_line().trim().split_whitespace(),
                                           &mut flag_builder,
                                           &self.loc)?;
                }
                "isa" => {
                    last_set_loc = None;
                    seen_isa = true;
                    let loc = self.loc;
                    // Grab the whole line so the lexer won't go looking for tokens on the
                    // following lines.
                    let mut words = self.consume_line().trim().split_whitespace();
                    // Look for `isa foo`.
                    let isa_name = match words.next() {
                        None => return err!(loc, "expected ISA name"),
                        Some(w) => w,
                    };
                    let mut isa_builder = match isa::lookup(isa_name) {
                        None => return err!(loc, "unknown ISA '{}'", isa_name),
                        Some(b) => b,
                    };
                    // Apply the ISA-specific settings to `isa_builder`.
                    isaspec::parse_options(words, &mut isa_builder, &self.loc)?;

                    // Construct a trait object with the aggregrate settings.
                    isas.push(isa_builder.finish(settings::Flags::new(&flag_builder)));
                }
                _ => break,
            }
        }
        if !seen_isa {
            // No `isa` commands, but we allow for `set` commands.
            Ok(isaspec::IsaSpec::None(settings::Flags::new(&flag_builder)))
        } else if let Some(loc) = last_set_loc {
            err!(loc,
                 "dangling 'set' command after ISA specification has no effect.")
        } else {
            Ok(isaspec::IsaSpec::Some(isas))
        }
    }

    /// Parse a list of function definitions.
    ///
    /// This is the top-level parse function matching the whole contents of a file.
    pub fn parse_function_list(&mut self,
                               unique_isa: Option<&TargetIsa>)
                               -> Result<Vec<(Function, Details<'a>)>> {
        let mut list = Vec::new();
        while self.token().is_some() {
            list.push(self.parse_function(unique_isa)?);
        }
        Ok(list)
    }

    // Parse a whole function definition.
    //
    // function ::= * function-spec "{" preamble function-body "}"
    //
    fn parse_function(&mut self,
                      unique_isa: Option<&TargetIsa>)
                      -> Result<(Function, Details<'a>)> {
        // Begin gathering comments.
        // Make sure we don't include any comments before the `function` keyword.
        self.token();
        self.comments.clear();
        self.gather_comments(AnyEntity::Function);

        let (location, name, sig) = self.parse_function_spec(unique_isa)?;
        let mut ctx = Context::new(Function::with_name_signature(name, sig), unique_isa);

        // function ::= function-spec * "{" preamble function-body "}"
        self.match_token(Token::LBrace, "expected '{' before function body")?;
        // function ::= function-spec "{" * preamble function-body "}"
        self.parse_preamble(&mut ctx)?;
        // function ::= function-spec "{"  preamble * function-body "}"
        self.parse_function_body(&mut ctx)?;
        // function ::= function-spec "{" preamble function-body * "}"
        self.match_token(Token::RBrace, "expected '}' after function body")?;

        // Collect any comments following the end of the function, then stop gathering comments.
        self.gather_comments(AnyEntity::Function);
        self.token();
        self.comment_entity = None;

        // Rewrite references to values and EBBs after parsing everything to allow forward
        // references.
        ctx.rewrite_references()?;

        let details = Details {
            location: location,
            comments: self.take_comments(),
            map: ctx.map,
        };

        Ok((ctx.function, details))
    }

    // Parse a function spec.
    //
    // function-spec ::= * "function" name signature
    //
    fn parse_function_spec(&mut self,
                           unique_isa: Option<&TargetIsa>)
                           -> Result<(Location, FunctionName, Signature)> {
        self.match_identifier("function", "expected 'function'")?;
        let location = self.loc;

        // function-spec ::= "function" * name signature
        let name = self.parse_function_name()?;

        // function-spec ::= "function" name * signature
        let sig = self.parse_signature(unique_isa)?;

        Ok((location, name, sig))
    }

    // Parse a function name.
    //
    // function ::= "function" * name signature { ... }
    //
    fn parse_function_name(&mut self) -> Result<FunctionName> {
        match self.token() {
            Some(Token::Identifier(s)) => {
                self.consume();
                Ok(FunctionName::new(s))
            }
            _ => err!(self.loc, "expected function name"),
        }
    }

    // Parse a function signature.
    //
    // signature ::=  * "(" [arglist] ")" ["->" retlist] [call_conv]
    //
    fn parse_signature(&mut self, unique_isa: Option<&TargetIsa>) -> Result<Signature> {
        let mut sig = Signature::new();

        self.match_token(Token::LPar, "expected function signature: ( args... )")?;
        // signature ::=  "(" * [arglist] ")" ["->" retlist] [call_conv]
        if self.token() != Some(Token::RPar) {
            sig.argument_types = self.parse_argument_list(unique_isa)?;
        }
        self.match_token(Token::RPar, "expected ')' after function arguments")?;
        if self.optional(Token::Arrow) {
            sig.return_types = self.parse_argument_list(unique_isa)?;
        }

        if sig.argument_types.iter().all(|a| a.location.is_assigned()) {
            sig.compute_argument_bytes();
        }

        // TBD: calling convention.

        Ok(sig)
    }

    // Parse list of function argument / return value types.
    //
    // arglist ::= * arg { "," arg }
    //
    fn parse_argument_list(&mut self, unique_isa: Option<&TargetIsa>) -> Result<Vec<ArgumentType>> {
        let mut list = Vec::new();

        // arglist ::= * arg { "," arg }
        list.push(self.parse_argument_type(unique_isa)?);

        // arglist ::= arg * { "," arg }
        while self.optional(Token::Comma) {
            // arglist ::= arg { "," * arg }
            list.push(self.parse_argument_type(unique_isa)?);
        }

        Ok(list)
    }

    // Parse a single argument type with flags.
    fn parse_argument_type(&mut self, unique_isa: Option<&TargetIsa>) -> Result<ArgumentType> {
        // arg ::= * type { flag } [ argumentloc ]
        let mut arg = ArgumentType::new(self.match_type("expected argument type")?);

        // arg ::= type * { flag } [ argumentloc ]
        while let Some(Token::Identifier(s)) = self.token() {
            match s {
                "uext" => arg.extension = ArgumentExtension::Uext,
                "sext" => arg.extension = ArgumentExtension::Sext,
                "inreg" => arg.inreg = true,
                _ => break,
            }
            self.consume();
        }

        // arg ::= type { flag } * [ argumentloc ]
        arg.location = self.parse_argument_location(unique_isa)?;

        Ok(arg)
    }

    // Parse an argument location specifier; either a register or a byte offset into the stack.
    fn parse_argument_location(&mut self, unique_isa: Option<&TargetIsa>) -> Result<ArgumentLoc> {
        // argumentloc ::= '[' regname | uimm32 ']'
        if self.optional(Token::LBracket) {
            let result = match self.token() {
                Some(Token::Name(name)) => {
                    self.consume();
                    if let Some(isa) = unique_isa {
                        isa.register_info()
                            .parse_regunit(name)
                            .map(ArgumentLoc::Reg)
                            .ok_or(self.error("invalid register name"))
                    } else {
                        // We are unable to parse the register without a TargetISA, so we quietly
                        // ignore it.
                        Ok(ArgumentLoc::Unassigned)
                    }
                }
                Some(Token::Integer(_)) => {
                    let offset = self.match_uimm32("expected stack argument byte offset")?;
                    Ok(ArgumentLoc::Stack(offset))
                }
                Some(Token::Minus) => {
                    self.consume();
                    Ok(ArgumentLoc::Unassigned)
                }
                _ => err!(self.loc, "expected argument location"),
            };

            self.match_token(Token::RBracket,
                             "expected ']' to end argument location annotation")?;

            result
        } else {
            Ok(ArgumentLoc::Unassigned)
        }
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
            match self.token() {
                Some(Token::StackSlot(..)) => {
                    self.gather_comments(ctx.function.stack_slots.next_key());
                    self.parse_stack_slot_decl()
                        .and_then(|(num, dat)| ctx.add_ss(num, dat, &self.loc))
                }
                Some(Token::SigRef(..)) => {
                    self.gather_comments(ctx.function.dfg.signatures.next_key());
                    self.parse_signature_decl(ctx.unique_isa)
                        .and_then(|(num, dat)| ctx.add_sig(num, dat, &self.loc))
                }
                Some(Token::FuncRef(..)) => {
                    self.gather_comments(ctx.function.dfg.ext_funcs.next_key());
                    self.parse_function_decl(ctx)
                        .and_then(|(num, dat)| ctx.add_fn(num, dat, &self.loc))
                }
                Some(Token::JumpTable(..)) => {
                    self.gather_comments(ctx.function.jump_tables.next_key());
                    self.parse_jump_table_decl()
                        .and_then(|(num, dat)| ctx.add_jt(num, dat, &self.loc))
                }
                // More to come..
                _ => return Ok(()),
            }?;
        }
    }

    // Parse a stack slot decl.
    //
    // stack-slot-decl ::= * StackSlot(ss) "=" "stack_slot" Bytes {"," stack-slot-flag}
    fn parse_stack_slot_decl(&mut self) -> Result<(u32, StackSlotData)> {
        let number = self.match_ss("expected stack slot number: ss«n»")?;
        self.match_token(Token::Equal, "expected '=' in stack_slot decl")?;
        self.match_identifier("stack_slot", "expected 'stack_slot'")?;

        // stack-slot-decl ::= StackSlot(ss) "=" "stack_slot" * Bytes {"," stack-slot-flag}
        let bytes: i64 = self.match_imm64("expected byte-size in stack_slot decl")?.into();
        if bytes < 0 {
            return err!(self.loc, "negative stack slot size");
        }
        if bytes > u32::MAX as i64 {
            return err!(self.loc, "stack slot too large");
        }
        let data = StackSlotData::new(bytes as u32);

        // TBD: stack-slot-decl ::= StackSlot(ss) "=" "stack_slot" Bytes * {"," stack-slot-flag}
        Ok((number, data))
    }

    // Parse a signature decl.
    //
    // signature-decl ::= SigRef(sigref) "=" "signature" signature
    //
    fn parse_signature_decl(&mut self, unique_isa: Option<&TargetIsa>) -> Result<(u32, Signature)> {
        let number = self.match_sig("expected signature number: sig«n»")?;
        self.match_token(Token::Equal, "expected '=' in signature decl")?;
        self.match_identifier("signature", "expected 'signature'")?;
        let data = self.parse_signature(unique_isa)?;
        Ok((number, data))
    }

    // Parse a function decl.
    //
    // Two variants:
    //
    // function-decl ::= FuncRef(fnref) "=" function-spec
    //                   FuncRef(fnref) "=" SigRef(sig) name
    //
    // The first variant allocates a new signature reference. The second references an existing
    // signature which must be declared first.
    //
    fn parse_function_decl(&mut self, ctx: &mut Context) -> Result<(u32, ExtFuncData)> {
        let number = self.match_fn("expected function number: fn«n»")?;
        self.match_token(Token::Equal, "expected '=' in function decl")?;

        let data = match self.token() {
            Some(Token::Identifier("function")) => {
                let (loc, name, sig) = self.parse_function_spec(ctx.unique_isa)?;
                let sigref = ctx.function.dfg.signatures.push(sig);
                ctx.map.def_entity(sigref.into(), &loc).expect("duplicate SigRef entities created");
                ExtFuncData {
                    name: name,
                    signature: sigref,
                }
            }
            Some(Token::SigRef(sig_src)) => {
                let sig = ctx.get_sig(sig_src, &self.loc)?;
                self.consume();
                let name = self.parse_function_name()?;
                ExtFuncData {
                    name: name,
                    signature: sig,
                }
            }
            _ => return err!(self.loc, "expected 'function' or sig«n» in function decl"),
        };
        Ok((number, data))
    }

    // Parse a jump table decl.
    //
    // jump-table-decl ::= * JumpTable(jt) "=" "jump_table" jt-entry {"," jt-entry}
    fn parse_jump_table_decl(&mut self) -> Result<(u32, JumpTableData)> {
        let number = self.match_jt()?;
        self.match_token(Token::Equal, "expected '=' in jump_table decl")?;
        self.match_identifier("jump_table", "expected 'jump_table'")?;

        let mut data = JumpTableData::new();

        // jump-table-decl ::= JumpTable(jt) "=" "jump_table" * jt-entry {"," jt-entry}
        for idx in 0usize.. {
            if let Some(dest) = self.parse_jump_table_entry()? {
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
            self.parse_extended_basic_block(ctx)?;
        }
        Ok(())
    }

    // Parse an extended basic block, add contents to `ctx`.
    //
    // extended-basic-block ::= * ebb-header { instruction }
    // ebb-header           ::= Ebb(ebb) [ebb-args] ":"
    //
    fn parse_extended_basic_block(&mut self, ctx: &mut Context) -> Result<()> {
        let ebb_num = self.match_ebb("expected EBB header")?;
        let ebb = ctx.add_ebb(ebb_num, &self.loc)?;
        self.gather_comments(ebb);

        if !self.optional(Token::Colon) {
            // ebb-header ::= Ebb(ebb) [ * ebb-args ] ":"
            self.parse_ebb_args(ctx, ebb)?;
            self.match_token(Token::Colon, "expected ':' after EBB arguments")?;
        }

        // extended-basic-block ::= ebb-header * { instruction }
        while match self.token() {
            Some(Token::Value(_)) => true,
            Some(Token::Identifier(_)) => true,
            Some(Token::LBracket) => true,
            _ => false,
        } {
            self.parse_instruction(ctx, ebb)?;
        }

        Ok(())
    }

    // Parse parenthesized list of EBB arguments. Returns a vector of (u32, Type) pairs with the
    // source vx numbers of the defined values and the defined types.
    //
    // ebb-args ::= * "(" ebb-arg { "," ebb-arg } ")"
    fn parse_ebb_args(&mut self, ctx: &mut Context, ebb: Ebb) -> Result<()> {
        // ebb-args ::= * "(" ebb-arg { "," ebb-arg } ")"
        self.match_token(Token::LPar, "expected '(' before EBB arguments")?;

        // ebb-args ::= "(" * ebb-arg { "," ebb-arg } ")"
        self.parse_ebb_arg(ctx, ebb)?;

        // ebb-args ::= "(" ebb-arg * { "," ebb-arg } ")"
        while self.optional(Token::Comma) {
            // ebb-args ::= "(" ebb-arg { "," * ebb-arg } ")"
            self.parse_ebb_arg(ctx, ebb)?;
        }

        // ebb-args ::= "(" ebb-arg { "," ebb-arg } * ")"
        self.match_token(Token::RPar, "expected ')' after EBB arguments")?;

        Ok(())
    }

    // Parse a single EBB argument declaration, and append it to `ebb`.
    //
    // ebb-arg ::= * Value(vx) ":" Type(t)
    //
    fn parse_ebb_arg(&mut self, ctx: &mut Context, ebb: Ebb) -> Result<()> {
        // ebb-arg ::= * Value(vx) ":" Type(t)
        let vx = self.match_value("EBB argument must be a value")?;
        let vx_location = self.loc;
        // ebb-arg ::= Value(vx) * ":" Type(t)
        self.match_token(Token::Colon, "expected ':' after EBB argument")?;
        // ebb-arg ::= Value(vx) ":" * Type(t)
        let t = self.match_type("expected EBB argument type")?;
        // Allocate the EBB argument and add the mapping.
        let value = ctx.function.dfg.append_ebb_arg(ebb, t);
        ctx.map.def_value(vx, value, &vx_location)
    }

    fn parse_value_location(&mut self, ctx: &Context) -> Result<ValueLoc> {
        match self.token() {
            Some(Token::StackSlot(src_num)) => {
                self.consume();
                if let Some(ss) = ctx.map.get_ss(src_num) {
                    Ok(ValueLoc::Stack(ss))
                } else {
                    err!(self.loc,
                         "attempted to use undefined stack slot ss{}",
                         src_num)
                }
            }
            Some(Token::Name(name)) => {
                self.consume();
                if let Some(isa) = ctx.unique_isa {
                    isa.register_info()
                        .parse_regunit(name)
                        .map(ValueLoc::Reg)
                        .ok_or(self.error("invalid register value location"))
                } else {
                    // For convenience we ignore value locations when no unique ISA is specified
                    Ok(ValueLoc::Unassigned)
                }
            }
            Some(Token::Minus) => {
                self.consume();
                Ok(ValueLoc::Unassigned)
            }
            _ => err!(self.loc, "invalid value location"),
        }
    }

    fn parse_instruction_encoding(&mut self,
                                  ctx: &Context)
                                  -> Result<(Option<Encoding>, Option<Vec<ValueLoc>>)> {
        let (mut encoding, mut result_locations) = (None, None);

        // encoding ::= "[" encoding_literal result_locations "]"
        if self.optional(Token::LBracket) {
            // encoding_literal ::= "-" | Identifier HexSequence
            if !self.optional(Token::Minus) {
                let recipe = self.match_any_identifier("expected instruction encoding or '-'")?;
                let bits = self.match_hex16("expected a hex sequence")?;

                if let Some(recipe_index) = ctx.find_recipe_index(recipe) {
                    encoding = Some(Encoding::new(recipe_index, bits));
                } else if ctx.unique_isa.is_some() {
                    return err!(self.loc, "invalid instruction recipe");
                } else {
                    // We allow encodings to be specified when there's no unique ISA purely
                    // for convenience, eg when copy-pasting code for a test.
                }
            }

            // result_locations ::= ("," ( "-" | names ) )?
            // names ::= Name { "," Name }
            if self.optional(Token::Comma) {
                let mut results = Vec::new();

                results.push(self.parse_value_location(ctx)?);
                while self.optional(Token::Comma) {
                    results.push(self.parse_value_location(ctx)?);
                }

                result_locations = Some(results);
            }

            self.match_token(Token::RBracket,
                             "expected ']' to terminate instruction encoding")?;
        }

        Ok((encoding, result_locations))
    }

    // Parse an instruction, append it to `ebb`.
    //
    // instruction ::= [inst-results "="] Opcode(opc) ["." Type] ...
    // inst-results ::= Value(v) { "," Value(vx) }
    //
    fn parse_instruction(&mut self, ctx: &mut Context, ebb: Ebb) -> Result<()> {
        // Collect comments for the next instruction to be allocated.
        self.gather_comments(ctx.function.dfg.next_inst());

        let (encoding, result_locations) = self.parse_instruction_encoding(ctx)?;

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
                results.push(self.match_value("expected result value")?);
            }

            self.match_token(Token::Equal, "expected '=' before opcode")?;
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
            Some(self.match_type("expected type after 'opcode.'")?)
        } else {
            None
        };

        // instruction ::=  [inst-results "="] Opcode(opc) ["." Type] * ...
        let inst_data = self.parse_inst_operands(ctx, opcode)?;

        // We're done parsing the instruction now.
        //
        // We still need to check that the number of result values in the source matches the opcode
        // or function call signature. We also need to create values with the right type for all
        // the instruction results.
        let ctrl_typevar = self.infer_typevar(ctx, opcode, explicit_ctrl_type, &inst_data)?;
        let inst = ctx.function.dfg.make_inst(inst_data);
        let num_results = ctx.function.dfg.make_inst_results(inst, ctrl_typevar);
        ctx.function.layout.append_inst(inst, ebb);
        ctx.map.def_entity(inst.into(), &opcode_loc).expect("duplicate inst references created");

        if let Some(encoding) = encoding {
            *ctx.function.encodings.ensure(inst) = encoding;
        }

        if results.len() != num_results {
            return err!(self.loc,
                        "instruction produces {} result values, {} given",
                        num_results,
                        results.len());
        }

        if let Some(ref result_locations) = result_locations {
            if results.len() != result_locations.len() {
                return err!(self.loc,
                            "instruction produces {} result values, but {} locations were \
                             specified",
                            results.len(),
                            result_locations.len());
            }
        }

        // Now map the source result values to the just created instruction results.
        // Pass a reference to `ctx.values` instead of `ctx` itself since the `Values` iterator
        // holds a reference to `ctx.function`.
        self.add_values(&mut ctx.map,
                        results.into_iter(),
                        ctx.function.dfg.inst_results(inst))?;

        if let Some(result_locations) = result_locations {
            for (value, loc) in ctx.function.dfg.inst_results(inst).zip(result_locations) {
                *ctx.function.locations.ensure(value) = loc;
            }
        }

        Ok(())
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
                    let ctrl_src_value = inst_data.typevar_operand(&ctx.function.dfg.value_lists)
                        .expect("Constraints <-> Format inconsistency");
                    ctx.function.dfg.value_type(match ctx.map.get_value(ctrl_src_value) {
                        Some(v) => v,
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
    fn add_values<S, V>(&self, map: &mut SourceMap, results: S, new_results: V) -> Result<()>
        where S: Iterator<Item = Value>,
              V: Iterator<Item = Value>
    {
        for (src, val) in results.zip(new_results) {
            map.def_value(src, val, &self.loc)?;
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
            args.push(self.match_value("expected value in argument list")?);
        }

        Ok(args)
    }

    // Parse an optional value list enclosed in parantheses.
    fn parse_opt_value_list(&mut self) -> Result<VariableArgs> {
        if !self.optional(Token::LPar) {
            return Ok(VariableArgs::new());
        }

        let args = self.parse_value_list()?;

        self.match_token(Token::RPar, "expected ')' after arguments")?;

        Ok(args)
    }

    // Parse the operands following the instruction opcode.
    // This depends on the format of the opcode.
    fn parse_inst_operands(&mut self,
                           ctx: &mut Context,
                           opcode: Opcode)
                           -> Result<InstructionData> {
        Ok(match opcode.format() {
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
                    arg: self.match_value("expected SSA value operand")?,
                }
            }
            InstructionFormat::UnaryImm => {
                InstructionData::UnaryImm {
                    opcode: opcode,
                    ty: VOID,
                    imm: self.match_imm64("expected immediate integer operand")?,
                }
            }
            InstructionFormat::UnaryIeee32 => {
                InstructionData::UnaryIeee32 {
                    opcode: opcode,
                    ty: VOID,
                    imm: self.match_ieee32("expected immediate 32-bit float operand")?,
                }
            }
            InstructionFormat::UnaryIeee64 => {
                InstructionData::UnaryIeee64 {
                    opcode: opcode,
                    ty: VOID,
                    imm: self.match_ieee64("expected immediate 64-bit float operand")?,
                }
            }
            InstructionFormat::UnaryImmVector => {
                unimplemented!();
            }
            InstructionFormat::UnarySplit => {
                InstructionData::UnarySplit {
                    opcode: opcode,
                    ty: VOID,
                    second_result: None.into(),
                    arg: self.match_value("expected SSA value operand")?,
                }
            }
            InstructionFormat::Binary => {
                let lhs = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let rhs = self.match_value("expected SSA value second operand")?;
                InstructionData::Binary {
                    opcode: opcode,
                    ty: VOID,
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::BinaryImm => {
                let lhs = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let rhs = self.match_imm64("expected immediate integer second operand")?;
                InstructionData::BinaryImm {
                    opcode: opcode,
                    ty: VOID,
                    arg: lhs,
                    imm: rhs,
                }
            }
            InstructionFormat::BinaryOverflow => {
                let lhs = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let rhs = self.match_value("expected SSA value second operand")?;
                InstructionData::BinaryOverflow {
                    opcode: opcode,
                    ty: VOID,
                    second_result: None.into(),
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::Ternary => {
                // Names here refer to the `select` instruction.
                // This format is also use by `fma`.
                let ctrl_arg = self.match_value("expected SSA value control operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let true_arg = self.match_value("expected SSA value true operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let false_arg = self.match_value("expected SSA value false operand")?;
                InstructionData::Ternary {
                    opcode: opcode,
                    ty: VOID,
                    args: [ctrl_arg, true_arg, false_arg],
                }
            }
            InstructionFormat::TernaryOverflow => {
                // Names here refer to the `iadd_carry` instruction.
                let lhs = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let rhs = self.match_value("expected SSA value second operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let cin = self.match_value("expected SSA value third operand")?;
                InstructionData::TernaryOverflow {
                    opcode: opcode,
                    ty: VOID,
                    second_result: None.into(),
                    data: Box::new(TernaryOverflowData { args: [lhs, rhs, cin] }),
                }
            }
            InstructionFormat::Jump => {
                // Parse the destination EBB number. Don't translate source to local numbers yet.
                let ebb_num = self.match_ebb("expected jump destination EBB")?;
                let args = self.parse_opt_value_list()?;
                InstructionData::Jump {
                    opcode: opcode,
                    ty: VOID,
                    destination: ebb_num,
                    args: args.into_value_list(&[], &mut ctx.function.dfg.value_lists),
                }
            }
            InstructionFormat::Branch => {
                let ctrl_arg = self.match_value("expected SSA value control operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let ebb_num = self.match_ebb("expected branch destination EBB")?;
                let args = self.parse_opt_value_list()?;
                InstructionData::Branch {
                    opcode: opcode,
                    ty: VOID,
                    destination: ebb_num,
                    args: args.into_value_list(&[ctrl_arg], &mut ctx.function.dfg.value_lists),
                }
            }
            InstructionFormat::InsertLane => {
                let lhs = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let lane = self.match_uimm8("expected lane number")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let rhs = self.match_value("expected SSA value last operand")?;
                InstructionData::InsertLane {
                    opcode: opcode,
                    ty: VOID,
                    lane: lane,
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::ExtractLane => {
                let arg = self.match_value("expected SSA value last operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let lane = self.match_uimm8("expected lane number")?;
                InstructionData::ExtractLane {
                    opcode: opcode,
                    ty: VOID,
                    lane: lane,
                    arg: arg,
                }
            }
            InstructionFormat::IntCompare => {
                let cond = self.match_enum("expected intcc condition code")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let lhs = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let rhs = self.match_value("expected SSA value second operand")?;
                InstructionData::IntCompare {
                    opcode: opcode,
                    ty: VOID,
                    cond: cond,
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::FloatCompare => {
                let cond = self.match_enum("expected floatcc condition code")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let lhs = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let rhs = self.match_value("expected SSA value second operand")?;
                InstructionData::FloatCompare {
                    opcode: opcode,
                    ty: VOID,
                    cond: cond,
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::Call => {
                let func_ref = self.match_fn("expected function reference")
                    .and_then(|num| ctx.get_fn(num, &self.loc))?;
                self.match_token(Token::LPar, "expected '(' before arguments")?;
                let args = self.parse_value_list()?;
                self.match_token(Token::RPar, "expected ')' after arguments")?;
                InstructionData::Call {
                    opcode: opcode,
                    ty: VOID,
                    second_result: None.into(),
                    func_ref: func_ref,
                    args: args.into_value_list(&[], &mut ctx.function.dfg.value_lists),
                }
            }
            InstructionFormat::IndirectCall => {
                let sig_ref = self.match_sig("expected signature reference")
                    .and_then(|num| ctx.get_sig(num, &self.loc))?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let callee = self.match_value("expected SSA value callee operand")?;
                self.match_token(Token::LPar, "expected '(' before arguments")?;
                let args = self.parse_value_list()?;
                self.match_token(Token::RPar, "expected ')' after arguments")?;
                InstructionData::IndirectCall {
                    opcode: opcode,
                    ty: VOID,
                    second_result: None.into(),
                    sig_ref: sig_ref,
                    args: args.into_value_list(&[callee], &mut ctx.function.dfg.value_lists),
                }
            }
            InstructionFormat::Return => {
                let args = self.parse_value_list()?;
                InstructionData::Return {
                    opcode: opcode,
                    ty: VOID,
                    args: args.into_value_list(&[], &mut ctx.function.dfg.value_lists),
                }
            }
            InstructionFormat::ReturnReg => {
                let raddr = self.match_value("expected SSA value return address operand")?;
                let args = if self.optional(Token::Comma) {
                    self.parse_value_list()?
                } else {
                    VariableArgs::new()
                };
                InstructionData::ReturnReg {
                    opcode: opcode,
                    ty: VOID,
                    args: args.into_value_list(&[raddr], &mut ctx.function.dfg.value_lists),
                }
            }
            InstructionFormat::BranchTable => {
                let arg = self.match_value("expected SSA value operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let table = self.match_jt().and_then(|num| ctx.get_jt(num, &self.loc))?;
                InstructionData::BranchTable {
                    opcode: opcode,
                    ty: VOID,
                    arg: arg,
                    table: table,
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cretonne::ir::ArgumentExtension;
    use cretonne::ir::types;
    use cretonne::ir::entities::AnyEntity;
    use testfile::{Details, Comment};
    use isaspec::IsaSpec;
    use error::Error;

    #[test]
    fn argument_type() {
        let mut p = Parser::new("i32 sext");
        let arg = p.parse_argument_type(None).unwrap();
        assert_eq!(arg.value_type, types::I32);
        assert_eq!(arg.extension, ArgumentExtension::Sext);
        assert_eq!(arg.inreg, false);
        let Error { location, message } = p.parse_argument_type(None).unwrap_err();
        assert_eq!(location.line_number, 1);
        assert_eq!(message, "expected argument type");
    }

    #[test]
    fn signature() {
        let sig = Parser::new("()").parse_signature(None).unwrap();
        assert_eq!(sig.argument_types.len(), 0);
        assert_eq!(sig.return_types.len(), 0);

        let sig2 = Parser::new("(i8 inreg uext, f32, f64) -> i32 sext, f64")
            .parse_signature(None)
            .unwrap();
        assert_eq!(sig2.to_string(),
                   "(i8 uext inreg, f32, f64) -> i32 sext, f64");

        // `void` is not recognized as a type by the lexer. It should not appear in files.
        assert_eq!(Parser::new("() -> void").parse_signature(None).unwrap_err().to_string(),
                   "1: expected argument type");
        assert_eq!(Parser::new("i8 -> i8").parse_signature(None).unwrap_err().to_string(),
                   "1: expected function signature: ( args... )");
        assert_eq!(Parser::new("(i8 -> i8").parse_signature(None).unwrap_err().to_string(),
                   "1: expected ')' after function arguments");
    }

    #[test]
    fn stack_slot_decl() {
        let (func, _) = Parser::new("function foo() {
                                       ss3 = stack_slot 13
                                       ss1 = stack_slot 1
                                     }")
            .parse_function(None)
            .unwrap();
        assert_eq!(func.name.to_string(), "foo");
        let mut iter = func.stack_slots.keys();
        let ss0 = iter.next().unwrap();
        assert_eq!(ss0.to_string(), "ss0");
        assert_eq!(func.stack_slots[ss0].size, 13);
        let ss1 = iter.next().unwrap();
        assert_eq!(ss1.to_string(), "ss1");
        assert_eq!(func.stack_slots[ss1].size, 1);
        assert_eq!(iter.next(), None);

        // Catch duplicate definitions.
        assert_eq!(Parser::new("function bar() {
                                    ss1  = stack_slot 13
                                    ss1  = stack_slot 1
                                }")
                       .parse_function(None)
                       .unwrap_err()
                       .to_string(),
                   "3: duplicate stack slot: ss1");
    }

    #[test]
    fn ebb_header() {
        let (func, _) = Parser::new("function ebbs() {
                                     ebb0:
                                     ebb4(vx3: i32):
                                     }")
            .parse_function(None)
            .unwrap();
        assert_eq!(func.name.to_string(), "ebbs");

        let mut ebbs = func.layout.ebbs();

        let ebb0 = ebbs.next().unwrap();
        assert_eq!(func.dfg.ebb_args(ebb0).next(), None);

        let ebb4 = ebbs.next().unwrap();
        let mut ebb4_args = func.dfg.ebb_args(ebb4);
        let arg0 = ebb4_args.next().unwrap();
        assert_eq!(func.dfg.value_type(arg0), types::I32);
        assert_eq!(ebb4_args.next(), None);
    }

    #[test]
    fn comments() {
        let (func, Details { comments, .. }) =
            Parser::new("; before
                         function comment() { ; decl
                            ss10  = stack_slot 13 ; stackslot.
                            ; Still stackslot.
                            jt10 = jump_table ebb0
                            ; Jumptable
                         ebb0: ; Basic block
                         trap ; Instruction
                         } ; Trailing.
                         ; More trailing.")
                .parse_function(None)
                .unwrap();
        assert_eq!(func.name.to_string(), "comment");
        assert_eq!(comments.len(), 8); // no 'before' comment.
        assert_eq!(comments[0],
                   Comment {
                       entity: AnyEntity::Function,
                       text: "; decl",
                   });
        assert_eq!(comments[1].entity.to_string(), "ss0");
        assert_eq!(comments[2].entity.to_string(), "ss0");
        assert_eq!(comments[2].text, "; Still stackslot.");
        assert_eq!(comments[3].entity.to_string(), "jt0");
        assert_eq!(comments[3].text, "; Jumptable");
        assert_eq!(comments[4].entity.to_string(), "ebb0");
        assert_eq!(comments[4].text, "; Basic block");

        assert_eq!(comments[5].entity.to_string(), "inst0");
        assert_eq!(comments[5].text, "; Instruction");

        assert_eq!(comments[6].entity, AnyEntity::Function);
        assert_eq!(comments[7].entity, AnyEntity::Function);
    }

    #[test]
    fn test_file() {
        let tf = parse_test("; before
                             test cfg option=5
                             test verify
                             set enable_float=false
                             ; still preamble
                             function comment() {}")
            .unwrap();
        assert_eq!(tf.commands.len(), 2);
        assert_eq!(tf.commands[0].command, "cfg");
        assert_eq!(tf.commands[1].command, "verify");
        match tf.isa_spec {
            IsaSpec::None(s) => assert!(!s.enable_float()),
            _ => panic!("unexpected ISAs"),
        }
        assert_eq!(tf.preamble_comments.len(), 2);
        assert_eq!(tf.preamble_comments[0].text, "; before");
        assert_eq!(tf.preamble_comments[1].text, "; still preamble");
        assert_eq!(tf.functions.len(), 1);
        assert_eq!(tf.functions[0].0.name.to_string(), "comment");
    }

    #[test]
    fn isa_spec() {
        assert!(parse_test("isa
                            function foo() {}")
            .is_err());

        assert!(parse_test("isa riscv
                            set enable_float=false
                            function foo() {}")
            .is_err());

        match parse_test("set enable_float=false
                          isa riscv
                          function foo() {}")
            .unwrap()
            .isa_spec {
            IsaSpec::None(_) => panic!("Expected some ISA"),
            IsaSpec::Some(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].name(), "riscv");
            }
        }
    }
}

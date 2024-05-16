//! Parser for .clif files.

use crate::error::{Location, ParseError, ParseResult};
use crate::isaspec;
use crate::lexer::{LexError, Lexer, LocatedError, LocatedToken, Token};
use crate::run_command::{Comparison, Invocation, RunCommand};
use crate::sourcemap::SourceMap;
use crate::testcommand::TestCommand;
use crate::testfile::{Comment, Details, Feature, TestFile};
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::entity::{EntityRef, PrimaryMap};
use cranelift_codegen::ir::entities::{AnyEntity, DynamicType, MemoryType};
use cranelift_codegen::ir::immediates::{Ieee32, Ieee64, Imm64, Offset32, Uimm32, Uimm64};
use cranelift_codegen::ir::instructions::{InstructionData, InstructionFormat, VariableArgs};
use cranelift_codegen::ir::pcc::{BaseExpr, Expr, Fact};
use cranelift_codegen::ir::types;
use cranelift_codegen::ir::types::*;
use cranelift_codegen::ir::{self, UserExternalNameRef};
use cranelift_codegen::ir::{
    AbiParam, ArgumentExtension, ArgumentPurpose, Block, Constant, ConstantData, DynamicStackSlot,
    DynamicStackSlotData, DynamicTypeData, ExtFuncData, ExternalName, FuncRef, Function,
    GlobalValue, GlobalValueData, JumpTableData, MemFlags, MemoryTypeData, MemoryTypeField, Opcode,
    SigRef, Signature, StackSlot, StackSlotData, StackSlotKind, UserFuncName, Value,
};
use cranelift_codegen::isa::{self, CallConv};
use cranelift_codegen::packed_option::ReservedValue;
use cranelift_codegen::{settings, settings::Configurable, timing};
use smallvec::SmallVec;
use std::mem;
use std::str::FromStr;
use std::{u16, u32};
use target_lexicon::Triple;

macro_rules! match_imm {
    ($signed:ty, $unsigned:ty, $parser:expr, $err_msg:expr) => {{
        if let Some(Token::Integer(text)) = $parser.token() {
            $parser.consume();
            let negative = text.starts_with('-');
            let positive = text.starts_with('+');
            let text = if negative || positive {
                // Strip sign prefix.
                &text[1..]
            } else {
                text
            };

            // Parse the text value; the lexer gives us raw text that looks like an integer.
            let value = if text.starts_with("0x") {
                // Skip underscores.
                let text = text.replace("_", "");
                // Parse it in hexadecimal form.
                <$unsigned>::from_str_radix(&text[2..], 16).map_err(|_| {
                    $parser.error(&format!(
                        "unable to parse '{}' value as a hexadecimal {} immediate",
                        &text[2..],
                        stringify!($unsigned),
                    ))
                })?
            } else {
                // Parse it as a signed type to check for overflow and other issues.
                text.parse()
                    .map_err(|_| $parser.error("expected decimal immediate"))?
            };

            // Apply sign if necessary.
            let signed = if negative {
                let value = value.wrapping_neg() as $signed;
                if value > 0 {
                    return Err($parser.error("negative number too small"));
                }
                value
            } else {
                value as $signed
            };

            Ok(signed)
        } else {
            err!($parser.loc, $err_msg)
        }
    }};
}

/// After some quick benchmarks a program should never have more than 100,000 blocks.
const MAX_BLOCKS_IN_A_FUNCTION: u32 = 100_000;

/// Parse the entire `text` into a list of functions.
///
/// Any test commands or target declarations are ignored.
pub fn parse_functions(text: &str) -> ParseResult<Vec<Function>> {
    let _tt = timing::parse_text();
    parse_test(text, ParseOptions::default())
        .map(|file| file.functions.into_iter().map(|(func, _)| func).collect())
}

/// Options for configuring the parsing of filetests.
pub struct ParseOptions<'a> {
    /// Compiler passes to run on the parsed functions.
    pub passes: Option<&'a [String]>,
    /// Target ISA for compiling the parsed functions, e.g. "x86_64 skylake".
    pub target: Option<&'a str>,
    /// Default calling convention used when none is specified for a parsed function.
    pub default_calling_convention: CallConv,
    /// Default for unwind-info setting (enabled or disabled).
    pub unwind_info: bool,
    /// Default for machine_code_cfg_info setting (enabled or disabled).
    pub machine_code_cfg_info: bool,
}

impl Default for ParseOptions<'_> {
    fn default() -> Self {
        Self {
            passes: None,
            target: None,
            default_calling_convention: CallConv::Fast,
            unwind_info: false,
            machine_code_cfg_info: false,
        }
    }
}

/// Parse the entire `text` as a test case file.
///
/// The returned `TestFile` contains direct references to substrings of `text`.
pub fn parse_test<'a>(text: &'a str, options: ParseOptions<'a>) -> ParseResult<TestFile<'a>> {
    let _tt = timing::parse_text();
    let mut parser = Parser::new(text);

    // Gather the preamble comments.
    parser.start_gathering_comments();

    let isa_spec: isaspec::IsaSpec;
    let commands: Vec<TestCommand<'a>>;

    // Check for specified passes and target, if present throw out test commands/targets specified
    // in file.
    match options.passes {
        Some(pass_vec) => {
            parser.parse_test_commands();
            commands = parser.parse_cmdline_passes(pass_vec);
            parser.parse_target_specs(&options)?;
            isa_spec = parser.parse_cmdline_target(options.target)?;
        }
        None => {
            commands = parser.parse_test_commands();
            isa_spec = parser.parse_target_specs(&options)?;
        }
    };
    let features = parser.parse_cranelift_features()?;

    // Decide between using the calling convention passed in the options or using the
    // host's calling convention--if any tests are to be run on the host we should default to the
    // host's calling convention.
    parser = if commands.iter().any(|tc| tc.command == "run") {
        let host_default_calling_convention = CallConv::triple_default(&Triple::host());
        parser.with_default_calling_convention(host_default_calling_convention)
    } else {
        parser.with_default_calling_convention(options.default_calling_convention)
    };

    parser.token();
    parser.claim_gathered_comments(AnyEntity::Function);

    let preamble_comments = parser.take_comments();
    let functions = parser.parse_function_list()?;

    Ok(TestFile {
        commands,
        isa_spec,
        features,
        preamble_comments,
        functions,
    })
}

/// Parse a CLIF comment `text` as a run command.
///
/// Return:
///  - `Ok(None)` if the comment is not intended to be a `RunCommand` (i.e. does not start with `run`
///    or `print`
///  - `Ok(Some(command))` if the comment is intended as a `RunCommand` and can be parsed to one
///  - `Err` otherwise.
pub fn parse_run_command<'a>(text: &str, signature: &Signature) -> ParseResult<Option<RunCommand>> {
    let _tt = timing::parse_text();
    // We remove leading spaces and semi-colons for convenience here instead of at the call sites
    // since this function will be attempting to parse a RunCommand from a CLIF comment.
    let trimmed_text = text.trim_start_matches(|c| c == ' ' || c == ';');
    let mut parser = Parser::new(trimmed_text);
    match parser.token() {
        Some(Token::Identifier("run")) | Some(Token::Identifier("print")) => {
            parser.parse_run_command(signature).map(|c| Some(c))
        }
        Some(_) | None => Ok(None),
    }
}

pub struct Parser<'a> {
    lex: Lexer<'a>,

    lex_error: Option<LexError>,

    /// Current lookahead token.
    lookahead: Option<Token<'a>>,

    /// Location of lookahead.
    loc: Location,

    /// Are we gathering any comments that we encounter?
    gathering_comments: bool,

    /// The gathered comments; claim them with `claim_gathered_comments`.
    gathered_comments: Vec<&'a str>,

    /// Comments collected so far.
    comments: Vec<Comment<'a>>,

    /// Maps inlined external names to a ref value, so they can be declared before parsing the rest
    /// of the function later.
    ///
    /// This maintains backward compatibility with previous ways for declaring external names.
    predeclared_external_names: PrimaryMap<UserExternalNameRef, ir::UserExternalName>,

    /// Default calling conventions; used when none is specified.
    default_calling_convention: CallConv,
}

/// Context for resolving references when parsing a single function.
struct Context {
    function: Function,
    map: SourceMap,

    /// Aliases to resolve once value definitions are known.
    aliases: Vec<Value>,
}

impl Context {
    fn new(f: Function) -> Self {
        Self {
            function: f,
            map: SourceMap::new(),
            aliases: Vec::new(),
        }
    }

    // Allocate a new stack slot.
    fn add_ss(&mut self, ss: StackSlot, data: StackSlotData, loc: Location) -> ParseResult<()> {
        self.map.def_ss(ss, loc)?;
        while self.function.sized_stack_slots.next_key().index() <= ss.index() {
            self.function.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                0,
                0,
            ));
        }
        self.function.sized_stack_slots[ss] = data;
        Ok(())
    }

    // Resolve a reference to a stack slot.
    fn check_ss(&self, ss: StackSlot, loc: Location) -> ParseResult<()> {
        if !self.map.contains_ss(ss) {
            err!(loc, "undefined stack slot {}", ss)
        } else {
            Ok(())
        }
    }

    // Allocate a new stack slot.
    fn add_dss(
        &mut self,
        ss: DynamicStackSlot,
        data: DynamicStackSlotData,
        loc: Location,
    ) -> ParseResult<()> {
        self.map.def_dss(ss, loc)?;
        while self.function.dynamic_stack_slots.next_key().index() <= ss.index() {
            self.function
                .create_dynamic_stack_slot(DynamicStackSlotData::new(
                    StackSlotKind::ExplicitDynamicSlot,
                    data.dyn_ty,
                ));
        }
        self.function.dynamic_stack_slots[ss] = data;
        Ok(())
    }

    // Resolve a reference to a dynamic stack slot.
    fn check_dss(&self, dss: DynamicStackSlot, loc: Location) -> ParseResult<()> {
        if !self.map.contains_dss(dss) {
            err!(loc, "undefined dynamic stack slot {}", dss)
        } else {
            Ok(())
        }
    }

    // Allocate a new dynamic type.
    fn add_dt(&mut self, dt: DynamicType, data: DynamicTypeData, loc: Location) -> ParseResult<()> {
        self.map.def_dt(dt, loc)?;
        while self.function.dfg.dynamic_types.next_key().index() <= dt.index() {
            self.function.dfg.make_dynamic_ty(DynamicTypeData::new(
                data.base_vector_ty,
                data.dynamic_scale,
            ));
        }
        self.function.dfg.dynamic_types[dt] = data;
        Ok(())
    }

    // Allocate a global value slot.
    fn add_gv(
        &mut self,
        gv: GlobalValue,
        data: GlobalValueData,
        maybe_fact: Option<Fact>,
        loc: Location,
    ) -> ParseResult<()> {
        self.map.def_gv(gv, loc)?;
        while self.function.global_values.next_key().index() <= gv.index() {
            self.function.create_global_value(GlobalValueData::Symbol {
                name: ExternalName::testcase(""),
                offset: Imm64::new(0),
                colocated: false,
                tls: false,
            });
        }
        self.function.global_values[gv] = data;
        if let Some(fact) = maybe_fact {
            self.function.global_value_facts[gv] = Some(fact);
        }
        Ok(())
    }

    // Allocate a memory-type slot.
    fn add_mt(&mut self, mt: MemoryType, data: MemoryTypeData, loc: Location) -> ParseResult<()> {
        self.map.def_mt(mt, loc)?;
        while self.function.memory_types.next_key().index() <= mt.index() {
            self.function.create_memory_type(MemoryTypeData::default());
        }
        self.function.memory_types[mt] = data;
        Ok(())
    }

    // Resolve a reference to a global value.
    fn check_gv(&self, gv: GlobalValue, loc: Location) -> ParseResult<()> {
        if !self.map.contains_gv(gv) {
            err!(loc, "undefined global value {}", gv)
        } else {
            Ok(())
        }
    }

    // Allocate a new signature.
    fn add_sig(
        &mut self,
        sig: SigRef,
        data: Signature,
        loc: Location,
        defaultcc: CallConv,
    ) -> ParseResult<()> {
        self.map.def_sig(sig, loc)?;
        while self.function.dfg.signatures.next_key().index() <= sig.index() {
            self.function.import_signature(Signature::new(defaultcc));
        }
        self.function.dfg.signatures[sig] = data;
        Ok(())
    }

    // Resolve a reference to a signature.
    fn check_sig(&self, sig: SigRef, loc: Location) -> ParseResult<()> {
        if !self.map.contains_sig(sig) {
            err!(loc, "undefined signature {}", sig)
        } else {
            Ok(())
        }
    }

    // Allocate a new external function.
    fn add_fn(&mut self, fn_: FuncRef, data: ExtFuncData, loc: Location) -> ParseResult<()> {
        self.map.def_fn(fn_, loc)?;
        while self.function.dfg.ext_funcs.next_key().index() <= fn_.index() {
            self.function.import_function(ExtFuncData {
                name: ExternalName::testcase(""),
                signature: SigRef::reserved_value(),
                colocated: false,
            });
        }
        self.function.dfg.ext_funcs[fn_] = data;
        Ok(())
    }

    // Resolve a reference to a function.
    fn check_fn(&self, fn_: FuncRef, loc: Location) -> ParseResult<()> {
        if !self.map.contains_fn(fn_) {
            err!(loc, "undefined function {}", fn_)
        } else {
            Ok(())
        }
    }

    // Allocate a new constant.
    fn add_constant(
        &mut self,
        constant: Constant,
        data: ConstantData,
        loc: Location,
    ) -> ParseResult<()> {
        self.map.def_constant(constant, loc)?;
        self.function.dfg.constants.set(constant, data);
        Ok(())
    }

    // Configure the stack limit of the current function.
    fn add_stack_limit(&mut self, limit: GlobalValue, loc: Location) -> ParseResult<()> {
        if self.function.stack_limit.is_some() {
            return err!(loc, "stack limit defined twice");
        }
        self.function.stack_limit = Some(limit);
        Ok(())
    }

    // Resolve a reference to a constant.
    fn check_constant(&self, c: Constant, loc: Location) -> ParseResult<()> {
        if !self.map.contains_constant(c) {
            err!(loc, "undefined constant {}", c)
        } else {
            Ok(())
        }
    }

    // Allocate a new block.
    fn add_block(&mut self, block: Block, loc: Location) -> ParseResult<Block> {
        self.map.def_block(block, loc)?;
        while self.function.dfg.num_blocks() <= block.index() {
            self.function.dfg.make_block();
        }
        self.function.layout.append_block(block);
        Ok(block)
    }

    /// Set a block as cold.
    fn set_cold_block(&mut self, block: Block) {
        self.function.layout.set_cold(block);
    }
}

impl<'a> Parser<'a> {
    /// Create a new `Parser` which reads `text`. The referenced text must outlive the parser.
    pub fn new(text: &'a str) -> Self {
        Self {
            lex: Lexer::new(text),
            lex_error: None,
            lookahead: None,
            loc: Location { line_number: 0 },
            gathering_comments: false,
            gathered_comments: Vec::new(),
            comments: Vec::new(),
            default_calling_convention: CallConv::Fast,
            predeclared_external_names: Default::default(),
        }
    }

    /// Modify the default calling convention; returns a new parser with the changed calling
    /// convention.
    pub fn with_default_calling_convention(self, default_calling_convention: CallConv) -> Self {
        Self {
            default_calling_convention,
            ..self
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
        while self.lookahead.is_none() {
            match self.lex.next() {
                Some(Ok(LocatedToken { token, location })) => {
                    match token {
                        Token::Comment(text) => {
                            if self.gathering_comments {
                                self.gathered_comments.push(text);
                            }
                        }
                        _ => self.lookahead = Some(token),
                    }
                    self.loc = location;
                }
                Some(Err(LocatedError { error, location })) => {
                    self.lex_error = Some(error);
                    self.loc = location;
                    break;
                }
                None => break,
            }
        }
        self.lookahead
    }

    // Enable gathering of all comments encountered.
    fn start_gathering_comments(&mut self) {
        debug_assert!(!self.gathering_comments);
        self.gathering_comments = true;
        debug_assert!(self.gathered_comments.is_empty());
    }

    // Claim the comments gathered up to the current position for the
    // given entity.
    fn claim_gathered_comments<E: Into<AnyEntity>>(&mut self, entity: E) {
        debug_assert!(self.gathering_comments);
        let entity = entity.into();
        self.comments.extend(
            self.gathered_comments
                .drain(..)
                .map(|text| Comment { entity, text }),
        );
        self.gathering_comments = false;
    }

    // Get the comments collected so far, clearing out the internal list.
    fn take_comments(&mut self) -> Vec<Comment<'a>> {
        debug_assert!(!self.gathering_comments);
        mem::replace(&mut self.comments, Vec::new())
    }

    // Match and consume a token without payload.
    fn match_token(&mut self, want: Token<'a>, err_msg: &str) -> ParseResult<Token<'a>> {
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
    fn match_identifier(&mut self, want: &'static str, err_msg: &str) -> ParseResult<Token<'a>> {
        if self.token() == Some(Token::Identifier(want)) {
            Ok(self.consume())
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume a type.
    fn match_type(&mut self, err_msg: &str) -> ParseResult<Type> {
        if let Some(Token::Type(t)) = self.token() {
            self.consume();
            Ok(t)
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume a stack slot reference.
    fn match_ss(&mut self, err_msg: &str) -> ParseResult<StackSlot> {
        if let Some(Token::StackSlot(ss)) = self.token() {
            self.consume();
            if let Some(ss) = StackSlot::with_number(ss) {
                return Ok(ss);
            }
        }
        err!(self.loc, err_msg)
    }

    // Match and consume a dynamic stack slot reference.
    fn match_dss(&mut self, err_msg: &str) -> ParseResult<DynamicStackSlot> {
        if let Some(Token::DynamicStackSlot(ss)) = self.token() {
            self.consume();
            if let Some(ss) = DynamicStackSlot::with_number(ss) {
                return Ok(ss);
            }
        }
        err!(self.loc, err_msg)
    }

    // Match and consume a dynamic type reference.
    fn match_dt(&mut self, err_msg: &str) -> ParseResult<DynamicType> {
        if let Some(Token::DynamicType(dt)) = self.token() {
            self.consume();
            if let Some(dt) = DynamicType::with_number(dt) {
                return Ok(dt);
            }
        }
        err!(self.loc, err_msg)
    }

    // Extract Type from DynamicType
    fn concrete_from_dt(&mut self, dt: DynamicType, ctx: &mut Context) -> Option<Type> {
        ctx.function.get_concrete_dynamic_ty(dt)
    }

    // Match and consume a global value reference.
    fn match_gv(&mut self, err_msg: &str) -> ParseResult<GlobalValue> {
        if let Some(Token::GlobalValue(gv)) = self.token() {
            self.consume();
            if let Some(gv) = GlobalValue::with_number(gv) {
                return Ok(gv);
            }
        }
        err!(self.loc, err_msg)
    }

    // Match and consume a function reference.
    fn match_fn(&mut self, err_msg: &str) -> ParseResult<FuncRef> {
        if let Some(Token::FuncRef(fnref)) = self.token() {
            self.consume();
            if let Some(fnref) = FuncRef::with_number(fnref) {
                return Ok(fnref);
            }
        }
        err!(self.loc, err_msg)
    }

    // Match and consume a signature reference.
    fn match_sig(&mut self, err_msg: &str) -> ParseResult<SigRef> {
        if let Some(Token::SigRef(sigref)) = self.token() {
            self.consume();
            if let Some(sigref) = SigRef::with_number(sigref) {
                return Ok(sigref);
            }
        }
        err!(self.loc, err_msg)
    }

    // Match and consume a memory-type reference.
    fn match_mt(&mut self, err_msg: &str) -> ParseResult<MemoryType> {
        if let Some(Token::MemoryType(mt)) = self.token() {
            self.consume();
            if let Some(mt) = MemoryType::with_number(mt) {
                return Ok(mt);
            }
        }
        err!(self.loc, err_msg)
    }

    // Match and consume a constant reference.
    fn match_constant(&mut self) -> ParseResult<Constant> {
        if let Some(Token::Constant(c)) = self.token() {
            self.consume();
            if let Some(c) = Constant::with_number(c) {
                return Ok(c);
            }
        }
        err!(self.loc, "expected constant number: const«n»")
    }

    // Match and consume a stack limit token
    fn match_stack_limit(&mut self) -> ParseResult<()> {
        if let Some(Token::Identifier("stack_limit")) = self.token() {
            self.consume();
            return Ok(());
        }
        err!(self.loc, "expected identifier: stack_limit")
    }

    // Match and consume a block reference.
    fn match_block(&mut self, err_msg: &str) -> ParseResult<Block> {
        if let Some(Token::Block(block)) = self.token() {
            self.consume();
            Ok(block)
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume a value reference.
    fn match_value(&mut self, err_msg: &str) -> ParseResult<Value> {
        if let Some(Token::Value(v)) = self.token() {
            self.consume();
            Ok(v)
        } else {
            err!(self.loc, err_msg)
        }
    }

    fn error(&self, message: &str) -> ParseError {
        ParseError {
            location: self.loc,
            message: message.to_string(),
            is_warning: false,
        }
    }

    // Match and consume an Imm64 immediate.
    fn match_imm64(&mut self, err_msg: &str) -> ParseResult<Imm64> {
        if let Some(Token::Integer(text)) = self.token() {
            self.consume();
            // Lexer just gives us raw text that looks like an integer.
            // Parse it as an Imm64 to check for overflow and other issues.
            text.parse().map_err(|e| self.error(e))
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume a hexadeximal immediate
    fn match_hexadecimal_constant(&mut self, err_msg: &str) -> ParseResult<ConstantData> {
        if let Some(Token::Integer(text)) = self.token() {
            self.consume();
            text.parse().map_err(|e| {
                self.error(&format!(
                    "expected hexadecimal immediate, failed to parse: {}",
                    e
                ))
            })
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume either a hexadecimal Uimm128 immediate (e.g. 0x000102...) or its literal
    // list form (e.g. [0 1 2...]). For convenience, since uimm128 values are stored in the
    // `ConstantPool`, this returns `ConstantData`.
    fn match_uimm128(&mut self, controlling_type: Type) -> ParseResult<ConstantData> {
        let expected_size = controlling_type.bytes() as usize;
        let constant_data = if self.optional(Token::LBracket) {
            // parse using a list of values, e.g. vconst.i32x4 [0 1 2 3]
            let uimm128 = self.parse_literals_to_constant_data(controlling_type)?;
            self.match_token(Token::RBracket, "expected a terminating right bracket")?;
            uimm128
        } else {
            // parse using a hexadecimal value, e.g. 0x000102...
            let uimm128 =
                self.match_hexadecimal_constant("expected an immediate hexadecimal operand")?;
            uimm128.expand_to(expected_size)
        };

        if constant_data.len() == expected_size {
            Ok(constant_data)
        } else {
            Err(self.error(&format!(
                "expected parsed constant to have {} bytes",
                expected_size
            )))
        }
    }

    // Match and consume a Uimm64 immediate.
    fn match_uimm64(&mut self, err_msg: &str) -> ParseResult<Uimm64> {
        if let Some(Token::Integer(text)) = self.token() {
            self.consume();
            // Lexer just gives us raw text that looks like an integer.
            // Parse it as an Uimm64 to check for overflow and other issues.
            text.parse()
                .map_err(|_| self.error("expected u64 decimal immediate"))
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume a Uimm32 immediate.
    fn match_uimm32(&mut self, err_msg: &str) -> ParseResult<Uimm32> {
        if let Some(Token::Integer(text)) = self.token() {
            self.consume();
            // Lexer just gives us raw text that looks like an integer.
            // Parse it as an Uimm32 to check for overflow and other issues.
            text.parse().map_err(|e| self.error(e))
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume a u8 immediate.
    // This is used for lane numbers in SIMD vectors.
    fn match_uimm8(&mut self, err_msg: &str) -> ParseResult<u8> {
        if let Some(Token::Integer(text)) = self.token() {
            self.consume();
            // Lexer just gives us raw text that looks like an integer.
            if text.starts_with("0x") {
                // Parse it as a u8 in hexadecimal form.
                u8::from_str_radix(&text[2..], 16)
                    .map_err(|_| self.error("unable to parse u8 as a hexadecimal immediate"))
            } else {
                // Parse it as a u8 to check for overflow and other issues.
                text.parse()
                    .map_err(|_| self.error("expected u8 decimal immediate"))
            }
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and consume an i8 immediate.
    fn match_imm8(&mut self, err_msg: &str) -> ParseResult<i8> {
        match_imm!(i8, u8, self, err_msg)
    }

    // Match and consume a signed 16-bit immediate.
    fn match_imm16(&mut self, err_msg: &str) -> ParseResult<i16> {
        match_imm!(i16, u16, self, err_msg)
    }

    // Match and consume an i32 immediate.
    // This is used for stack argument byte offsets.
    fn match_imm32(&mut self, err_msg: &str) -> ParseResult<i32> {
        match_imm!(i32, u32, self, err_msg)
    }

    // Match and consume an i128 immediate.
    fn match_imm128(&mut self, err_msg: &str) -> ParseResult<i128> {
        match_imm!(i128, u128, self, err_msg)
    }

    // Match and consume an optional offset32 immediate.
    //
    // Note that this will match an empty string as an empty offset, and that if an offset is
    // present, it must contain a sign.
    fn optional_offset32(&mut self) -> ParseResult<Offset32> {
        if let Some(Token::Integer(text)) = self.token() {
            if text.starts_with('+') || text.starts_with('-') {
                self.consume();
                // Lexer just gives us raw text that looks like an integer.
                // Parse it as an `Offset32` to check for overflow and other issues.
                return text.parse().map_err(|e| self.error(e));
            }
        }
        // An offset32 operand can be absent.
        Ok(Offset32::new(0))
    }

    // Match and consume an optional offset32 immediate.
    //
    // Note that this will match an empty string as an empty offset, and that if an offset is
    // present, it must contain a sign.
    fn optional_offset_imm64(&mut self) -> ParseResult<Imm64> {
        if let Some(Token::Integer(text)) = self.token() {
            if text.starts_with('+') || text.starts_with('-') {
                self.consume();
                // Lexer just gives us raw text that looks like an integer.
                // Parse it as an `Offset32` to check for overflow and other issues.
                return text.parse().map_err(|e| self.error(e));
            }
        }
        // If no explicit offset is present, the offset is 0.
        Ok(Imm64::new(0))
    }

    // Match and consume an Ieee32 immediate.
    fn match_ieee32(&mut self, err_msg: &str) -> ParseResult<Ieee32> {
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
    fn match_ieee64(&mut self, err_msg: &str) -> ParseResult<Ieee64> {
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
    fn match_enum<T: FromStr>(&mut self, err_msg: &str) -> ParseResult<T> {
        if let Some(Token::Identifier(text)) = self.token() {
            self.consume();
            text.parse().map_err(|_| self.error(err_msg))
        } else {
            err!(self.loc, err_msg)
        }
    }

    // Match and a consume a possibly empty sequence of memory operation flags.
    fn optional_memflags(&mut self) -> ParseResult<MemFlags> {
        let mut flags = MemFlags::new();
        while let Some(Token::Identifier(text)) = self.token() {
            match flags.set_by_name(text) {
                Ok(true) => {
                    self.consume();
                }
                Ok(false) => break,
                Err(msg) => return err!(self.loc, msg),
            }
        }
        Ok(flags)
    }

    // Match and consume an identifier.
    fn match_any_identifier(&mut self, err_msg: &str) -> ParseResult<&'a str> {
        if let Some(Token::Identifier(text)) = self.token() {
            self.consume();
            Ok(text)
        } else {
            err!(self.loc, err_msg)
        }
    }

    /// Parse an optional source location.
    ///
    /// Return an optional source location if no real location is present.
    fn optional_srcloc(&mut self) -> ParseResult<ir::SourceLoc> {
        if let Some(Token::SourceLoc(text)) = self.token() {
            match u32::from_str_radix(text, 16) {
                Ok(num) => {
                    self.consume();
                    Ok(ir::SourceLoc::new(num))
                }
                Err(_) => return err!(self.loc, "invalid source location: {}", text),
            }
        } else {
            Ok(Default::default())
        }
    }

    /// Parse a list of literals (i.e. integers, floats, booleans); e.g. `0 1 2 3`, usually as
    /// part of something like `vconst.i32x4 [0 1 2 3]`.
    fn parse_literals_to_constant_data(&mut self, ty: Type) -> ParseResult<ConstantData> {
        macro_rules! consume {
            ( $ty:ident, $match_fn:expr ) => {{
                assert!($ty.is_vector());
                let mut data = ConstantData::default();
                for _ in 0..$ty.lane_count() {
                    data = data.append($match_fn);
                }
                data
            }};
        }

        if !ty.is_vector() && !ty.is_dynamic_vector() {
            err!(self.loc, "Expected a controlling vector type, not {}", ty)
        } else {
            let constant_data = match ty.lane_type() {
                I8 => consume!(ty, self.match_imm8("Expected an 8-bit integer")?),
                I16 => consume!(ty, self.match_imm16("Expected a 16-bit integer")?),
                I32 => consume!(ty, self.match_imm32("Expected a 32-bit integer")?),
                I64 => consume!(ty, self.match_imm64("Expected a 64-bit integer")?),
                F32 => consume!(ty, self.match_ieee32("Expected a 32-bit float")?),
                F64 => consume!(ty, self.match_ieee64("Expected a 64-bit float")?),
                _ => return err!(self.loc, "Expected a type of: float, int, bool"),
            };
            Ok(constant_data)
        }
    }

    /// Parse a list of test command passes specified in command line.
    pub fn parse_cmdline_passes(&mut self, passes: &'a [String]) -> Vec<TestCommand<'a>> {
        let mut list = Vec::new();
        for pass in passes {
            list.push(TestCommand::new(pass));
        }
        list
    }

    /// Parse a list of test commands.
    pub fn parse_test_commands(&mut self) -> Vec<TestCommand<'a>> {
        let mut list = Vec::new();
        while self.token() == Some(Token::Identifier("test")) {
            list.push(TestCommand::new(self.consume_line()));
        }
        list
    }

    /// Parse a target spec.
    ///
    /// Accept the target from the command line for pass command.
    ///
    fn parse_cmdline_target(&mut self, target_pass: Option<&str>) -> ParseResult<isaspec::IsaSpec> {
        // Were there any `target` commands specified?
        let mut specified_target = false;

        let mut targets = Vec::new();
        let flag_builder = settings::builder();

        if let Some(targ) = target_pass {
            let loc = self.loc;
            let triple = match Triple::from_str(targ) {
                Ok(triple) => triple,
                Err(err) => return err!(loc, err),
            };
            let isa_builder = match isa::lookup(triple) {
                Err(isa::LookupError::SupportDisabled) => {
                    return err!(loc, "support disabled target '{}'", targ);
                }
                Err(isa::LookupError::Unsupported) => {
                    return warn!(loc, "unsupported target '{}'", targ);
                }
                Ok(b) => b,
            };
            specified_target = true;

            // Construct a trait object with the aggregate settings.
            targets.push(
                isa_builder
                    .finish(settings::Flags::new(flag_builder.clone()))
                    .map_err(|e| ParseError {
                        location: loc,
                        message: format!("invalid ISA flags for '{}': {:?}", targ, e),
                        is_warning: false,
                    })?,
            );
        }

        if !specified_target {
            // No `target` commands.
            Ok(isaspec::IsaSpec::None(settings::Flags::new(flag_builder)))
        } else {
            Ok(isaspec::IsaSpec::Some(targets))
        }
    }

    /// Parse a list of target specs.
    ///
    /// Accept a mix of `target` and `set` command lines. The `set` commands are cumulative.
    ///
    fn parse_target_specs(&mut self, options: &ParseOptions) -> ParseResult<isaspec::IsaSpec> {
        // Were there any `target` commands?
        let mut seen_target = false;
        // Location of last `set` command since the last `target`.
        let mut last_set_loc = None;

        let mut targets = Vec::new();
        let mut flag_builder = settings::builder();

        let bool_to_str = |val: bool| {
            if val {
                "true"
            } else {
                "false"
            }
        };

        // default to enabling cfg info
        flag_builder
            .set(
                "machine_code_cfg_info",
                bool_to_str(options.machine_code_cfg_info),
            )
            .expect("machine_code_cfg_info option should be present");

        flag_builder
            .set("unwind_info", bool_to_str(options.unwind_info))
            .expect("unwind_info option should be present");

        while let Some(Token::Identifier(command)) = self.token() {
            match command {
                "set" => {
                    last_set_loc = Some(self.loc);
                    isaspec::parse_options(
                        self.consume_line().trim().split_whitespace(),
                        &mut flag_builder,
                        self.loc,
                    )
                    .map_err(|err| ParseError::from(err))?;
                }
                "target" => {
                    let loc = self.loc;
                    // Grab the whole line so the lexer won't go looking for tokens on the
                    // following lines.
                    let mut words = self.consume_line().trim().split_whitespace().peekable();
                    // Look for `target foo`.
                    let target_name = match words.next() {
                        Some(w) => w,
                        None => return err!(loc, "expected target triple"),
                    };
                    let triple = match Triple::from_str(target_name) {
                        Ok(triple) => triple,
                        Err(err) => return err!(loc, err),
                    };
                    let mut isa_builder = match isa::lookup(triple) {
                        Err(isa::LookupError::SupportDisabled) => {
                            continue;
                        }
                        Err(isa::LookupError::Unsupported) => {
                            return warn!(loc, "unsupported target '{}'", target_name);
                        }
                        Ok(b) => b,
                    };
                    last_set_loc = None;
                    seen_target = true;
                    // Apply the target-specific settings to `isa_builder`.
                    isaspec::parse_options(words, &mut isa_builder, self.loc)?;

                    // Construct a trait object with the aggregate settings.
                    targets.push(
                        isa_builder
                            .finish(settings::Flags::new(flag_builder.clone()))
                            .map_err(|e| ParseError {
                                location: loc,
                                message: format!(
                                    "invalid ISA flags for '{}': {:?}",
                                    target_name, e
                                ),
                                is_warning: false,
                            })?,
                    );
                }
                _ => break,
            }
        }

        if !seen_target {
            // No `target` commands, but we allow for `set` commands.
            Ok(isaspec::IsaSpec::None(settings::Flags::new(flag_builder)))
        } else if let Some(loc) = last_set_loc {
            err!(
                loc,
                "dangling 'set' command after ISA specification has no effect."
            )
        } else {
            Ok(isaspec::IsaSpec::Some(targets))
        }
    }

    /// Parse a list of expected features that Cranelift should be compiled with, or without.
    pub fn parse_cranelift_features(&mut self) -> ParseResult<Vec<Feature<'a>>> {
        let mut list = Vec::new();
        while self.token() == Some(Token::Identifier("feature")) {
            self.consume();
            let has = !self.optional(Token::Bang);
            match (self.token(), has) {
                (Some(Token::String(flag)), true) => list.push(Feature::With(flag)),
                (Some(Token::String(flag)), false) => list.push(Feature::Without(flag)),
                (tok, _) => {
                    return err!(
                        self.loc,
                        format!("Expected feature flag string, got {:?}", tok)
                    )
                }
            }
            self.consume();
        }
        Ok(list)
    }

    /// Parse a list of function definitions.
    ///
    /// This is the top-level parse function matching the whole contents of a file.
    pub fn parse_function_list(&mut self) -> ParseResult<Vec<(Function, Details<'a>)>> {
        let mut list = Vec::new();
        while self.token().is_some() {
            list.push(self.parse_function()?);
        }
        if let Some(err) = self.lex_error {
            return match err {
                LexError::InvalidChar => err!(self.loc, "invalid character"),
            };
        }
        Ok(list)
    }

    // Parse a whole function definition.
    //
    // function ::= * "function" name signature "{" preamble function-body "}"
    //
    fn parse_function(&mut self) -> ParseResult<(Function, Details<'a>)> {
        // Begin gathering comments.
        // Make sure we don't include any comments before the `function` keyword.
        self.token();
        debug_assert!(self.comments.is_empty());
        self.start_gathering_comments();

        self.match_identifier("function", "expected 'function'")?;

        let location = self.loc;

        // function ::= "function" * name signature "{" preamble function-body "}"
        let name = self.parse_user_func_name()?;

        // function ::= "function" name * signature "{" preamble function-body "}"
        let sig = self.parse_signature()?;

        let mut ctx = Context::new(Function::with_name_signature(name, sig));

        // function ::= "function" name signature * "{" preamble function-body "}"
        self.match_token(Token::LBrace, "expected '{' before function body")?;

        self.token();
        self.claim_gathered_comments(AnyEntity::Function);

        // function ::= "function" name signature "{" * preamble function-body "}"
        self.parse_preamble(&mut ctx)?;
        // function ::= "function" name signature "{"  preamble * function-body "}"
        self.parse_function_body(&mut ctx)?;
        // function ::= "function" name signature "{" preamble function-body * "}"
        self.match_token(Token::RBrace, "expected '}' after function body")?;

        // Collect any comments following the end of the function, then stop gathering comments.
        self.start_gathering_comments();
        self.token();
        self.claim_gathered_comments(AnyEntity::Function);

        // Claim all the declared user-defined function names.
        for (user_func_ref, user_external_name) in
            std::mem::take(&mut self.predeclared_external_names)
        {
            let actual_ref = ctx
                .function
                .declare_imported_user_function(user_external_name);
            assert_eq!(user_func_ref, actual_ref);
        }

        let details = Details {
            location,
            comments: self.take_comments(),
            map: ctx.map,
        };

        Ok((ctx.function, details))
    }

    // Parse a user-defined function name
    //
    // For example, in a function decl, the parser would be in this state:
    //
    // function ::= "function" * name signature { ... }
    //
    fn parse_user_func_name(&mut self) -> ParseResult<UserFuncName> {
        match self.token() {
            Some(Token::Name(s)) => {
                self.consume();
                Ok(UserFuncName::testcase(s))
            }
            Some(Token::UserRef(namespace)) => {
                self.consume();
                match self.token() {
                    Some(Token::Colon) => {
                        self.consume();
                        match self.token() {
                            Some(Token::Integer(index_str)) => {
                                self.consume();
                                let index: u32 =
                                    u32::from_str_radix(index_str, 10).map_err(|_| {
                                        self.error("the integer given overflows the u32 type")
                                    })?;
                                Ok(UserFuncName::user(namespace, index))
                            }
                            _ => err!(self.loc, "expected integer"),
                        }
                    }
                    _ => {
                        err!(self.loc, "expected user function name in the form uX:Y")
                    }
                }
            }
            _ => err!(self.loc, "expected external name"),
        }
    }

    // Parse an external name.
    //
    // For example, in a function reference decl, the parser would be in this state:
    //
    // fn0 = * name signature
    //
    fn parse_external_name(&mut self) -> ParseResult<ExternalName> {
        match self.token() {
            Some(Token::Name(s)) => {
                self.consume();
                s.parse()
                    .map_err(|_| self.error("invalid test case or libcall name"))
            }

            Some(Token::UserNameRef(name_ref)) => {
                self.consume();
                Ok(ExternalName::user(UserExternalNameRef::new(
                    name_ref as usize,
                )))
            }

            Some(Token::UserRef(namespace)) => {
                self.consume();
                if let Some(Token::Colon) = self.token() {
                    self.consume();
                    match self.token() {
                        Some(Token::Integer(index_str)) => {
                            let index: u32 = u32::from_str_radix(index_str, 10).map_err(|_| {
                                self.error("the integer given overflows the u32 type")
                            })?;
                            self.consume();

                            // Deduplicate the reference (O(n), but should be fine for tests),
                            // to follow `FunctionParameters::declare_imported_user_function`,
                            // otherwise this will cause ref mismatches when asserted below.
                            let name_ref = self
                                .predeclared_external_names
                                .iter()
                                .find_map(|(reff, name)| {
                                    if name.index == index && name.namespace == namespace {
                                        Some(reff)
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or_else(|| {
                                    self.predeclared_external_names
                                        .push(ir::UserExternalName { namespace, index })
                                });

                            Ok(ExternalName::user(name_ref))
                        }
                        _ => err!(self.loc, "expected integer"),
                    }
                } else {
                    err!(self.loc, "expected colon")
                }
            }

            _ => err!(self.loc, "expected external name"),
        }
    }

    // Parse a function signature.
    //
    // signature ::=  * "(" [paramlist] ")" ["->" retlist] [callconv]
    //
    fn parse_signature(&mut self) -> ParseResult<Signature> {
        // Calling convention defaults to `fast`, but can be changed.
        let mut sig = Signature::new(self.default_calling_convention);

        self.match_token(Token::LPar, "expected function signature: ( args... )")?;
        // signature ::=  "(" * [abi-param-list] ")" ["->" retlist] [callconv]
        if self.token() != Some(Token::RPar) {
            sig.params = self.parse_abi_param_list()?;
        }
        self.match_token(Token::RPar, "expected ')' after function arguments")?;
        if self.optional(Token::Arrow) {
            sig.returns = self.parse_abi_param_list()?;
        }

        // The calling convention is optional.
        if let Some(Token::Identifier(text)) = self.token() {
            match text.parse() {
                Ok(cc) => {
                    self.consume();
                    sig.call_conv = cc;
                }
                _ => return err!(self.loc, "unknown calling convention: {}", text),
            }
        }

        Ok(sig)
    }

    // Parse list of function parameter / return value types.
    //
    // paramlist ::= * param { "," param }
    //
    fn parse_abi_param_list(&mut self) -> ParseResult<Vec<AbiParam>> {
        let mut list = Vec::new();

        // abi-param-list ::= * abi-param { "," abi-param }
        list.push(self.parse_abi_param()?);

        // abi-param-list ::= abi-param * { "," abi-param }
        while self.optional(Token::Comma) {
            // abi-param-list ::= abi-param { "," * abi-param }
            list.push(self.parse_abi_param()?);
        }

        Ok(list)
    }

    // Parse a single argument type with flags.
    fn parse_abi_param(&mut self) -> ParseResult<AbiParam> {
        // abi-param ::= * type { flag }
        let mut arg = AbiParam::new(self.match_type("expected parameter type")?);

        // abi-param ::= type * { flag }
        while let Some(Token::Identifier(s)) = self.token() {
            match s {
                "uext" => arg.extension = ArgumentExtension::Uext,
                "sext" => arg.extension = ArgumentExtension::Sext,
                "sarg" => {
                    self.consume();
                    self.match_token(Token::LPar, "expected '(' to begin sarg size")?;
                    let size = self.match_uimm32("expected byte-size in sarg decl")?;
                    self.match_token(Token::RPar, "expected ')' to end sarg size")?;
                    arg.purpose = ArgumentPurpose::StructArgument(size.into());
                    continue;
                }
                _ => {
                    if let Ok(purpose) = s.parse() {
                        arg.purpose = purpose;
                    } else {
                        break;
                    }
                }
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
    //                   * stack-limit-decl
    //
    // The parsed decls are added to `ctx` rather than returned.
    fn parse_preamble(&mut self, ctx: &mut Context) -> ParseResult<()> {
        loop {
            match self.token() {
                Some(Token::StackSlot(..)) => {
                    self.start_gathering_comments();
                    let loc = self.loc;
                    self.parse_stack_slot_decl()
                        .and_then(|(ss, dat)| ctx.add_ss(ss, dat, loc))
                }
                Some(Token::DynamicStackSlot(..)) => {
                    self.start_gathering_comments();
                    let loc = self.loc;
                    self.parse_dynamic_stack_slot_decl()
                        .and_then(|(dss, dat)| ctx.add_dss(dss, dat, loc))
                }
                Some(Token::DynamicType(..)) => {
                    self.start_gathering_comments();
                    let loc = self.loc;
                    self.parse_dynamic_type_decl()
                        .and_then(|(dt, dat)| ctx.add_dt(dt, dat, loc))
                }
                Some(Token::GlobalValue(..)) => {
                    self.start_gathering_comments();
                    self.parse_global_value_decl()
                        .and_then(|(gv, dat, maybe_fact)| ctx.add_gv(gv, dat, maybe_fact, self.loc))
                }
                Some(Token::MemoryType(..)) => {
                    self.start_gathering_comments();
                    self.parse_memory_type_decl()
                        .and_then(|(mt, dat)| ctx.add_mt(mt, dat, self.loc))
                }
                Some(Token::SigRef(..)) => {
                    self.start_gathering_comments();
                    self.parse_signature_decl().and_then(|(sig, dat)| {
                        ctx.add_sig(sig, dat, self.loc, self.default_calling_convention)
                    })
                }
                Some(Token::FuncRef(..)) => {
                    self.start_gathering_comments();
                    self.parse_function_decl(ctx)
                        .and_then(|(fn_, dat)| ctx.add_fn(fn_, dat, self.loc))
                }
                Some(Token::Constant(..)) => {
                    self.start_gathering_comments();
                    self.parse_constant_decl()
                        .and_then(|(c, v)| ctx.add_constant(c, v, self.loc))
                }
                Some(Token::Identifier("stack_limit")) => {
                    self.start_gathering_comments();
                    self.parse_stack_limit_decl()
                        .and_then(|gv| ctx.add_stack_limit(gv, self.loc))
                }
                // More to come..
                _ => return Ok(()),
            }?;
        }
    }

    // Parse a stack slot decl.
    //
    // stack-slot-decl ::= * StackSlot(ss) "=" stack-slot-kind Bytes {"," stack-slot-flag}
    // stack-slot-kind ::= "explicit_slot"
    //                   | "spill_slot"
    //                   | "incoming_arg"
    //                   | "outgoing_arg"
    // stack-slot-flag ::= "align" "=" Bytes
    fn parse_stack_slot_decl(&mut self) -> ParseResult<(StackSlot, StackSlotData)> {
        let ss = self.match_ss("expected stack slot number: ss«n»")?;
        self.match_token(Token::Equal, "expected '=' in stack slot declaration")?;
        let kind = self.match_enum("expected stack slot kind")?;

        // stack-slot-decl ::= StackSlot(ss) "=" stack-slot-kind * Bytes {"," stack-slot-flag}
        let bytes: i64 = self
            .match_imm64("expected byte-size in stack_slot decl")?
            .into();
        if bytes < 0 {
            return err!(self.loc, "negative stack slot size");
        }
        if bytes > i64::from(u32::MAX) {
            return err!(self.loc, "stack slot too large");
        }

        // Parse flags.
        let align = if self.token() == Some(Token::Comma) {
            self.consume();
            self.match_token(
                Token::Identifier("align"),
                "expected a valid stack-slot flag (currently only `align`)",
            )?;
            self.match_token(Token::Equal, "expected `=` after flag")?;
            let align: i64 = self
                .match_imm64("expected alignment-size after `align` flag")?
                .into();
            u32::try_from(align)
                .map_err(|_| self.error("alignment must be a 32-bit unsigned integer"))?
        } else {
            1
        };

        if !align.is_power_of_two() {
            return err!(self.loc, "stack slot alignment is not a power of two");
        }
        let align_shift = u8::try_from(align.ilog2()).unwrap(); // Always succeeds: range 0..=31.

        let data = StackSlotData::new(kind, bytes as u32, align_shift);

        // Collect any trailing comments.
        self.token();
        self.claim_gathered_comments(ss);

        // TBD: stack-slot-decl ::= StackSlot(ss) "=" stack-slot-kind Bytes * {"," stack-slot-flag}
        Ok((ss, data))
    }

    fn parse_dynamic_stack_slot_decl(
        &mut self,
    ) -> ParseResult<(DynamicStackSlot, DynamicStackSlotData)> {
        let dss = self.match_dss("expected stack slot number: dss«n»")?;
        self.match_token(Token::Equal, "expected '=' in stack slot declaration")?;
        let kind = self.match_enum("expected stack slot kind")?;
        let dt = self.match_dt("expected dynamic type")?;
        let data = DynamicStackSlotData::new(kind, dt);
        // Collect any trailing comments.
        self.token();
        self.claim_gathered_comments(dss);

        // TBD: stack-slot-decl ::= StackSlot(ss) "=" stack-slot-kind Bytes * {"," stack-slot-flag}
        Ok((dss, data))
    }

    fn parse_dynamic_type_decl(&mut self) -> ParseResult<(DynamicType, DynamicTypeData)> {
        let dt = self.match_dt("expected dynamic type number: dt«n»")?;
        self.match_token(Token::Equal, "expected '=' in stack slot declaration")?;
        let vector_base_ty = self.match_type("expected base type")?;
        assert!(vector_base_ty.is_vector(), "expected vector type");
        self.match_token(
            Token::Multiply,
            "expected '*' followed by a dynamic scale value",
        )?;
        let dyn_scale = self.match_gv("expected dynamic scale global value")?;
        let data = DynamicTypeData::new(vector_base_ty, dyn_scale);
        // Collect any trailing comments.
        self.token();
        self.claim_gathered_comments(dt);
        Ok((dt, data))
    }

    // Parse a global value decl.
    //
    // global-val-decl ::= * GlobalValue(gv) [ "!" fact ] "=" global-val-desc
    // global-val-desc ::= "vmctx"
    //                   | "load" "." type "notrap" "aligned" GlobalValue(base) [offset]
    //                   | "iadd_imm" "(" GlobalValue(base) ")" imm64
    //                   | "symbol" ["colocated"] name + imm64
    //                   | "dyn_scale_target_const" "." type
    //
    fn parse_global_value_decl(
        &mut self,
    ) -> ParseResult<(GlobalValue, GlobalValueData, Option<Fact>)> {
        let gv = self.match_gv("expected global value number: gv«n»")?;

        let fact = if self.token() == Some(Token::Bang) {
            self.consume();
            Some(self.parse_fact()?)
        } else {
            None
        };

        self.match_token(Token::Equal, "expected '=' in global value declaration")?;

        let data = match self.match_any_identifier("expected global value kind")? {
            "vmctx" => GlobalValueData::VMContext,
            "load" => {
                self.match_token(
                    Token::Dot,
                    "expected '.' followed by type in load global value decl",
                )?;
                let global_type = self.match_type("expected load type")?;
                let flags = self.optional_memflags()?;
                let base = self.match_gv("expected global value: gv«n»")?;
                let offset = self.optional_offset32()?;

                if !(flags.notrap() && flags.aligned()) {
                    return err!(self.loc, "global-value load must be notrap and aligned");
                }
                GlobalValueData::Load {
                    base,
                    offset,
                    global_type,
                    flags,
                }
            }
            "iadd_imm" => {
                self.match_token(
                    Token::Dot,
                    "expected '.' followed by type in iadd_imm global value decl",
                )?;
                let global_type = self.match_type("expected iadd type")?;
                let base = self.match_gv("expected global value: gv«n»")?;
                self.match_token(
                    Token::Comma,
                    "expected ',' followed by rhs in iadd_imm global value decl",
                )?;
                let offset = self.match_imm64("expected iadd_imm immediate")?;
                GlobalValueData::IAddImm {
                    base,
                    offset,
                    global_type,
                }
            }
            "symbol" => {
                let colocated = self.optional(Token::Identifier("colocated"));
                let tls = self.optional(Token::Identifier("tls"));
                let name = self.parse_external_name()?;
                let offset = self.optional_offset_imm64()?;
                GlobalValueData::Symbol {
                    name,
                    offset,
                    colocated,
                    tls,
                }
            }
            "dyn_scale_target_const" => {
                self.match_token(
                    Token::Dot,
                    "expected '.' followed by type in dynamic scale global value decl",
                )?;
                let vector_type = self.match_type("expected load type")?;
                assert!(vector_type.is_vector(), "Expected vector type");
                GlobalValueData::DynScaleTargetConst { vector_type }
            }
            other => return err!(self.loc, "Unknown global value kind '{}'", other),
        };

        // Collect any trailing comments.
        self.token();
        self.claim_gathered_comments(gv);

        Ok((gv, data, fact))
    }

    // Parse one field definition in a memory-type struct decl.
    //
    // memory-type-field ::=  offset ":" type ["readonly"] [ "!" fact ]
    // offset ::= uimm64
    fn parse_memory_type_field(&mut self) -> ParseResult<MemoryTypeField> {
        let offset: u64 = self
            .match_uimm64(
                "expected u64 constant value for field offset in struct memory-type declaration",
            )?
            .into();
        self.match_token(
            Token::Colon,
            "expected colon after field offset in struct memory-type declaration",
        )?;
        let ty = self.match_type("expected type for field in struct memory-type declaration")?;
        let readonly = if self.token() == Some(Token::Identifier("readonly")) {
            self.consume();
            true
        } else {
            false
        };
        let fact = if self.token() == Some(Token::Bang) {
            self.consume();
            let fact = self.parse_fact()?;
            Some(fact)
        } else {
            None
        };
        Ok(MemoryTypeField {
            offset,
            ty,
            readonly,
            fact,
        })
    }

    // Parse a memory-type decl.
    //
    // memory-type-decl ::= MemoryType(mt) "=" memory-type-desc
    // memory-type-desc ::= "struct" size "{" memory-type-field,* "}"
    //                    | "memory" size
    //                    | "dynamic_memory" GlobalValue "+" offset
    //                    | "empty"
    // size ::= uimm64
    // offset ::= uimm64
    fn parse_memory_type_decl(&mut self) -> ParseResult<(MemoryType, MemoryTypeData)> {
        let mt = self.match_mt("expected memory type number: mt«n»")?;
        self.match_token(Token::Equal, "expected '=' in memory type declaration")?;

        let data = match self.token() {
            Some(Token::Identifier("struct")) => {
                self.consume();
                let size: u64 = self.match_uimm64("expected u64 constant value for struct size in struct memory-type declaration")?.into();
                self.match_token(Token::LBrace, "expected opening brace to start struct fields in struct memory-type declaration")?;
                let mut fields = vec![];
                while self.token() != Some(Token::RBrace) {
                    let field = self.parse_memory_type_field()?;
                    fields.push(field);
                    if self.token() == Some(Token::Comma) {
                        self.consume();
                    } else {
                        break;
                    }
                }
                self.match_token(
                    Token::RBrace,
                    "expected closing brace after struct fields in struct memory-type declaration",
                )?;
                MemoryTypeData::Struct { size, fields }
            }
            Some(Token::Identifier("memory")) => {
                self.consume();
                let size: u64 = self.match_uimm64("expected u64 constant value for size in static-memory memory-type declaration")?.into();
                MemoryTypeData::Memory { size }
            }
            Some(Token::Identifier("dynamic_memory")) => {
                self.consume();
                let gv = self.match_gv(
                    "expected a global value for `dynamic_memory` memory-type declaration",
                )?;
                self.match_token(
                    Token::Plus,
                    "expected `+` after global value in `dynamic_memory` memory-type declaration",
                )?;
                let size: u64 = self.match_uimm64("expected u64 constant value for size offset in `dynamic_memory` memory-type declaration")?.into();
                MemoryTypeData::DynamicMemory { gv, size }
            }
            Some(Token::Identifier("empty")) => {
                self.consume();
                MemoryTypeData::Empty
            }
            other => {
                return err!(
                    self.loc,
                    "Unknown memory type declaration kind '{:?}'",
                    other
                )
            }
        };

        // Collect any trailing comments.
        self.token();
        self.claim_gathered_comments(mt);

        Ok((mt, data))
    }

    // Parse a signature decl.
    //
    // signature-decl ::= SigRef(sigref) "=" signature
    //
    fn parse_signature_decl(&mut self) -> ParseResult<(SigRef, Signature)> {
        let sig = self.match_sig("expected signature number: sig«n»")?;
        self.match_token(Token::Equal, "expected '=' in signature decl")?;
        let data = self.parse_signature()?;

        // Collect any trailing comments.
        self.token();
        self.claim_gathered_comments(sig);

        Ok((sig, data))
    }

    // Parse a function decl.
    //
    // Two variants:
    //
    // function-decl ::= FuncRef(fnref) "=" ["colocated"]" name function-decl-sig
    // function-decl-sig ::= SigRef(sig) | signature
    //
    // The first variant allocates a new signature reference. The second references an existing
    // signature which must be declared first.
    //
    fn parse_function_decl(&mut self, ctx: &mut Context) -> ParseResult<(FuncRef, ExtFuncData)> {
        let fn_ = self.match_fn("expected function number: fn«n»")?;
        self.match_token(Token::Equal, "expected '=' in function decl")?;

        let loc = self.loc;

        // function-decl ::= FuncRef(fnref) "=" * ["colocated"] name function-decl-sig
        let colocated = self.optional(Token::Identifier("colocated"));

        // function-decl ::= FuncRef(fnref) "=" ["colocated"] * name function-decl-sig
        let name = self.parse_external_name()?;

        // function-decl ::= FuncRef(fnref) "=" ["colocated"] name * function-decl-sig
        let data = match self.token() {
            Some(Token::LPar) => {
                // function-decl ::= FuncRef(fnref) "=" ["colocated"] name * signature
                let sig = self.parse_signature()?;
                let sigref = ctx.function.import_signature(sig);
                ctx.map
                    .def_entity(sigref.into(), loc)
                    .expect("duplicate SigRef entities created");
                ExtFuncData {
                    name,
                    signature: sigref,
                    colocated,
                }
            }
            Some(Token::SigRef(sig_src)) => {
                let sig = match SigRef::with_number(sig_src) {
                    None => {
                        return err!(self.loc, "attempted to use invalid signature ss{}", sig_src);
                    }
                    Some(sig) => sig,
                };
                ctx.check_sig(sig, self.loc)?;
                self.consume();
                ExtFuncData {
                    name,
                    signature: sig,
                    colocated,
                }
            }
            _ => return err!(self.loc, "expected 'function' or sig«n» in function decl"),
        };

        // Collect any trailing comments.
        self.token();
        self.claim_gathered_comments(fn_);

        Ok((fn_, data))
    }

    // Parse a jump table literal.
    //
    // jump-table-lit ::= "[" block(args) {"," block(args) } "]"
    //                  | "[]"
    fn parse_jump_table(
        &mut self,
        ctx: &mut Context,
        def: ir::BlockCall,
    ) -> ParseResult<ir::JumpTable> {
        self.match_token(Token::LBracket, "expected '[' before jump table contents")?;

        let mut data = Vec::new();

        match self.token() {
            Some(Token::Block(dest)) => {
                self.consume();
                let args = self.parse_opt_value_list()?;
                data.push(ctx.function.dfg.block_call(dest, &args));

                loop {
                    match self.token() {
                        Some(Token::Comma) => {
                            self.consume();
                            if let Some(Token::Block(dest)) = self.token() {
                                self.consume();
                                let args = self.parse_opt_value_list()?;
                                data.push(ctx.function.dfg.block_call(dest, &args));
                            } else {
                                return err!(self.loc, "expected jump_table entry");
                            }
                        }
                        Some(Token::RBracket) => break,
                        _ => return err!(self.loc, "expected ']' after jump table contents"),
                    }
                }
            }
            Some(Token::RBracket) => (),
            _ => return err!(self.loc, "expected jump_table entry"),
        }

        self.consume();

        Ok(ctx
            .function
            .dfg
            .jump_tables
            .push(JumpTableData::new(def, &data)))
    }

    // Parse a constant decl.
    //
    // constant-decl ::= * Constant(c) "=" ty? "[" literal {"," literal} "]"
    fn parse_constant_decl(&mut self) -> ParseResult<(Constant, ConstantData)> {
        let name = self.match_constant()?;
        self.match_token(Token::Equal, "expected '=' in constant decl")?;
        let data = if let Some(Token::Type(_)) = self.token() {
            let ty = self.match_type("expected type of constant")?;
            self.match_uimm128(ty)
        } else {
            self.match_hexadecimal_constant("expected an immediate hexadecimal operand")
        }?;

        // Collect any trailing comments.
        self.token();
        self.claim_gathered_comments(name);

        Ok((name, data))
    }

    // Parse a stack limit decl
    //
    // stack-limit-decl ::= * StackLimit "=" GlobalValue(gv)
    fn parse_stack_limit_decl(&mut self) -> ParseResult<GlobalValue> {
        self.match_stack_limit()?;
        self.match_token(Token::Equal, "expected '=' in stack limit decl")?;
        let limit = match self.token() {
            Some(Token::GlobalValue(base_num)) => match GlobalValue::with_number(base_num) {
                Some(gv) => gv,
                None => return err!(self.loc, "invalid global value number for stack limit"),
            },
            _ => return err!(self.loc, "expected global value"),
        };
        self.consume();

        // Collect any trailing comments.
        self.token();
        self.claim_gathered_comments(AnyEntity::StackLimit);

        Ok(limit)
    }

    // Parse a function body, add contents to `ctx`.
    //
    // function-body ::= * { extended-basic-block }
    //
    fn parse_function_body(&mut self, ctx: &mut Context) -> ParseResult<()> {
        while self.token() != Some(Token::RBrace) {
            self.parse_basic_block(ctx)?;
        }

        // Now that we've seen all defined values in the function, ensure that
        // all references refer to a definition.
        for block in &ctx.function.layout {
            for inst in ctx.function.layout.block_insts(block) {
                for value in ctx.function.dfg.inst_values(inst) {
                    if !ctx.map.contains_value(value) {
                        return err!(
                            ctx.map.location(AnyEntity::Inst(inst)).unwrap(),
                            "undefined operand value {}",
                            value
                        );
                    }
                }
            }
        }

        for alias in &ctx.aliases {
            if !ctx.function.dfg.set_alias_type_for_parser(*alias) {
                let loc = ctx.map.location(AnyEntity::Value(*alias)).unwrap();
                return err!(loc, "alias cycle involving {}", alias);
            }
        }

        Ok(())
    }

    // Parse a basic block, add contents to `ctx`.
    //
    // extended-basic-block ::= * block-header { instruction }
    // block-header         ::= Block(block) [block-params] [block-flags] ":"
    // block-flags          ::= [Cold]
    //
    fn parse_basic_block(&mut self, ctx: &mut Context) -> ParseResult<()> {
        // Collect comments for the next block.
        self.start_gathering_comments();

        let block_num = self.match_block("expected block header")?;
        let block = ctx.add_block(block_num, self.loc)?;

        if block_num.as_u32() >= MAX_BLOCKS_IN_A_FUNCTION {
            return Err(self.error("too many blocks"));
        }

        if self.token() == Some(Token::LPar) {
            self.parse_block_params(ctx, block)?;
        }

        if self.optional(Token::Cold) {
            ctx.set_cold_block(block);
        }

        self.match_token(Token::Colon, "expected ':' after block parameters")?;

        // Collect any trailing comments.
        self.token();
        self.claim_gathered_comments(block);

        // extended-basic-block ::= block-header * { instruction }
        while match self.token() {
            Some(Token::Value(_))
            | Some(Token::Identifier(_))
            | Some(Token::LBracket)
            | Some(Token::SourceLoc(_)) => true,
            _ => false,
        } {
            let srcloc = self.optional_srcloc()?;

            // We need to parse instruction results here because they are shared
            // between the parsing of value aliases and the parsing of instructions.
            //
            // inst-results ::= Value(v) { "," Value(v) }
            let results = self.parse_inst_results(ctx)?;

            for result in &results {
                while ctx.function.dfg.num_values() <= result.index() {
                    ctx.function.dfg.make_invalid_value_for_parser();
                }
            }

            match self.token() {
                Some(Token::Arrow) => {
                    self.consume();
                    self.parse_value_alias(&results, ctx)?;
                }
                Some(Token::Equal) => {
                    self.consume();
                    self.parse_instruction(&results, srcloc, ctx, block)?;
                }
                _ if !results.is_empty() => return err!(self.loc, "expected -> or ="),
                _ => self.parse_instruction(&results, srcloc, ctx, block)?,
            }
        }

        Ok(())
    }

    // Parse parenthesized list of block parameters. Returns a vector of (u32, Type) pairs with the
    // value numbers of the defined values and the defined types.
    //
    // block-params ::= * "(" block-param { "," block-param } ")"
    fn parse_block_params(&mut self, ctx: &mut Context, block: Block) -> ParseResult<()> {
        // block-params ::= * "(" block-param { "," block-param } ")"
        self.match_token(Token::LPar, "expected '(' before block parameters")?;

        // block-params ::= "(" * block-param { "," block-param } ")"
        self.parse_block_param(ctx, block)?;

        // block-params ::= "(" block-param * { "," block-param } ")"
        while self.optional(Token::Comma) {
            // block-params ::= "(" block-param { "," * block-param } ")"
            self.parse_block_param(ctx, block)?;
        }

        // block-params ::= "(" block-param { "," block-param } * ")"
        self.match_token(Token::RPar, "expected ')' after block parameters")?;

        Ok(())
    }

    // Parse a single block parameter declaration, and append it to `block`.
    //
    // block-param ::= * Value(v) [ "!" fact ]  ":" Type(t) arg-loc?
    // arg-loc ::= "[" value-location "]"
    //
    fn parse_block_param(&mut self, ctx: &mut Context, block: Block) -> ParseResult<()> {
        // block-param ::= * Value(v) [ "!" fact ] ":" Type(t) arg-loc?
        let v = self.match_value("block argument must be a value")?;
        let v_location = self.loc;
        // block-param ::= Value(v) * [ "!" fact ]  ":" Type(t) arg-loc?
        let fact = if self.token() == Some(Token::Bang) {
            self.consume();
            // block-param ::= Value(v) [ "!" * fact ]  ":" Type(t) arg-loc?
            Some(self.parse_fact()?)
        } else {
            None
        };
        self.match_token(Token::Colon, "expected ':' after block argument")?;
        // block-param ::= Value(v) [ "!" fact ] ":" * Type(t) arg-loc?

        while ctx.function.dfg.num_values() <= v.index() {
            ctx.function.dfg.make_invalid_value_for_parser();
        }

        let t = self.match_type("expected block argument type")?;
        // Allocate the block argument.
        ctx.function.dfg.append_block_param_for_parser(block, t, v);
        ctx.map.def_value(v, v_location)?;
        ctx.function.dfg.facts[v] = fact;

        Ok(())
    }

    // Parse a "fact" for proof-carrying code, attached to a value.
    //
    // fact ::= "range" "(" bit-width "," min-value "," max-value ")"
    //        | "dynamic_range" "(" bit-width "," expr "," expr ")"
    //        | "mem" "(" memory-type "," mt-offset "," mt-offset [ "," "nullable" ] ")"
    //        | "dynamic_mem" "(" memory-type "," expr "," expr [ "," "nullable" ] ")"
    //        | "conflict"
    // bit-width ::= uimm64
    // min-value ::= uimm64
    // max-value ::= uimm64
    // valid-range ::= uimm64
    // mt-offset ::= uimm64
    fn parse_fact(&mut self) -> ParseResult<Fact> {
        match self.token() {
            Some(Token::Identifier("range")) => {
                self.consume();
                self.match_token(Token::LPar, "`range` fact needs an opening `(`")?;
                let bit_width: u64 = self
                    .match_uimm64("expected a bit-width value for `range` fact")?
                    .into();
                self.match_token(Token::Comma, "expected a comma")?;
                let min: u64 = self
                    .match_uimm64("expected a min value for `range` fact")?
                    .into();
                self.match_token(Token::Comma, "expected a comma")?;
                let max: u64 = self
                    .match_uimm64("expected a max value for `range` fact")?
                    .into();
                self.match_token(Token::RPar, "`range` fact needs a closing `)`")?;
                let bit_width_max = match bit_width {
                    x if x > 64 => {
                        return Err(self.error("bitwidth must be <= 64 bits on a `range` fact"));
                    }
                    64 => u64::MAX,
                    x => (1u64 << x) - 1,
                };
                if min > max {
                    return Err(self.error(
                        "min value must be less than or equal to max value on a `range` fact",
                    ));
                }
                if max > bit_width_max {
                    return Err(
                        self.error("max value is out of range for bitwidth on a `range` fact")
                    );
                }
                Ok(Fact::Range {
                    bit_width: u16::try_from(bit_width).unwrap(),
                    min: min.into(),
                    max: max.into(),
                })
            }
            Some(Token::Identifier("dynamic_range")) => {
                self.consume();
                self.match_token(Token::LPar, "`dynamic_range` fact needs an opening `(`")?;
                let bit_width: u64 = self
                    .match_uimm64("expected a bit-width value for `dynamic_range` fact")?
                    .into();
                self.match_token(Token::Comma, "expected a comma")?;
                let min = self.parse_expr()?;
                self.match_token(Token::Comma, "expected a comma")?;
                let max = self.parse_expr()?;
                self.match_token(Token::RPar, "`dynamic_range` fact needs a closing `)`")?;
                Ok(Fact::DynamicRange {
                    bit_width: u16::try_from(bit_width).unwrap(),
                    min,
                    max,
                })
            }
            Some(Token::Identifier("mem")) => {
                self.consume();
                self.match_token(Token::LPar, "expected a `(`")?;
                let ty = self.match_mt("expected a memory type for `mem` fact")?;
                self.match_token(
                    Token::Comma,
                    "expected a comma after memory type in `mem` fact",
                )?;
                let min_offset: u64 = self
                    .match_uimm64("expected a uimm64 minimum pointer offset for `mem` fact")?
                    .into();
                self.match_token(Token::Comma, "expected a comma after offset in `mem` fact")?;
                let max_offset: u64 = self
                    .match_uimm64("expected a uimm64 maximum pointer offset for `mem` fact")?
                    .into();
                let nullable = if self.token() == Some(Token::Comma) {
                    self.consume();
                    self.match_token(
                        Token::Identifier("nullable"),
                        "expected `nullable` in last optional field of `dynamic_mem`",
                    )?;
                    true
                } else {
                    false
                };
                self.match_token(Token::RPar, "expected a `)`")?;
                Ok(Fact::Mem {
                    ty,
                    min_offset,
                    max_offset,
                    nullable,
                })
            }
            Some(Token::Identifier("dynamic_mem")) => {
                self.consume();
                self.match_token(Token::LPar, "expected a `(`")?;
                let ty = self.match_mt("expected a memory type for `dynamic_mem` fact")?;
                self.match_token(
                    Token::Comma,
                    "expected a comma after memory type in `dynamic_mem` fact",
                )?;
                let min = self.parse_expr()?;
                self.match_token(
                    Token::Comma,
                    "expected a comma after offset in `dynamic_mem` fact",
                )?;
                let max = self.parse_expr()?;
                let nullable = if self.token() == Some(Token::Comma) {
                    self.consume();
                    self.match_token(
                        Token::Identifier("nullable"),
                        "expected `nullable` in last optional field of `dynamic_mem`",
                    )?;
                    true
                } else {
                    false
                };
                self.match_token(Token::RPar, "expected a `)`")?;
                Ok(Fact::DynamicMem {
                    ty,
                    min,
                    max,
                    nullable,
                })
            }
            Some(Token::Identifier("def")) => {
                self.consume();
                self.match_token(Token::LPar, "expected a `(`")?;
                let value = self.match_value("expected a value number in `def` fact")?;
                self.match_token(Token::RPar, "expected a `)`")?;
                Ok(Fact::Def { value })
            }
            Some(Token::Identifier("compare")) => {
                self.consume();
                self.match_token(Token::LPar, "expected a `(`")?;
                let kind = self.match_enum("expected intcc condition code in `compare` fact")?;
                self.match_token(
                    Token::Comma,
                    "expected comma in `compare` fact after condition code",
                )?;
                let lhs = self.parse_expr()?;
                self.match_token(Token::Comma, "expected comma in `compare` fact after LHS")?;
                let rhs = self.parse_expr()?;
                self.match_token(Token::RPar, "expected a `)`")?;
                Ok(Fact::Compare { kind, lhs, rhs })
            }
            Some(Token::Identifier("conflict")) => {
                self.consume();
                Ok(Fact::Conflict)
            }
            _ => Err(self.error(
                "expected a `range`, 'dynamic_range', `mem`, `dynamic_mem`, `def`, `compare` or `conflict` fact",
            )),
        }
    }

    // Parse a dynamic expression used in some kinds of PCC facts.
    //
    // expr ::= base-expr
    //        | base-expr + uimm64  // but in-range for imm64
    //        | base-expr - uimm64  // but in-range for imm64
    //        | imm64
    fn parse_expr(&mut self) -> ParseResult<Expr> {
        if let Some(Token::Integer(_)) = self.token() {
            let offset: i64 = self
                .match_imm64("expected imm64 for dynamic expression")?
                .into();
            Ok(Expr {
                base: BaseExpr::None,
                offset,
            })
        } else {
            let base = self.parse_base_expr()?;
            match self.token() {
                Some(Token::Plus) => {
                    self.consume();
                    let offset: u64 = self
                        .match_uimm64(
                            "expected uimm64 in imm64 range for offset in dynamic expression",
                        )?
                        .into();
                    let offset: i64 = i64::try_from(offset).map_err(|_| {
                        self.error("integer offset in dynamic expression is out of range")
                    })?;
                    Ok(Expr { base, offset })
                }
                Some(Token::Integer(x)) if x.starts_with("-") => {
                    let offset: i64 = self
                        .match_imm64("expected an imm64 range for offset in dynamic expression")?
                        .into();
                    Ok(Expr { base, offset })
                }
                _ => Ok(Expr { base, offset: 0 }),
            }
        }
    }

    // Parse the base part of a dynamic expression, used in some PCC facts.
    //
    // base-expr ::= GlobalValue(base)
    //             | Value(base)
    //             | "max"
    //             | (epsilon)
    fn parse_base_expr(&mut self) -> ParseResult<BaseExpr> {
        match self.token() {
            Some(Token::Identifier("max")) => {
                self.consume();
                Ok(BaseExpr::Max)
            }
            Some(Token::GlobalValue(..)) => {
                let gv = self.match_gv("expected global value")?;
                Ok(BaseExpr::GlobalValue(gv))
            }
            Some(Token::Value(..)) => {
                let value = self.match_value("expected value")?;
                Ok(BaseExpr::Value(value))
            }
            _ => Ok(BaseExpr::None),
        }
    }

    // Parse instruction results and return them.
    //
    // inst-results ::= Value(v) { "," Value(v) }
    //
    fn parse_inst_results(&mut self, ctx: &mut Context) -> ParseResult<SmallVec<[Value; 1]>> {
        // Result value numbers.
        let mut results = SmallVec::new();

        // instruction  ::=  * [inst-results "="] Opcode(opc) ["." Type] ...
        // inst-results ::= * Value(v) { "," Value(v) }
        if let Some(Token::Value(v)) = self.token() {
            self.consume();

            results.push(v);

            let fact = if self.token() == Some(Token::Bang) {
                self.consume();
                // block-param ::= Value(v) [ "!" * fact ]  ":" Type(t) arg-loc?
                Some(self.parse_fact()?)
            } else {
                None
            };
            ctx.function.dfg.facts[v] = fact;

            // inst-results ::= Value(v) * { "," Value(v) }
            while self.optional(Token::Comma) {
                // inst-results ::= Value(v) { "," * Value(v) }
                let v = self.match_value("expected result value")?;
                results.push(v);

                let fact = if self.token() == Some(Token::Bang) {
                    self.consume();
                    // block-param ::= Value(v) [ "!" * fact ]  ":" Type(t) arg-loc?
                    Some(self.parse_fact()?)
                } else {
                    None
                };
                ctx.function.dfg.facts[v] = fact;
            }
        }

        Ok(results)
    }

    // Parse a value alias, and append it to `block`.
    //
    // value_alias ::= [inst-results] "->" Value(v)
    //
    fn parse_value_alias(&mut self, results: &[Value], ctx: &mut Context) -> ParseResult<()> {
        if results.len() != 1 {
            return err!(self.loc, "wrong number of aliases");
        }
        let result = results[0];
        let dest = self.match_value("expected value alias")?;

        // Allow duplicate definitions of aliases, as long as they are identical.
        if ctx.map.contains_value(result) {
            if let Some(old) = ctx.function.dfg.value_alias_dest_for_serialization(result) {
                if old != dest {
                    return err!(
                        self.loc,
                        "value {} is already defined as an alias with destination {}",
                        result,
                        old
                    );
                }
            } else {
                return err!(self.loc, "value {} is already defined");
            }
        } else {
            ctx.map.def_value(result, self.loc)?;
        }

        if !ctx.map.contains_value(dest) {
            return err!(self.loc, "value {} is not yet defined", dest);
        }

        ctx.function
            .dfg
            .make_value_alias_for_serialization(dest, result);

        ctx.aliases.push(result);
        Ok(())
    }

    // Parse an instruction, append it to `block`.
    //
    // instruction ::= [inst-results "="] Opcode(opc) ["." Type] ...
    //
    fn parse_instruction(
        &mut self,
        results: &[Value],
        srcloc: ir::SourceLoc,
        ctx: &mut Context,
        block: Block,
    ) -> ParseResult<()> {
        // Define the result values.
        for val in results {
            ctx.map.def_value(*val, self.loc)?;
        }

        // Collect comments for the next instruction.
        self.start_gathering_comments();

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
            if let Some(Token::Type(_t)) = self.token() {
                Some(self.match_type("expected type after 'opcode.'")?)
            } else {
                let dt = self.match_dt("expected dynamic type")?;
                self.concrete_from_dt(dt, ctx)
            }
        } else {
            None
        };

        // instruction ::=  [inst-results "="] Opcode(opc) ["." Type] * ...
        let inst_data = self.parse_inst_operands(ctx, opcode, explicit_ctrl_type)?;

        // We're done parsing the instruction now.
        //
        // We still need to check that the number of result values in the source matches the opcode
        // or function call signature. We also need to create values with the right type for all
        // the instruction results.
        let ctrl_typevar = self.infer_typevar(ctx, opcode, explicit_ctrl_type, &inst_data)?;
        let inst = ctx.function.dfg.make_inst(inst_data);
        let num_results =
            ctx.function
                .dfg
                .make_inst_results_for_parser(inst, ctrl_typevar, results);
        ctx.function.layout.append_inst(inst, block);
        ctx.map
            .def_entity(inst.into(), opcode_loc)
            .expect("duplicate inst references created");

        if !srcloc.is_default() {
            ctx.function.set_srcloc(inst, srcloc);
        }

        if results.len() != num_results {
            return err!(
                self.loc,
                "instruction produces {} result values, {} given",
                num_results,
                results.len()
            );
        }

        // Collect any trailing comments.
        self.token();
        self.claim_gathered_comments(inst);

        Ok(())
    }

    // Type inference for polymorphic instructions.
    //
    // The controlling type variable can be specified explicitly as 'splat.i32x4 v5', or it can be
    // inferred from `inst_data.typevar_operand` for some opcodes.
    //
    // Returns the controlling typevar for a polymorphic opcode, or `INVALID` for a non-polymorphic
    // opcode.
    fn infer_typevar(
        &self,
        ctx: &Context,
        opcode: Opcode,
        explicit_ctrl_type: Option<Type>,
        inst_data: &InstructionData,
    ) -> ParseResult<Type> {
        let constraints = opcode.constraints();
        let ctrl_type = match explicit_ctrl_type {
            Some(t) => t,
            None => {
                if constraints.use_typevar_operand() {
                    // This is an opcode that supports type inference, AND there was no
                    // explicit type specified. Look up `ctrl_value` to see if it was defined
                    // already.
                    // TBD: If it is defined in another block, the type should have been
                    // specified explicitly. It is unfortunate that the correctness of IR
                    // depends on the layout of the blocks.
                    let ctrl_src_value = inst_data
                        .typevar_operand(&ctx.function.dfg.value_lists)
                        .expect("Constraints <-> Format inconsistency");
                    if !ctx.map.contains_value(ctrl_src_value) {
                        return err!(
                            self.loc,
                            "type variable required for polymorphic opcode, e.g. '{}.{}'; \
                             can't infer from {} which is not yet defined",
                            opcode,
                            constraints.ctrl_typeset().unwrap().example(),
                            ctrl_src_value
                        );
                    }
                    if !ctx.function.dfg.value_is_valid_for_parser(ctrl_src_value) {
                        return err!(
                            self.loc,
                            "type variable required for polymorphic opcode, e.g. '{}.{}'; \
                             can't infer from {} which is not yet resolved",
                            opcode,
                            constraints.ctrl_typeset().unwrap().example(),
                            ctrl_src_value
                        );
                    }
                    ctx.function.dfg.value_type(ctrl_src_value)
                } else if constraints.is_polymorphic() {
                    // This opcode does not support type inference, so the explicit type
                    // variable is required.
                    return err!(
                        self.loc,
                        "type variable required for polymorphic opcode, e.g. '{}.{}'",
                        opcode,
                        constraints.ctrl_typeset().unwrap().example()
                    );
                } else {
                    // This is a non-polymorphic opcode. No typevar needed.
                    INVALID
                }
            }
        };

        // Verify that `ctrl_type` is valid for the controlling type variable. We don't want to
        // attempt deriving types from an incorrect basis.
        // This is not a complete type check. The verifier does that.
        if let Some(typeset) = constraints.ctrl_typeset() {
            // This is a polymorphic opcode.
            if !typeset.contains(ctrl_type) {
                return err!(
                    self.loc,
                    "{} is not a valid typevar for {}",
                    ctrl_type,
                    opcode
                );
            }
        // Treat it as a syntax error to specify a typevar on a non-polymorphic opcode.
        } else if ctrl_type != INVALID {
            return err!(self.loc, "{} does not take a typevar", opcode);
        }

        Ok(ctrl_type)
    }

    // Parse comma-separated value list into a VariableArgs struct.
    //
    // value_list ::= [ value { "," value } ]
    //
    fn parse_value_list(&mut self) -> ParseResult<VariableArgs> {
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

    // Parse an optional value list enclosed in parentheses.
    fn parse_opt_value_list(&mut self) -> ParseResult<VariableArgs> {
        if !self.optional(Token::LPar) {
            return Ok(VariableArgs::new());
        }

        let args = self.parse_value_list()?;

        self.match_token(Token::RPar, "expected ')' after arguments")?;

        Ok(args)
    }

    /// Parse a CLIF run command.
    ///
    /// run-command ::= "run" [":" invocation comparison expected]
    ///               \ "print" [":" invocation]
    fn parse_run_command(&mut self, sig: &Signature) -> ParseResult<RunCommand> {
        // skip semicolon
        match self.token() {
            Some(Token::Identifier("run")) => {
                self.consume();
                if self.optional(Token::Colon) {
                    let invocation = self.parse_run_invocation(sig)?;
                    let comparison = self.parse_run_comparison()?;
                    let expected = self.parse_run_returns(sig)?;
                    Ok(RunCommand::Run(invocation, comparison, expected))
                } else if sig.params.is_empty()
                    && sig.returns.len() == 1
                    && sig.returns[0].value_type.is_int()
                {
                    // To match the existing run behavior that does not require an explicit
                    // invocation, we create an invocation from a function like `() -> i*` and
                    // require the result to be non-zero.
                    let invocation = Invocation::new("default", vec![]);
                    let expected = vec![DataValue::I8(0)];
                    let comparison = Comparison::NotEquals;
                    Ok(RunCommand::Run(invocation, comparison, expected))
                } else {
                    Err(self.error("unable to parse the run command"))
                }
            }
            Some(Token::Identifier("print")) => {
                self.consume();
                if self.optional(Token::Colon) {
                    Ok(RunCommand::Print(self.parse_run_invocation(sig)?))
                } else if sig.params.is_empty() {
                    // To allow printing of functions like `() -> *`, we create a no-arg invocation.
                    let invocation = Invocation::new("default", vec![]);
                    Ok(RunCommand::Print(invocation))
                } else {
                    Err(self.error("unable to parse the print command"))
                }
            }
            _ => Err(self.error("expected a 'run:' or 'print:' command")),
        }
    }

    /// Parse the invocation of a CLIF function.
    ///
    /// This is different from parsing a CLIF `call`; it is used in parsing run commands like
    /// `run: %fn(42, 4.2) == false`.
    ///
    /// invocation ::= name "(" [data-value-list] ")"
    fn parse_run_invocation(&mut self, sig: &Signature) -> ParseResult<Invocation> {
        if let Some(Token::Name(name)) = self.token() {
            self.consume();
            self.match_token(
                Token::LPar,
                "expected invocation parentheses, e.g. %fn(...)",
            )?;

            let arg_types = sig
                .params
                .iter()
                .map(|abi| abi.value_type)
                .collect::<Vec<_>>();
            let args = self.parse_data_value_list(&arg_types)?;

            self.match_token(
                Token::RPar,
                "expected invocation parentheses, e.g. %fn(...)",
            )?;
            Ok(Invocation::new(name, args))
        } else {
            Err(self.error("expected a function name, e.g. %my_fn"))
        }
    }

    /// Parse a comparison operator for run commands.
    ///
    /// comparison ::= "==" | "!="
    fn parse_run_comparison(&mut self) -> ParseResult<Comparison> {
        if self.optional(Token::Equal) {
            self.match_token(Token::Equal, "expected another =")?;
            Ok(Comparison::Equals)
        } else if self.optional(Token::Bang) {
            self.match_token(Token::Equal, "expected a =")?;
            Ok(Comparison::NotEquals)
        } else {
            Err(self.error("unable to parse a valid comparison operator"))
        }
    }

    /// Parse the expected return values of a run invocation.
    ///
    /// expected ::= "[" "]"
    ///            | data-value
    ///            | "[" data-value-list "]"
    fn parse_run_returns(&mut self, sig: &Signature) -> ParseResult<Vec<DataValue>> {
        if sig.returns.len() != 1 {
            self.match_token(Token::LBracket, "expected a left bracket [")?;
        }

        let returns = self
            .parse_data_value_list(&sig.returns.iter().map(|a| a.value_type).collect::<Vec<_>>())?;

        if sig.returns.len() != 1 {
            self.match_token(Token::RBracket, "expected a right bracket ]")?;
        }
        Ok(returns)
    }

    /// Parse a comma-separated list of data values.
    ///
    /// data-value-list ::= [data-value {"," data-value-list}]
    fn parse_data_value_list(&mut self, types: &[Type]) -> ParseResult<Vec<DataValue>> {
        let mut values = vec![];
        for ty in types.iter().take(1) {
            values.push(self.parse_data_value(*ty)?);
        }
        for ty in types.iter().skip(1) {
            self.match_token(
                Token::Comma,
                "expected a comma between invocation arguments",
            )?;
            values.push(self.parse_data_value(*ty)?);
        }
        Ok(values)
    }

    /// Parse a data value; e.g. `42`, `4.2`, `true`.
    ///
    /// data-value-list ::= [data-value {"," data-value-list}]
    fn parse_data_value(&mut self, ty: Type) -> ParseResult<DataValue> {
        let dv = match ty {
            I8 => DataValue::from(self.match_imm8("expected a i8")?),
            I16 => DataValue::from(self.match_imm16("expected an i16")?),
            I32 => DataValue::from(self.match_imm32("expected an i32")?),
            I64 => DataValue::from(Into::<i64>::into(self.match_imm64("expected an i64")?)),
            I128 => DataValue::from(self.match_imm128("expected an i128")?),
            F32 => DataValue::from(self.match_ieee32("expected an f32")?),
            F64 => DataValue::from(self.match_ieee64("expected an f64")?),
            _ if (ty.is_vector() || ty.is_dynamic_vector()) => {
                let as_vec = self.match_uimm128(ty)?.into_vec();
                if as_vec.len() == 16 {
                    let mut as_array = [0; 16];
                    as_array.copy_from_slice(&as_vec[..]);
                    DataValue::from(as_array)
                } else if as_vec.len() == 8 {
                    let mut as_array = [0; 8];
                    as_array.copy_from_slice(&as_vec[..]);
                    DataValue::from(as_array)
                } else {
                    return Err(self.error("only 128-bit vectors are currently supported"));
                }
            }
            _ => return Err(self.error(&format!("don't know how to parse data values of: {}", ty))),
        };
        Ok(dv)
    }

    // Parse the operands following the instruction opcode.
    // This depends on the format of the opcode.
    fn parse_inst_operands(
        &mut self,
        ctx: &mut Context,
        opcode: Opcode,
        explicit_control_type: Option<Type>,
    ) -> ParseResult<InstructionData> {
        let idata = match opcode.format() {
            InstructionFormat::Unary => InstructionData::Unary {
                opcode,
                arg: self.match_value("expected SSA value operand")?,
            },
            InstructionFormat::UnaryImm => {
                let msg = |bits| format!("expected immediate {bits}-bit integer operand");
                let unsigned = match explicit_control_type {
                    Some(types::I8) => self.match_imm8(&msg(8))? as u8 as i64,
                    Some(types::I16) => self.match_imm16(&msg(16))? as u16 as i64,
                    Some(types::I32) => self.match_imm32(&msg(32))? as u32 as i64,
                    Some(types::I64) => self.match_imm64(&msg(64))?.bits(),
                    _ => {
                        return err!(
                            self.loc,
                            "expected one of the following type: i8, i16, i32 or i64"
                        )
                    }
                };
                InstructionData::UnaryImm {
                    opcode,
                    imm: Imm64::new(unsigned),
                }
            }
            InstructionFormat::UnaryIeee32 => InstructionData::UnaryIeee32 {
                opcode,
                imm: self.match_ieee32("expected immediate 32-bit float operand")?,
            },
            InstructionFormat::UnaryIeee64 => InstructionData::UnaryIeee64 {
                opcode,
                imm: self.match_ieee64("expected immediate 64-bit float operand")?,
            },
            InstructionFormat::UnaryConst => {
                let constant_handle = if let Some(Token::Constant(_)) = self.token() {
                    // If handed a `const?`, use that.
                    let c = self.match_constant()?;
                    ctx.check_constant(c, self.loc)?;
                    c
                } else if let Some(controlling_type) = explicit_control_type {
                    // If an explicit control type is present, we expect a sized value and insert
                    // it in the constant pool.
                    let uimm128 = self.match_uimm128(controlling_type)?;
                    ctx.function.dfg.constants.insert(uimm128)
                } else {
                    return err!(
                        self.loc,
                        "Expected either a const entity or a typed value, e.g. inst.i32x4 [...]"
                    );
                };
                InstructionData::UnaryConst {
                    opcode,
                    constant_handle,
                }
            }
            InstructionFormat::UnaryGlobalValue => {
                let gv = self.match_gv("expected global value")?;
                ctx.check_gv(gv, self.loc)?;
                InstructionData::UnaryGlobalValue {
                    opcode,
                    global_value: gv,
                }
            }
            InstructionFormat::Binary => {
                let lhs = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let rhs = self.match_value("expected SSA value second operand")?;
                InstructionData::Binary {
                    opcode,
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::BinaryImm8 => {
                let arg = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let imm = self.match_uimm8("expected unsigned 8-bit immediate")?;
                InstructionData::BinaryImm8 { opcode, arg, imm }
            }
            InstructionFormat::BinaryImm64 => {
                let lhs = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let rhs = self.match_imm64("expected immediate integer second operand")?;
                InstructionData::BinaryImm64 {
                    opcode,
                    arg: lhs,
                    imm: rhs,
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
                    opcode,
                    args: [ctrl_arg, true_arg, false_arg],
                }
            }
            InstructionFormat::MultiAry => {
                let args = self.parse_value_list()?;
                InstructionData::MultiAry {
                    opcode,
                    args: args.into_value_list(&[], &mut ctx.function.dfg.value_lists),
                }
            }
            InstructionFormat::NullAry => InstructionData::NullAry { opcode },
            InstructionFormat::Jump => {
                // Parse the destination block number.
                let block_num = self.match_block("expected jump destination block")?;
                let args = self.parse_opt_value_list()?;
                let destination = ctx.function.dfg.block_call(block_num, &args);
                InstructionData::Jump {
                    opcode,
                    destination,
                }
            }
            InstructionFormat::Brif => {
                let arg = self.match_value("expected SSA value control operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let block_then = {
                    let block_num = self.match_block("expected branch then block")?;
                    let args = self.parse_opt_value_list()?;
                    ctx.function.dfg.block_call(block_num, &args)
                };
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let block_else = {
                    let block_num = self.match_block("expected branch else block")?;
                    let args = self.parse_opt_value_list()?;
                    ctx.function.dfg.block_call(block_num, &args)
                };
                InstructionData::Brif {
                    opcode,
                    arg,
                    blocks: [block_then, block_else],
                }
            }
            InstructionFormat::BranchTable => {
                let arg = self.match_value("expected SSA value operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let block_num = self.match_block("expected branch destination block")?;
                let args = self.parse_opt_value_list()?;
                let destination = ctx.function.dfg.block_call(block_num, &args);
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let table = self.parse_jump_table(ctx, destination)?;
                InstructionData::BranchTable { opcode, arg, table }
            }
            InstructionFormat::TernaryImm8 => {
                let lhs = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let rhs = self.match_value("expected SSA value last operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let imm = self.match_uimm8("expected 8-bit immediate")?;
                InstructionData::TernaryImm8 {
                    opcode,
                    imm,
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::Shuffle => {
                let a = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let b = self.match_value("expected SSA value second operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let uimm128 = self.match_uimm128(I8X16)?;
                let imm = ctx.function.dfg.immediates.push(uimm128);
                InstructionData::Shuffle {
                    opcode,
                    imm,
                    args: [a, b],
                }
            }
            InstructionFormat::IntCompare => {
                let cond = self.match_enum("expected intcc condition code")?;
                let lhs = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let rhs = self.match_value("expected SSA value second operand")?;
                InstructionData::IntCompare {
                    opcode,
                    cond,
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::IntCompareImm => {
                let cond = self.match_enum("expected intcc condition code")?;
                let lhs = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let rhs = self.match_imm64("expected immediate second operand")?;
                InstructionData::IntCompareImm {
                    opcode,
                    cond,
                    arg: lhs,
                    imm: rhs,
                }
            }
            InstructionFormat::FloatCompare => {
                let cond = self.match_enum("expected floatcc condition code")?;
                let lhs = self.match_value("expected SSA value first operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let rhs = self.match_value("expected SSA value second operand")?;
                InstructionData::FloatCompare {
                    opcode,
                    cond,
                    args: [lhs, rhs],
                }
            }
            InstructionFormat::Call => {
                let func_ref = self.match_fn("expected function reference")?;
                ctx.check_fn(func_ref, self.loc)?;
                self.match_token(Token::LPar, "expected '(' before arguments")?;
                let args = self.parse_value_list()?;
                self.match_token(Token::RPar, "expected ')' after arguments")?;
                InstructionData::Call {
                    opcode,
                    func_ref,
                    args: args.into_value_list(&[], &mut ctx.function.dfg.value_lists),
                }
            }
            InstructionFormat::CallIndirect => {
                let sig_ref = self.match_sig("expected signature reference")?;
                ctx.check_sig(sig_ref, self.loc)?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let callee = self.match_value("expected SSA value callee operand")?;
                self.match_token(Token::LPar, "expected '(' before arguments")?;
                let args = self.parse_value_list()?;
                self.match_token(Token::RPar, "expected ')' after arguments")?;
                InstructionData::CallIndirect {
                    opcode,
                    sig_ref,
                    args: args.into_value_list(&[callee], &mut ctx.function.dfg.value_lists),
                }
            }
            InstructionFormat::FuncAddr => {
                let func_ref = self.match_fn("expected function reference")?;
                ctx.check_fn(func_ref, self.loc)?;
                InstructionData::FuncAddr { opcode, func_ref }
            }
            InstructionFormat::StackLoad => {
                let ss = self.match_ss("expected stack slot number: ss«n»")?;
                ctx.check_ss(ss, self.loc)?;
                let offset = self.optional_offset32()?;
                InstructionData::StackLoad {
                    opcode,
                    stack_slot: ss,
                    offset,
                }
            }
            InstructionFormat::StackStore => {
                let arg = self.match_value("expected SSA value operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let ss = self.match_ss("expected stack slot number: ss«n»")?;
                ctx.check_ss(ss, self.loc)?;
                let offset = self.optional_offset32()?;
                InstructionData::StackStore {
                    opcode,
                    arg,
                    stack_slot: ss,
                    offset,
                }
            }
            InstructionFormat::DynamicStackLoad => {
                let dss = self.match_dss("expected dynamic stack slot number: dss«n»")?;
                ctx.check_dss(dss, self.loc)?;
                InstructionData::DynamicStackLoad {
                    opcode,
                    dynamic_stack_slot: dss,
                }
            }
            InstructionFormat::DynamicStackStore => {
                let arg = self.match_value("expected SSA value operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let dss = self.match_dss("expected dynamic stack slot number: dss«n»")?;
                ctx.check_dss(dss, self.loc)?;
                InstructionData::DynamicStackStore {
                    opcode,
                    arg,
                    dynamic_stack_slot: dss,
                }
            }
            InstructionFormat::Load => {
                let flags = self.optional_memflags()?;
                let addr = self.match_value("expected SSA value address")?;
                let offset = self.optional_offset32()?;
                InstructionData::Load {
                    opcode,
                    flags,
                    arg: addr,
                    offset,
                }
            }
            InstructionFormat::Store => {
                let flags = self.optional_memflags()?;
                let arg = self.match_value("expected SSA value operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let addr = self.match_value("expected SSA value address")?;
                let offset = self.optional_offset32()?;
                InstructionData::Store {
                    opcode,
                    flags,
                    args: [arg, addr],
                    offset,
                }
            }
            InstructionFormat::Trap => {
                let code = self.match_enum("expected trap code")?;
                InstructionData::Trap { opcode, code }
            }
            InstructionFormat::CondTrap => {
                let arg = self.match_value("expected SSA value operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let code = self.match_enum("expected trap code")?;
                InstructionData::CondTrap { opcode, arg, code }
            }
            InstructionFormat::AtomicCas => {
                let flags = self.optional_memflags()?;
                let addr = self.match_value("expected SSA value address")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let expected = self.match_value("expected SSA value address")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let replacement = self.match_value("expected SSA value address")?;
                InstructionData::AtomicCas {
                    opcode,
                    flags,
                    args: [addr, expected, replacement],
                }
            }
            InstructionFormat::AtomicRmw => {
                let flags = self.optional_memflags()?;
                let op = self.match_enum("expected AtomicRmwOp")?;
                let addr = self.match_value("expected SSA value address")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let arg2 = self.match_value("expected SSA value address")?;
                InstructionData::AtomicRmw {
                    opcode,
                    flags,
                    op,
                    args: [addr, arg2],
                }
            }
            InstructionFormat::LoadNoOffset => {
                let flags = self.optional_memflags()?;
                let addr = self.match_value("expected SSA value address")?;
                InstructionData::LoadNoOffset {
                    opcode,
                    flags,
                    arg: addr,
                }
            }
            InstructionFormat::StoreNoOffset => {
                let flags = self.optional_memflags()?;
                let arg = self.match_value("expected SSA value operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let addr = self.match_value("expected SSA value address")?;
                InstructionData::StoreNoOffset {
                    opcode,
                    flags,
                    args: [arg, addr],
                }
            }
            InstructionFormat::IntAddTrap => {
                let a = self.match_value("expected SSA value operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let b = self.match_value("expected SSA value operand")?;
                self.match_token(Token::Comma, "expected ',' between operands")?;
                let code = self.match_enum("expected trap code")?;
                InstructionData::IntAddTrap {
                    opcode,
                    args: [a, b],
                    code,
                }
            }
        };
        Ok(idata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isaspec::IsaSpec;

    #[test]
    fn argument_type() {
        let mut p = Parser::new("i32 sext");
        let arg = p.parse_abi_param().unwrap();
        assert_eq!(arg.value_type, types::I32);
        assert_eq!(arg.extension, ArgumentExtension::Sext);
        assert_eq!(arg.purpose, ArgumentPurpose::Normal);
        let ParseError {
            location,
            message,
            is_warning,
        } = p.parse_abi_param().unwrap_err();
        assert_eq!(location.line_number, 1);
        assert_eq!(message, "expected parameter type");
        assert!(!is_warning);
    }

    #[test]
    fn aliases() {
        let (func, details) = Parser::new(
            "function %qux() system_v {
                                           block0:
                                             v4 = iconst.i8 6
                                             v3 -> v4
                                             v1 = iadd_imm v3, 17
                                           }",
        )
        .parse_function()
        .unwrap();
        assert_eq!(func.name.to_string(), "%qux");
        let v4 = details.map.lookup_str("v4").unwrap();
        assert_eq!(v4.to_string(), "v4");
        let v3 = details.map.lookup_str("v3").unwrap();
        assert_eq!(v3.to_string(), "v3");
        match v3 {
            AnyEntity::Value(v3) => {
                let aliased_to = func.dfg.resolve_aliases(v3);
                assert_eq!(aliased_to.to_string(), "v4");
            }
            _ => panic!("expected value: {}", v3),
        }
    }

    #[test]
    fn signature() {
        let sig = Parser::new("()system_v").parse_signature().unwrap();
        assert_eq!(sig.params.len(), 0);
        assert_eq!(sig.returns.len(), 0);
        assert_eq!(sig.call_conv, CallConv::SystemV);

        let sig2 = Parser::new("(i8 uext, f32, f64, i32 sret) -> i32 sext, f64 system_v")
            .parse_signature()
            .unwrap();
        assert_eq!(
            sig2.to_string(),
            "(i8 uext, f32, f64, i32 sret) -> i32 sext, f64 system_v"
        );
        assert_eq!(sig2.call_conv, CallConv::SystemV);

        // Old-style signature without a calling convention.
        assert_eq!(
            Parser::new("()").parse_signature().unwrap().to_string(),
            "() fast"
        );
        assert_eq!(
            Parser::new("() notacc")
                .parse_signature()
                .unwrap_err()
                .to_string(),
            "1: unknown calling convention: notacc"
        );

        // `void` is not recognized as a type by the lexer. It should not appear in files.
        assert_eq!(
            Parser::new("() -> void")
                .parse_signature()
                .unwrap_err()
                .to_string(),
            "1: expected parameter type"
        );
        assert_eq!(
            Parser::new("i8 -> i8")
                .parse_signature()
                .unwrap_err()
                .to_string(),
            "1: expected function signature: ( args... )"
        );
        assert_eq!(
            Parser::new("(i8 -> i8")
                .parse_signature()
                .unwrap_err()
                .to_string(),
            "1: expected ')' after function arguments"
        );
    }

    #[test]
    fn stack_slot_decl() {
        let (func, _) = Parser::new(
            "function %foo() system_v {
                                       ss3 = explicit_slot 13
                                       ss1 = explicit_slot 1
                                     }",
        )
        .parse_function()
        .unwrap();
        assert_eq!(func.name.to_string(), "%foo");
        let mut iter = func.sized_stack_slots.keys();
        let _ss0 = iter.next().unwrap();
        let ss1 = iter.next().unwrap();
        assert_eq!(ss1.to_string(), "ss1");
        assert_eq!(
            func.sized_stack_slots[ss1].kind,
            StackSlotKind::ExplicitSlot
        );
        assert_eq!(func.sized_stack_slots[ss1].size, 1);
        let _ss2 = iter.next().unwrap();
        let ss3 = iter.next().unwrap();
        assert_eq!(ss3.to_string(), "ss3");
        assert_eq!(
            func.sized_stack_slots[ss3].kind,
            StackSlotKind::ExplicitSlot
        );
        assert_eq!(func.sized_stack_slots[ss3].size, 13);
        assert_eq!(iter.next(), None);

        // Catch duplicate definitions.
        assert_eq!(
            Parser::new(
                "function %bar() system_v {
                                    ss1  = explicit_slot 13
                                    ss1  = explicit_slot 1
                                }",
            )
            .parse_function()
            .unwrap_err()
            .to_string(),
            "3: duplicate entity: ss1"
        );
    }

    #[test]
    fn block_header() {
        let (func, _) = Parser::new(
            "function %blocks() system_v {
                                     block0:
                                     block4(v3: i32):
                                     }",
        )
        .parse_function()
        .unwrap();
        assert_eq!(func.name.to_string(), "%blocks");

        let mut blocks = func.layout.blocks();

        let block0 = blocks.next().unwrap();
        assert_eq!(func.dfg.block_params(block0), &[]);

        let block4 = blocks.next().unwrap();
        let block4_args = func.dfg.block_params(block4);
        assert_eq!(block4_args.len(), 1);
        assert_eq!(func.dfg.value_type(block4_args[0]), types::I32);
    }

    #[test]
    fn duplicate_block() {
        let ParseError {
            location,
            message,
            is_warning,
        } = Parser::new(
            "function %blocks() system_v {
                block0:
                block0:
                    return 2",
        )
        .parse_function()
        .unwrap_err();

        assert_eq!(location.line_number, 3);
        assert_eq!(message, "duplicate entity: block0");
        assert!(!is_warning);
    }

    #[test]
    fn number_of_blocks() {
        let ParseError {
            location,
            message,
            is_warning,
        } = Parser::new(
            "function %a() {
                block100000:",
        )
        .parse_function()
        .unwrap_err();

        assert_eq!(location.line_number, 2);
        assert_eq!(message, "too many blocks");
        assert!(!is_warning);
    }

    #[test]
    fn duplicate_ss() {
        let ParseError {
            location,
            message,
            is_warning,
        } = Parser::new(
            "function %blocks() system_v {
                ss0 = explicit_slot 8
                ss0 = explicit_slot 8",
        )
        .parse_function()
        .unwrap_err();

        assert_eq!(location.line_number, 3);
        assert_eq!(message, "duplicate entity: ss0");
        assert!(!is_warning);
    }

    #[test]
    fn duplicate_gv() {
        let ParseError {
            location,
            message,
            is_warning,
        } = Parser::new(
            "function %blocks() system_v {
                gv0 = vmctx
                gv0 = vmctx",
        )
        .parse_function()
        .unwrap_err();

        assert_eq!(location.line_number, 3);
        assert_eq!(message, "duplicate entity: gv0");
        assert!(!is_warning);
    }

    #[test]
    fn duplicate_sig() {
        let ParseError {
            location,
            message,
            is_warning,
        } = Parser::new(
            "function %blocks() system_v {
                sig0 = ()
                sig0 = ()",
        )
        .parse_function()
        .unwrap_err();

        assert_eq!(location.line_number, 3);
        assert_eq!(message, "duplicate entity: sig0");
        assert!(!is_warning);
    }

    #[test]
    fn duplicate_fn() {
        let ParseError {
            location,
            message,
            is_warning,
        } = Parser::new(
            "function %blocks() system_v {
                sig0 = ()
                fn0 = %foo sig0
                fn0 = %foo sig0",
        )
        .parse_function()
        .unwrap_err();

        assert_eq!(location.line_number, 4);
        assert_eq!(message, "duplicate entity: fn0");
        assert!(!is_warning);
    }

    #[test]
    fn comments() {
        let (func, Details { comments, .. }) = Parser::new(
            "; before
                         function %comment() system_v { ; decl
                            ss10  = explicit_slot 13 ; stackslot.
                            ; Still stackslot.
                         block0: ; Basic block
                         trap user42; Instruction
                         } ; Trailing.
                         ; More trailing.",
        )
        .parse_function()
        .unwrap();
        assert_eq!(func.name.to_string(), "%comment");
        assert_eq!(comments.len(), 7); // no 'before' comment.
        assert_eq!(
            comments[0],
            Comment {
                entity: AnyEntity::Function,
                text: "; decl",
            }
        );
        assert_eq!(comments[1].entity.to_string(), "ss10");
        assert_eq!(comments[2].entity.to_string(), "ss10");
        assert_eq!(comments[2].text, "; Still stackslot.");
        assert_eq!(comments[3].entity.to_string(), "block0");
        assert_eq!(comments[3].text, "; Basic block");

        assert_eq!(comments[4].entity.to_string(), "inst0");
        assert_eq!(comments[4].text, "; Instruction");

        assert_eq!(comments[5].entity, AnyEntity::Function);
        assert_eq!(comments[6].entity, AnyEntity::Function);
    }

    #[test]
    fn test_file() {
        let tf = parse_test(
            r#"; before
                             test cfg option=5
                             test verify
                             set enable_float=false
                             feature "foo"
                             feature !"bar"
                             ; still preamble
                             function %comment() system_v {}"#,
            ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(tf.commands.len(), 2);
        assert_eq!(tf.commands[0].command, "cfg");
        assert_eq!(tf.commands[1].command, "verify");
        match tf.isa_spec {
            IsaSpec::None(s) => {
                assert!(s.enable_verifier());
                assert!(!s.enable_float());
            }
            _ => panic!("unexpected ISAs"),
        }
        assert_eq!(tf.features[0], Feature::With(&"foo"));
        assert_eq!(tf.features[1], Feature::Without(&"bar"));
        assert_eq!(tf.preamble_comments.len(), 2);
        assert_eq!(tf.preamble_comments[0].text, "; before");
        assert_eq!(tf.preamble_comments[1].text, "; still preamble");
        assert_eq!(tf.functions.len(), 1);
        assert_eq!(tf.functions[0].0.name.to_string(), "%comment");
    }

    #[test]
    fn isa_spec() {
        assert!(parse_test(
            "target
                            function %foo() system_v {}",
            ParseOptions::default()
        )
        .is_err());

        assert!(parse_test(
            "target x86_64
                            set enable_float=false
                            function %foo() system_v {}",
            ParseOptions::default()
        )
        .is_err());

        match parse_test(
            "set enable_float=false
                          target x86_64
                          function %foo() system_v {}",
            ParseOptions::default(),
        )
        .unwrap()
        .isa_spec
        {
            IsaSpec::None(_) => panic!("Expected some ISA"),
            IsaSpec::Some(v) => {
                assert_eq!(v.len(), 1);
                assert!(v[0].name() == "x64" || v[0].name() == "x86");
            }
        }
    }

    #[test]
    fn user_function_name() {
        // Valid characters in the name:
        let func = Parser::new(
            "function u1:2() system_v {
                                           block0:
                                             trap int_divz
                                           }",
        )
        .parse_function()
        .unwrap()
        .0;
        assert_eq!(func.name.to_string(), "u1:2");

        // Invalid characters in the name:
        let mut parser = Parser::new(
            "function u123:abc() system_v {
                                           block0:
                                             trap stk_ovf
                                           }",
        );
        assert!(parser.parse_function().is_err());

        // Incomplete function names should not be valid:
        let mut parser = Parser::new(
            "function u() system_v {
                                           block0:
                                             trap int_ovf
                                           }",
        );
        assert!(parser.parse_function().is_err());

        let mut parser = Parser::new(
            "function u0() system_v {
                                           block0:
                                             trap int_ovf
                                           }",
        );
        assert!(parser.parse_function().is_err());

        let mut parser = Parser::new(
            "function u0:() system_v {
                                           block0:
                                             trap int_ovf
                                           }",
        );
        assert!(parser.parse_function().is_err());
    }

    #[test]
    fn change_default_calling_convention() {
        let code = "function %test() {
        block0:
            return
        }";

        // By default the parser will use the fast calling convention if none is specified.
        let mut parser = Parser::new(code);
        assert_eq!(
            parser.parse_function().unwrap().0.signature.call_conv,
            CallConv::Fast
        );

        // However, we can specify a different calling convention to be the default.
        let mut parser = Parser::new(code).with_default_calling_convention(CallConv::Cold);
        assert_eq!(
            parser.parse_function().unwrap().0.signature.call_conv,
            CallConv::Cold
        );
    }

    #[test]
    fn u8_as_hex() {
        fn parse_as_uimm8(text: &str) -> ParseResult<u8> {
            Parser::new(text).match_uimm8("unable to parse u8")
        }

        assert_eq!(parse_as_uimm8("0").unwrap(), 0);
        assert_eq!(parse_as_uimm8("0xff").unwrap(), 255);
        assert!(parse_as_uimm8("-1").is_err());
        assert!(parse_as_uimm8("0xffa").is_err());
    }

    #[test]
    fn i16_as_hex() {
        fn parse_as_imm16(text: &str) -> ParseResult<i16> {
            Parser::new(text).match_imm16("unable to parse i16")
        }

        assert_eq!(parse_as_imm16("0x8000").unwrap(), -32768);
        assert_eq!(parse_as_imm16("0xffff").unwrap(), -1);
        assert_eq!(parse_as_imm16("0").unwrap(), 0);
        assert_eq!(parse_as_imm16("0x7fff").unwrap(), 32767);
        assert_eq!(
            parse_as_imm16("-0x0001").unwrap(),
            parse_as_imm16("0xffff").unwrap()
        );
        assert_eq!(
            parse_as_imm16("-0x7fff").unwrap(),
            parse_as_imm16("0x8001").unwrap()
        );
        assert!(parse_as_imm16("0xffffa").is_err());
    }

    #[test]
    fn i32_as_hex() {
        fn parse_as_imm32(text: &str) -> ParseResult<i32> {
            Parser::new(text).match_imm32("unable to parse i32")
        }

        assert_eq!(parse_as_imm32("0x80000000").unwrap(), -2147483648);
        assert_eq!(parse_as_imm32("0xffffffff").unwrap(), -1);
        assert_eq!(parse_as_imm32("0").unwrap(), 0);
        assert_eq!(parse_as_imm32("0x7fffffff").unwrap(), 2147483647);
        assert_eq!(
            parse_as_imm32("-0x00000001").unwrap(),
            parse_as_imm32("0xffffffff").unwrap()
        );
        assert_eq!(
            parse_as_imm32("-0x7fffffff").unwrap(),
            parse_as_imm32("0x80000001").unwrap()
        );
        assert!(parse_as_imm32("0xffffffffa").is_err());
    }

    #[test]
    fn i64_as_hex() {
        fn parse_as_imm64(text: &str) -> ParseResult<Imm64> {
            Parser::new(text).match_imm64("unable to parse Imm64")
        }

        assert_eq!(
            parse_as_imm64("0x8000000000000000").unwrap(),
            Imm64::new(-9223372036854775808)
        );
        assert_eq!(
            parse_as_imm64("0xffffffffffffffff").unwrap(),
            Imm64::new(-1)
        );
        assert_eq!(parse_as_imm64("0").unwrap(), Imm64::new(0));
        assert_eq!(
            parse_as_imm64("0x7fffffffffffffff").unwrap(),
            Imm64::new(9223372036854775807)
        );
        assert_eq!(
            parse_as_imm64("-0x0000000000000001").unwrap(),
            parse_as_imm64("0xffffffffffffffff").unwrap()
        );
        assert_eq!(
            parse_as_imm64("-0x7fffffffffffffff").unwrap(),
            parse_as_imm64("0x8000000000000001").unwrap()
        );
        assert!(parse_as_imm64("0xffffffffffffffffa").is_err());
    }

    #[test]
    fn uimm128() {
        macro_rules! parse_as_constant_data {
            ($text:expr, $type:expr) => {{
                Parser::new($text).parse_literals_to_constant_data($type)
            }};
        }
        macro_rules! can_parse_as_constant_data {
            ($text:expr, $type:expr) => {{
                assert!(parse_as_constant_data!($text, $type).is_ok())
            }};
        }
        macro_rules! cannot_parse_as_constant_data {
            ($text:expr, $type:expr) => {{
                assert!(parse_as_constant_data!($text, $type).is_err())
            }};
        }

        can_parse_as_constant_data!("1 2 3 4", I32X4);
        can_parse_as_constant_data!("1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16", I8X16);
        can_parse_as_constant_data!("0x1.1 0x2.2 0x3.3 0x4.4", F32X4);
        can_parse_as_constant_data!("0x0 0x1 0x2 0x3", I32X4);
        can_parse_as_constant_data!("-1 0 -1 0 -1 0 -1 0", I16X8);
        can_parse_as_constant_data!("0 -1", I64X2);
        can_parse_as_constant_data!("-1 0", I64X2);
        can_parse_as_constant_data!("-1 -1 -1 -1 -1", I32X4); // note that parse_literals_to_constant_data will leave extra tokens unconsumed

        cannot_parse_as_constant_data!("1 2 3", I32X4);
        cannot_parse_as_constant_data!(" ", F32X4);
    }

    #[test]
    fn parse_constant_from_booleans() {
        let c = Parser::new("-1 0 -1 0")
            .parse_literals_to_constant_data(I32X4)
            .unwrap();
        assert_eq!(
            c.into_vec(),
            [0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0]
        )
    }

    #[test]
    fn parse_unbounded_constants() {
        // Unlike match_uimm128, match_hexadecimal_constant can parse byte sequences of any size:
        assert_eq!(
            Parser::new("0x0100")
                .match_hexadecimal_constant("err message")
                .unwrap(),
            vec![0, 1].into()
        );

        // Only parse hexadecimal constants:
        assert!(Parser::new("228")
            .match_hexadecimal_constant("err message")
            .is_err());
    }

    #[test]
    fn parse_run_commands() {
        // Helper for creating signatures.
        fn sig(ins: &[Type], outs: &[Type]) -> Signature {
            let mut sig = Signature::new(CallConv::Fast);
            for i in ins {
                sig.params.push(AbiParam::new(*i));
            }
            for o in outs {
                sig.returns.push(AbiParam::new(*o));
            }
            sig
        }

        // Helper for parsing run commands.
        fn parse(text: &str, sig: &Signature) -> ParseResult<RunCommand> {
            Parser::new(text).parse_run_command(sig)
        }

        // Check that we can parse and display the same set of run commands.
        fn assert_roundtrip(text: &str, sig: &Signature) {
            assert_eq!(parse(text, sig).unwrap().to_string(), text);
        }
        assert_roundtrip("run: %fn0() == 42", &sig(&[], &[I32]));
        assert_roundtrip(
            "run: %fn0(8, 16, 32, 64) == 1",
            &sig(&[I8, I16, I32, I64], &[I8]),
        );
        assert_roundtrip(
            "run: %my_func(1) == 0x0f0e0d0c0b0a09080706050403020100",
            &sig(&[I32], &[I8X16]),
        );

        // Verify that default invocations are created when not specified.
        assert_eq!(
            parse("run", &sig(&[], &[I32])).unwrap().to_string(),
            "run: %default() != 0"
        );
        assert_eq!(
            parse("print", &sig(&[], &[F32X4, I16X8]))
                .unwrap()
                .to_string(),
            "print: %default()"
        );

        // Demonstrate some unparsable cases.
        assert!(parse("print", &sig(&[I32], &[I32])).is_err());
        assert!(parse("print:", &sig(&[], &[])).is_err());
        assert!(parse("run: ", &sig(&[], &[])).is_err());
    }

    #[test]
    fn parse_data_values() {
        fn parse(text: &str, ty: Type) -> DataValue {
            Parser::new(text).parse_data_value(ty).unwrap()
        }

        assert_eq!(parse("8", I8).to_string(), "8");
        assert_eq!(parse("16", I16).to_string(), "16");
        assert_eq!(parse("32", I32).to_string(), "32");
        assert_eq!(parse("64", I64).to_string(), "64");
        assert_eq!(
            parse("0x01234567_01234567_01234567_01234567", I128).to_string(),
            "1512366032949150931280199141537564007"
        );
        assert_eq!(parse("1234567", I128).to_string(), "1234567");
        assert_eq!(parse("0x32.32", F32).to_string(), "0x1.919000p5");
        assert_eq!(parse("0x64.64", F64).to_string(), "0x1.9190000000000p6");
        assert_eq!(
            parse("[0 1 2 3]", I32X4).to_string(),
            "0x00000003000000020000000100000000"
        );
    }

    #[test]
    fn parse_cold_blocks() {
        let code = "function %test() {
        block0 cold:
            return
        block1(v0: i32) cold:
            return
        block2(v1: i32):
            return
        }";

        let mut parser = Parser::new(code);
        let func = parser.parse_function().unwrap().0;
        assert_eq!(func.layout.blocks().count(), 3);
        assert!(func.layout.is_cold(Block::from_u32(0)));
        assert!(func.layout.is_cold(Block::from_u32(1)));
        assert!(!func.layout.is_cold(Block::from_u32(2)));
    }
}

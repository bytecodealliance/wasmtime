use std::rc::Rc;

use crate::cdsl::ast::Literal;
use crate::cdsl::formats::{FormatRegistry, InstructionFormat};
use crate::cdsl::instructions::InstructionPredicate;
use crate::cdsl::recipes::{
    EncodingRecipe, EncodingRecipeBuilder, OperandConstraint, Register, Stack,
};
use crate::cdsl::regs::IsaRegs;
use crate::cdsl::settings::SettingGroup;
use crate::shared::Definitions as SharedDefinitions;

/// Helper data structure to create recipes and template recipes.
/// It contains all the recipes and recipe templates that might be used in the encodings crate of
/// this same directory.
pub struct RecipeGroup<'builder> {
    /// Memoized format pointer, to pass it to builders later.
    formats: &'builder FormatRegistry,

    /// Memoized registers description, to pass it to builders later.
    regs: &'builder IsaRegs,

    /// All the recipes explicitly created in this file. This is different from the final set of
    /// recipes, which is definitive only once encodings have generated new recipes on the fly.
    recipes: Vec<EncodingRecipe>,

    /// All the recipe templates created in this file.
    templates: Vec<Rc<Template<'builder>>>,
}

impl<'builder> RecipeGroup<'builder> {
    fn new(formats: &'builder FormatRegistry, regs: &'builder IsaRegs) -> Self {
        Self {
            formats,
            regs,
            recipes: Vec::new(),
            templates: Vec::new(),
        }
    }
    fn add_recipe(&mut self, recipe: EncodingRecipeBuilder) {
        self.recipes.push(recipe.build(self.formats));
    }
    fn add_template_recipe(&mut self, recipe: EncodingRecipeBuilder) -> Rc<Template<'builder>> {
        let template = Rc::new(Template::new(recipe, self.formats, self.regs));
        self.templates.push(template.clone());
        template
    }
    fn add_template(&mut self, template: Template<'builder>) -> Rc<Template<'builder>> {
        let template = Rc::new(template);
        self.templates.push(template.clone());
        template
    }
    pub fn recipe(&self, name: &str) -> &EncodingRecipe {
        self.recipes
            .iter()
            .find(|recipe| &recipe.name == name)
            .expect(&format!("unknown recipe name: {}. Try template?", name))
    }
    pub fn template(&self, name: &str) -> &Template {
        self.templates
            .iter()
            .find(|recipe| recipe.name() == name)
            .expect(&format!("unknown tail recipe name: {}. Try recipe?", name))
    }
}

// Opcode representation.
//
// Cranelift requires each recipe to have a single encoding size in bytes, and x86 opcodes are
// variable length, so we use separate recipes for different styles of opcodes and prefixes. The
// opcode format is indicated by the recipe name prefix.
//
// The match case below does not include the REX prefix which goes after the mandatory prefix.
// VEX/XOP and EVEX prefixes are not yet supported. Encodings using any of these prefixes are
// represented by separate recipes.
//
// The encoding bits are:
//
// 0-7:   The opcode byte <op>.
// 8-9:   pp, mandatory prefix:
//        00 none (Op*)
//        01 66   (Mp*)
//        10 F3   (Mp*)
//        11 F2   (Mp*)
// 10-11: mm, opcode map:
//        00 <op>        (Op1/Mp1)
//        01 0F <op>     (Op2/Mp2)
//        10 0F 38 <op>  (Op3/Mp3)
//        11 0F 3A <op>  (Op3/Mp3)
// 12-14  rrr, opcode bits for the ModR/M byte for certain opcodes.
// 15:    REX.W bit (or VEX.W/E)
//
// There is some redundancy between bits 8-11 and the recipe names, but we have enough bits, and
// the pp+mm format is ready for supporting VEX prefixes.
//
// TODO Cranelift doesn't actually require recipe to have different encoding sizes anymore, so this
// could be simplified.

/// Given a sequence of opcode bytes, compute the recipe name prefix and encoding bits.
fn decode_opcodes(op_bytes: &[u8], rrr: u16, w: u16) -> (&'static str, u16) {
    assert!(op_bytes.len() >= 1, "at least one opcode byte");

    let prefix_bytes = &op_bytes[..op_bytes.len() - 1];
    let (name, mmpp) = match prefix_bytes {
        [] => ("Op1", 0b000),
        [0x66] => ("Mp1", 0b0001),
        [0xf3] => ("Mp1", 0b0010),
        [0xf2] => ("Mp1", 0b0011),
        [0x0f] => ("Op2", 0b0100),
        [0x66, 0x0f] => ("Mp2", 0b0101),
        [0xf3, 0x0f] => ("Mp2", 0b0110),
        [0xf2, 0x0f] => ("Mp2", 0b0111),
        [0x0f, 0x38] => ("Op3", 0b1000),
        [0x66, 0x0f, 0x38] => ("Mp3", 0b1001),
        [0xf3, 0x0f, 0x38] => ("Mp3", 0b1010),
        [0xf2, 0x0f, 0x38] => ("Mp3", 0b1011),
        [0x0f, 0x3a] => ("Op3", 0b1100),
        [0x66, 0x0f, 0x3a] => ("Mp3", 0b1101),
        [0xf3, 0x0f, 0x3a] => ("Mp3", 0b1110),
        [0xf2, 0x0f, 0x3a] => ("Mp3", 0b1111),
        _ => {
            panic!("unexpected opcode sequence: {:?}", op_bytes);
        }
    };

    let opcode_byte = op_bytes[op_bytes.len() - 1] as u16;
    (name, opcode_byte | (mmpp << 8) | (rrr << 12) | w << 15)
}

/// Given a snippet of Rust code (or None), replace the `PUT_OP` macro with the
/// corresponding `put_*` function from the `binemit.rs` module.
fn replace_put_op(code: Option<String>, prefix: &str) -> Option<String> {
    code.map(|code| code.replace("{{PUT_OP}}", &format!("put_{}", prefix.to_lowercase())))
}

/// Replaces constraints to a REX-prefixed register class by the equivalent non-REX register class.
fn replace_nonrex_constraints(
    regs: &IsaRegs,
    constraints: Vec<OperandConstraint>,
) -> Vec<OperandConstraint> {
    constraints
        .into_iter()
        .map(|constraint| match constraint {
            OperandConstraint::RegClass(rc_index) => {
                let new_rc_index = if rc_index == regs.class_by_name("GPR") {
                    regs.class_by_name("GPR8")
                } else if rc_index == regs.class_by_name("FPR") {
                    regs.class_by_name("FPR8")
                } else {
                    rc_index
                };
                OperandConstraint::RegClass(new_rc_index)
            }
            _ => constraint,
        })
        .collect()
}

/// Previously called a TailRecipe in the Python meta language, this allows to create multiple
/// variants of a single base EncodingRecipe (rex prefix, specialized w/rrr bits, different
/// opcodes). It serves as a prototype of an EncodingRecipe, which is then used when actually creating
/// Encodings, in encodings.rs. This is an idiosyncrasy of the x86 meta-language, and could be
/// reconsidered later.
#[derive(Clone)]
pub struct Template<'builder> {
    /// Mapping of format indexes to format data, used in the build() method.
    formats: &'builder FormatRegistry,

    /// Description of registers, used in the build() method.
    regs: &'builder IsaRegs,

    /// The recipe template, which is to be specialized (by copy).
    recipe: EncodingRecipeBuilder,

    /// Does this recipe requires a REX prefix?
    requires_prefix: bool,

    /// Other recipe to use when REX-prefixed.
    when_prefixed: Option<Rc<Template<'builder>>>,

    // Specialized parameters.
    /// Should we include the REX prefix?
    rex: bool,
    /// Value of the W bit (0 or 1).
    w_bit: u16,
    /// Value of the RRR bits (between 0 and 0b111).
    rrr_bits: u16,
    /// Opcode bytes.
    op_bytes: Vec<u8>,
}

impl<'builder> Template<'builder> {
    fn new(
        recipe: EncodingRecipeBuilder,
        formats: &'builder FormatRegistry,
        regs: &'builder IsaRegs,
    ) -> Self {
        Self {
            formats,
            regs,
            recipe,
            requires_prefix: false,
            when_prefixed: None,
            rex: false,
            w_bit: 0,
            rrr_bits: 0,
            op_bytes: Vec::new(),
        }
    }

    fn name(&self) -> &str {
        &self.recipe.name
    }
    fn requires_prefix(self, value: bool) -> Self {
        Self {
            requires_prefix: value,
            ..self
        }
    }
    fn when_prefixed(self, template: Rc<Template<'builder>>) -> Self {
        assert!(self.when_prefixed.is_none());
        Self {
            when_prefixed: Some(template),
            ..self
        }
    }

    // Copy setters.
    pub fn opcodes(&self, op_bytes: Vec<u8>) -> Self {
        assert!(!op_bytes.is_empty());
        let mut copy = self.clone();
        copy.op_bytes = op_bytes;
        copy
    }
    pub fn w(&self) -> Self {
        let mut copy = self.clone();
        copy.w_bit = 1;
        copy
    }
    pub fn rrr(&self, value: u16) -> Self {
        assert!(value <= 0b111);
        let mut copy = self.clone();
        copy.rrr_bits = value;
        copy
    }
    pub fn nonrex(&self) -> Self {
        assert!(!self.requires_prefix, "Tail recipe requires REX prefix.");
        let mut copy = self.clone();
        copy.rex = false;
        copy
    }
    pub fn rex(&self) -> Self {
        if let Some(prefixed) = &self.when_prefixed {
            let mut ret = prefixed.rex();
            // Forward specialized parameters.
            ret.op_bytes = self.op_bytes.clone();
            ret.w_bit = self.w_bit;
            ret.rrr_bits = self.rrr_bits;
            return ret;
        }
        let mut copy = self.clone();
        copy.rex = true;
        copy
    }

    pub fn build(mut self) -> (EncodingRecipe, u16) {
        let (name, bits) = decode_opcodes(&self.op_bytes, self.rrr_bits, self.w_bit);

        let (name, rex_prefix_size) = if self.rex {
            ("Rex".to_string() + name, 1)
        } else {
            (name.into(), 0)
        };

        let size_addendum = self.op_bytes.len() as u64 + rex_prefix_size;
        self.recipe.base_size += size_addendum;

        // Branch ranges are relative to the end of the instruction.
        self.recipe
            .branch_range
            .as_mut()
            .map(|range| range.inst_size += size_addendum);

        self.recipe.emit = replace_put_op(self.recipe.emit, &name);
        self.recipe.name = name + &self.recipe.name;

        if !self.rex {
            let operands_in = self.recipe.operands_in.unwrap_or(Vec::new());
            self.recipe.operands_in = Some(replace_nonrex_constraints(self.regs, operands_in));
            let operands_out = self.recipe.operands_out.unwrap_or(Vec::new());
            self.recipe.operands_out = Some(replace_nonrex_constraints(self.regs, operands_out));
        }

        (self.recipe.build(self.formats), bits)
    }
}

/// Returns a predicate checking that the "cond" field of the instruction contains one of the
/// directly supported floating point condition codes.
fn supported_floatccs_predicate(
    supported_cc: &[Literal],
    format: &InstructionFormat,
) -> InstructionPredicate {
    supported_cc
        .iter()
        .fold(InstructionPredicate::new(), |pred, literal| {
            pred.or(InstructionPredicate::new_is_field_equal(
                format,
                "cond",
                literal.to_rust_code(),
            ))
        })
}

/// Return an instruction predicate that checks if `iform.imm` is a valid `scale` for a SIB byte.
fn valid_scale(format: &InstructionFormat) -> InstructionPredicate {
    ["1", "2", "4", "8"]
        .iter()
        .fold(InstructionPredicate::new(), |pred, &literal| {
            pred.or(InstructionPredicate::new_is_field_equal(
                format,
                "imm",
                literal.into(),
            ))
        })
}

pub(crate) fn define<'shared>(
    shared_defs: &'shared SharedDefinitions,
    settings: &'shared SettingGroup,
    regs: &'shared IsaRegs,
) -> RecipeGroup<'shared> {
    // The set of floating point condition codes that are directly supported.
    // Other condition codes need to be reversed or expressed as two tests.
    let floatcc = &shared_defs.imm.floatcc;
    let supported_floatccs: Vec<Literal> = ["ord", "uno", "one", "ueq", "gt", "ge", "ult", "ule"]
        .iter()
        .map(|name| Literal::enumerator_for(floatcc, name))
        .collect();

    let formats = &shared_defs.format_registry;

    // Register classes shorthands.
    let abcd = regs.class_by_name("ABCD");
    let gpr = regs.class_by_name("GPR");
    let fpr = regs.class_by_name("FPR");
    let flag = regs.class_by_name("FLAG");

    // Operand constraints shorthands.
    let reg_rflags = Register::new(flag, regs.regunit_by_name(flag, "rflags"));
    let reg_rax = Register::new(gpr, regs.regunit_by_name(gpr, "rax"));
    let reg_rcx = Register::new(gpr, regs.regunit_by_name(gpr, "rcx"));
    let reg_rdx = Register::new(gpr, regs.regunit_by_name(gpr, "rdx"));
    let reg_r15 = Register::new(gpr, regs.regunit_by_name(gpr, "r15"));

    // Stack operand with a 32-bit signed displacement from either RBP or RSP.
    let stack_gpr32 = Stack::new(gpr);
    let stack_fpr32 = Stack::new(fpr);

    // Format shorthands, prefixed with f_.
    let f_binary = formats.by_name("Binary");
    let f_binary_imm = formats.by_name("BinaryImm");
    let f_branch = formats.by_name("Branch");
    let f_branch_float = formats.by_name("BranchFloat");
    let f_branch_int = formats.by_name("BranchInt");
    let f_branch_table_entry = formats.by_name("BranchTableEntry");
    let f_branch_table_base = formats.by_name("BranchTableBase");
    let f_call = formats.by_name("Call");
    let f_call_indirect = formats.by_name("CallIndirect");
    let f_copy_special = formats.by_name("CopySpecial");
    let f_copy_to_ssa = formats.by_name("CopyToSsa");
    let f_extract_lane = formats.by_name("ExtractLane"); // TODO this would preferably retrieve a BinaryImm8 format but because formats are compared structurally and ExtractLane has the same structure this is impossible--if we rename ExtractLane, it may even impact parsing
    let f_float_compare = formats.by_name("FloatCompare");
    let f_float_cond = formats.by_name("FloatCond");
    let f_float_cond_trap = formats.by_name("FloatCondTrap");
    let f_func_addr = formats.by_name("FuncAddr");
    let f_indirect_jump = formats.by_name("IndirectJump");
    let f_insert_lane = formats.by_name("InsertLane");
    let f_int_compare = formats.by_name("IntCompare");
    let f_int_compare_imm = formats.by_name("IntCompareImm");
    let f_int_cond = formats.by_name("IntCond");
    let f_int_cond_trap = formats.by_name("IntCondTrap");
    let f_int_select = formats.by_name("IntSelect");
    let f_jump = formats.by_name("Jump");
    let f_load = formats.by_name("Load");
    let f_load_complex = formats.by_name("LoadComplex");
    let f_multiary = formats.by_name("MultiAry");
    let f_nullary = formats.by_name("NullAry");
    let f_reg_fill = formats.by_name("RegFill");
    let f_reg_move = formats.by_name("RegMove");
    let f_reg_spill = formats.by_name("RegSpill");
    let f_stack_load = formats.by_name("StackLoad");
    let f_store = formats.by_name("Store");
    let f_store_complex = formats.by_name("StoreComplex");
    let f_ternary = formats.by_name("Ternary");
    let f_trap = formats.by_name("Trap");
    let f_unary = formats.by_name("Unary");
    let f_unary_bool = formats.by_name("UnaryBool");
    let f_unary_global_value = formats.by_name("UnaryGlobalValue");
    let f_unary_ieee32 = formats.by_name("UnaryIeee32");
    let f_unary_ieee64 = formats.by_name("UnaryIeee64");
    let f_unary_imm = formats.by_name("UnaryImm");
    let f_unary_imm128 = formats.by_name("UnaryImm128");

    // Predicates shorthands.
    let use_sse41 = settings.predicate_by_name("use_sse41");

    // Definitions.
    let mut recipes = RecipeGroup::new(formats, regs);

    // A null unary instruction that takes a GPR register. Can be used for identity copies and
    // no-op conversions.
    recipes.add_recipe(
        EncodingRecipeBuilder::new("null", f_unary, 0)
            .operands_in(vec![gpr])
            .operands_out(vec![0])
            .emit(""),
    );
    recipes.add_recipe(
        EncodingRecipeBuilder::new("null_fpr", f_unary, 0)
            .operands_in(vec![fpr])
            .operands_out(vec![0])
            .emit(""),
    );
    recipes.add_recipe(
        EncodingRecipeBuilder::new("stacknull", f_unary, 0)
            .operands_in(vec![stack_gpr32])
            .operands_out(vec![stack_gpr32])
            .emit(""),
    );

    recipes.add_recipe(
        EncodingRecipeBuilder::new("get_pinned_reg", f_nullary, 0)
            .operands_out(vec![reg_r15])
            .emit(""),
    );
    // umr with a fixed register output that's r15.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("set_pinned_reg", f_unary, 1)
            .operands_in(vec![gpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    let r15 = RU::r15.into();
                    {{PUT_OP}}(bits, rex2(r15, in_reg0), sink);
                    modrm_rr(r15, in_reg0, sink);
                "#,
            ),
    );

    // No-op fills, created by late-stage redundant-fill removal.
    recipes.add_recipe(
        EncodingRecipeBuilder::new("fillnull", f_unary, 0)
            .operands_in(vec![stack_gpr32])
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .emit(""),
    );
    recipes.add_recipe(
        EncodingRecipeBuilder::new("ffillnull", f_unary, 0)
            .operands_in(vec![stack_gpr32])
            .operands_out(vec![fpr])
            .clobbers_flags(false)
            .emit(""),
    );

    recipes
        .add_recipe(EncodingRecipeBuilder::new("debugtrap", f_nullary, 1).emit("sink.put1(0xcc);"));

    // XX opcode, no ModR/M.
    recipes.add_template_recipe(EncodingRecipeBuilder::new("trap", f_trap, 0).emit(
        r#"
            sink.trap(code, func.srclocs[inst]);
            {{PUT_OP}}(bits, BASE_REX, sink);
        "#,
    ));

    // Macro: conditional jump over a ud2.
    recipes.add_recipe(
        EncodingRecipeBuilder::new("trapif", f_int_cond_trap, 4)
            .operands_in(vec![reg_rflags])
            .clobbers_flags(false)
            .emit(
                r#"
                    // Jump over a 2-byte ud2.
                    sink.put1(0x70 | (icc2opc(cond.inverse()) as u8));
                    sink.put1(2);
                    // ud2.
                    sink.trap(code, func.srclocs[inst]);
                    sink.put1(0x0f);
                    sink.put1(0x0b);
                "#,
            ),
    );

    recipes.add_recipe(
        EncodingRecipeBuilder::new("trapff", f_float_cond_trap, 4)
            .operands_in(vec![reg_rflags])
            .clobbers_flags(false)
            .inst_predicate(supported_floatccs_predicate(
                &supported_floatccs,
                formats.get(f_float_cond_trap),
            ))
            .emit(
                r#"
                    // Jump over a 2-byte ud2.
                    sink.put1(0x70 | (fcc2opc(cond.inverse()) as u8));
                    sink.put1(2);
                    // ud2.
                    sink.trap(code, func.srclocs[inst]);
                    sink.put1(0x0f);
                    sink.put1(0x0b);
                "#,
            ),
    );

    // XX /r
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rr", f_binary, 1)
            .operands_in(vec![gpr, gpr])
            .operands_out(vec![0])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, in_reg1), sink);
                    modrm_rr(in_reg0, in_reg1, sink);
                "#,
            ),
    );

    // XX /r with operands swapped. (RM form).
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rrx", f_binary, 1)
            .operands_in(vec![gpr, gpr])
            .operands_out(vec![0])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                    modrm_rr(in_reg1, in_reg0, sink);
                "#,
            ),
    );

    // XX /r with FPR ins and outs. A form.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("fa", f_binary, 1)
            .operands_in(vec![fpr, fpr])
            .operands_out(vec![0])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                    modrm_rr(in_reg1, in_reg0, sink);
                "#,
            ),
    );

    // XX /r with FPR ins and outs. A form with input operands swapped.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("fax", f_binary, 1)
            .operands_in(vec![fpr, fpr])
            .operands_out(vec![1])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, in_reg1), sink);
                    modrm_rr(in_reg0, in_reg1, sink);
                "#,
            ),
    );

    // XX /r with FPR ins and outs. A form with a byte immediate.
    {
        let format = formats.get(f_insert_lane);
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("fa_ib", f_insert_lane, 2)
                .operands_in(vec![fpr, fpr])
                .operands_out(vec![0])
                .inst_predicate(InstructionPredicate::new_is_unsigned_int(
                    format, "lane", 8, 0,
                ))
                .emit(
                    r#"
                    {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                    modrm_rr(in_reg1, in_reg0, sink);
                    let imm:i64 = lane.into();
                    sink.put1(imm as u8);
                "#,
                ),
        );
    }

    // XX /n for a unary operation with extension bits.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("ur", f_unary, 1)
            .operands_in(vec![gpr])
            .operands_out(vec![0])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex1(in_reg0), sink);
                    modrm_r_bits(in_reg0, bits, sink);
                "#,
            ),
    );

    // XX /r, but for a unary operator with separate input/output register, like
    // copies. MR form, preserving flags.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("umr", f_unary, 1)
            .operands_in(vec![gpr])
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(out_reg0, in_reg0), sink);
                    modrm_rr(out_reg0, in_reg0, sink);
                "#,
            ),
    );

    // Same as umr, but with FPR -> GPR registers.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rfumr", f_unary, 1)
            .operands_in(vec![fpr])
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(out_reg0, in_reg0), sink);
                    modrm_rr(out_reg0, in_reg0, sink);
                "#,
            ),
    );

    // Same as umr, but with the source register specified directly.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("umr_reg_to_ssa", f_copy_to_ssa, 1)
            // No operands_in to mention, because a source register is specified directly.
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(out_reg0, src), sink);
                    modrm_rr(out_reg0, src, sink);
                "#,
            ),
    );

    // XX /r, but for a unary operator with separate input/output register.
    // RM form. Clobbers FLAGS.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("urm", f_unary, 1)
            .operands_in(vec![gpr])
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                    modrm_rr(in_reg0, out_reg0, sink);
                "#,
            ),
    );

    // XX /r. Same as urm, but doesn't clobber FLAGS.
    let urm_noflags = recipes.add_template_recipe(
        EncodingRecipeBuilder::new("urm_noflags", f_unary, 1)
            .operands_in(vec![gpr])
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                    modrm_rr(in_reg0, out_reg0, sink);
                "#,
            ),
    );

    // XX /r. Same as urm_noflags, but input limited to ABCD.
    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("urm_noflags_abcd", f_unary, 1)
                .operands_in(vec![abcd])
                .operands_out(vec![gpr])
                .clobbers_flags(false)
                .emit(
                    r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                    modrm_rr(in_reg0, out_reg0, sink);
                "#,
                ),
            formats,
            regs,
        )
        .when_prefixed(urm_noflags),
    );

    // XX /r, RM form, FPR -> FPR.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("furm", f_unary, 1)
            .operands_in(vec![fpr])
            .operands_out(vec![fpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                    modrm_rr(in_reg0, out_reg0, sink);
                "#,
            ),
    );

    // Same as furm, but with the source register specified directly.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("furm_reg_to_ssa", f_copy_to_ssa, 1)
            // No operands_in to mention, because a source register is specified directly.
            .operands_out(vec![fpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(src, out_reg0), sink);
                    modrm_rr(src, out_reg0, sink);
                "#,
            ),
    );

    // XX /r, RM form, GPR -> FPR.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("frurm", f_unary, 1)
            .operands_in(vec![gpr])
            .operands_out(vec![fpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                    modrm_rr(in_reg0, out_reg0, sink);
                "#,
            ),
    );

    // XX /r, RM form, FPR -> GPR.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rfurm", f_unary, 1)
            .operands_in(vec![fpr])
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                    modrm_rr(in_reg0, out_reg0, sink);
                "#,
            ),
    );

    // XX /r, RMI form for one of the roundXX SSE 4.1 instructions.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("furmi_rnd", f_unary, 2)
            .operands_in(vec![fpr])
            .operands_out(vec![fpr])
            .isa_predicate(use_sse41)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                    modrm_rr(in_reg0, out_reg0, sink);
                    sink.put1(match opcode {
                        Opcode::Nearest => 0b00,
                        Opcode::Floor => 0b01,
                        Opcode::Ceil => 0b10,
                        Opcode::Trunc => 0b11,
                        x => panic!("{} unexpected for furmi_rnd", opcode),
                    });
                "#,
            ),
    );

    // XX /r, for regmove instructions.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rmov", f_reg_move, 1)
            .operands_in(vec![gpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(dst, src), sink);
                    modrm_rr(dst, src, sink);
                "#,
            ),
    );

    // XX /r, for regmove instructions (FPR version, RM encoded).
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("frmov", f_reg_move, 1)
            .operands_in(vec![fpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(src, dst), sink);
                    modrm_rr(src, dst, sink);
                "#,
            ),
    );

    // XX /n with one arg in %rcx, for shifts.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rc", f_binary, 1)
            .operands_in(vec![
                OperandConstraint::RegClass(gpr),
                OperandConstraint::FixedReg(reg_rcx),
            ])
            .operands_out(vec![0])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex1(in_reg0), sink);
                    modrm_r_bits(in_reg0, bits, sink);
                "#,
            ),
    );

    // XX /n for division: inputs in %rax, %rdx, r. Outputs in %rax, %rdx.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("div", f_ternary, 1)
            .operands_in(vec![
                OperandConstraint::FixedReg(reg_rax),
                OperandConstraint::FixedReg(reg_rdx),
                OperandConstraint::RegClass(gpr),
            ])
            .operands_out(vec![reg_rax, reg_rdx])
            .emit(
                r#"
                    sink.trap(TrapCode::IntegerDivisionByZero, func.srclocs[inst]);
                    {{PUT_OP}}(bits, rex1(in_reg2), sink);
                    modrm_r_bits(in_reg2, bits, sink);
                "#,
            ),
    );

    // XX /n for {s,u}mulx: inputs in %rax, r. Outputs in %rdx(hi):%rax(lo)
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("mulx", f_binary, 1)
            .operands_in(vec![
                OperandConstraint::FixedReg(reg_rax),
                OperandConstraint::RegClass(gpr),
            ])
            .operands_out(vec![
                OperandConstraint::FixedReg(reg_rax),
                OperandConstraint::FixedReg(reg_rdx),
            ])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex1(in_reg1), sink);
                    modrm_r_bits(in_reg1, bits, sink);
                "#,
            ),
    );

    // XX /n ib with 8-bit immediate sign-extended.
    {
        let format = formats.get(f_binary_imm);
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("r_ib", f_binary_imm, 2)
                .operands_in(vec![gpr])
                .operands_out(vec![0])
                .inst_predicate(InstructionPredicate::new_is_signed_int(format, "imm", 8, 0))
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex1(in_reg0), sink);
                        modrm_r_bits(in_reg0, bits, sink);
                        let imm: i64 = imm.into();
                        sink.put1(imm as u8);
                    "#,
                ),
        );

        // XX /n id with 32-bit immediate sign-extended.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("r_id", f_binary_imm, 5)
                .operands_in(vec![gpr])
                .operands_out(vec![0])
                .inst_predicate(InstructionPredicate::new_is_signed_int(
                    format, "imm", 32, 0,
                ))
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex1(in_reg0), sink);
                        modrm_r_bits(in_reg0, bits, sink);
                        let imm: i64 = imm.into();
                        sink.put4(imm as u32);
                    "#,
                ),
        );
    }

    // XX /r ib with 8-bit unsigned immediate (e.g. for pshufd)
    {
        let format = formats.get(f_extract_lane);
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("r_ib_unsigned_fpr", f_extract_lane, 2)
                .operands_in(vec![fpr])
                .operands_out(vec![fpr])
                .inst_predicate(InstructionPredicate::new_is_unsigned_int(
                    format, "lane", 8, 0,
                )) // TODO if the format name is changed then "lane" should be renamed to something more appropriate--ordering mask? broadcast immediate?
                .emit(
                    r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                    modrm_rr(in_reg0, out_reg0, sink);
                    let imm:i64 = lane.into();
                    sink.put1(imm as u8);
                "#,
                ),
        );
    }

    // XX /r ib with 8-bit unsigned immediate (e.g. for extractlane)
    {
        let format = formats.get(f_extract_lane);
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("r_ib_unsigned_gpr", f_extract_lane, 2)
                .operands_in(vec![fpr])
                .operands_out(vec![gpr])
                .inst_predicate(InstructionPredicate::new_is_unsigned_int(
                    format, "lane", 8, 0,
                ))
                .emit(
                    r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                    modrm_rr(out_reg0, in_reg0, sink); // note the flipped register in the ModR/M byte
                    let imm:i64 = lane.into();
                    sink.put1(imm as u8);
                "#,
                ),
        );
    }

    // XX /r ib with 8-bit unsigned immediate (e.g. for insertlane)
    {
        let format = formats.get(f_insert_lane);
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("r_ib_unsigned_r", f_insert_lane, 2)
                .operands_in(vec![fpr, gpr])
                .operands_out(vec![0])
                .inst_predicate(InstructionPredicate::new_is_unsigned_int(
                    format, "lane", 8, 0,
                ))
                .emit(
                    r#"
                    {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                    modrm_rr(in_reg1, in_reg0, sink);
                    let imm:i64 = lane.into();
                    sink.put1(imm as u8);
                "#,
                ),
        );
    }

    {
        // XX /n id with 32-bit immediate sign-extended. UnaryImm version.
        let format = formats.get(f_unary_imm);
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("u_id", f_unary_imm, 5)
                .operands_out(vec![gpr])
                .inst_predicate(InstructionPredicate::new_is_signed_int(
                    format, "imm", 32, 0,
                ))
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex1(out_reg0), sink);
                        modrm_r_bits(out_reg0, bits, sink);
                        let imm: i64 = imm.into();
                        sink.put4(imm as u32);
                    "#,
                ),
        );
    }

    // XX+rd id unary with 32-bit immediate. Note no recipe predicate.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("pu_id", f_unary_imm, 4)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    // The destination register is encoded in the low bits of the opcode.
                    // No ModR/M.
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    let imm: i64 = imm.into();
                    sink.put4(imm as u32);
                "#,
            ),
    );

    // XX+rd id unary with bool immediate. Note no recipe predicate.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("pu_id_bool", f_unary_bool, 4)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    // The destination register is encoded in the low bits of the opcode.
                    // No ModR/M.
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    let imm: u32 = if imm { 1 } else { 0 };
                    sink.put4(imm);
                "#,
            ),
    );

    // XX+rd id nullary with 0 as 32-bit immediate. Note no recipe predicate.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("pu_id_ref", f_nullary, 4)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    // The destination register is encoded in the low bits of the opcode.
                    // No ModR/M.
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    sink.put4(0);
                "#,
            ),
    );

    // XX+rd iq unary with 64-bit immediate.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("pu_iq", f_unary_imm, 8)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    let imm: i64 = imm.into();
                    sink.put8(imm as u64);
                "#,
            ),
    );

    // XX /n Unary with floating point 32-bit immediate equal to zero.
    {
        let format = formats.get(f_unary_ieee32);
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("f32imm_z", f_unary_ieee32, 1)
                .operands_out(vec![fpr])
                .inst_predicate(InstructionPredicate::new_is_zero_32bit_float(format, "imm"))
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex2(out_reg0, out_reg0), sink);
                        modrm_rr(out_reg0, out_reg0, sink);
                    "#,
                ),
        );
    }

    // XX /n Unary with floating point 64-bit immediate equal to zero.
    {
        let format = formats.get(f_unary_ieee64);
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("f64imm_z", f_unary_ieee64, 1)
                .operands_out(vec![fpr])
                .inst_predicate(InstructionPredicate::new_is_zero_64bit_float(format, "imm"))
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex2(out_reg0, out_reg0), sink);
                        modrm_rr(out_reg0, out_reg0, sink);
                    "#,
                ),
        );
    }

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("pushq", f_unary, 0)
            .operands_in(vec![gpr])
            .emit(
                r#"
                    sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                    {{PUT_OP}}(bits | (in_reg0 & 7), rex1(in_reg0), sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("popq", f_nullary, 0)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                "#,
            ),
    );

    // XX /r, for regmove instructions.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("copysp", f_copy_special, 1)
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(dst, src), sink);
                    modrm_rr(dst, src, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("adjustsp", f_unary, 1)
            .operands_in(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(RU::rsp.into(), in_reg0), sink);
                    modrm_rr(RU::rsp.into(), in_reg0, sink);
                "#,
            ),
    );

    {
        let format = formats.get(f_unary_imm);
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("adjustsp_ib", f_unary_imm, 2)
                .inst_predicate(InstructionPredicate::new_is_signed_int(format, "imm", 8, 0))
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex1(RU::rsp.into()), sink);
                        modrm_r_bits(RU::rsp.into(), bits, sink);
                        let imm: i64 = imm.into();
                        sink.put1(imm as u8);
                    "#,
                ),
        );

        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("adjustsp_id", f_unary_imm, 5)
                .inst_predicate(InstructionPredicate::new_is_signed_int(
                    format, "imm", 32, 0,
                ))
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex1(RU::rsp.into()), sink);
                        modrm_r_bits(RU::rsp.into(), bits, sink);
                        let imm: i64 = imm.into();
                        sink.put4(imm as u32);
                    "#,
                ),
        );
    }

    // XX+rd id with Abs4 function relocation.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("fnaddr4", f_func_addr, 4)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    sink.reloc_external(Reloc::Abs4,
                                        &func.dfg.ext_funcs[func_ref].name,
                                        0);
                    sink.put4(0);
                "#,
            ),
    );

    // XX+rd iq with Abs8 function relocation.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("fnaddr8", f_func_addr, 8)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    sink.reloc_external(Reloc::Abs8,
                                        &func.dfg.ext_funcs[func_ref].name,
                                        0);
                    sink.put8(0);
                "#,
            ),
    );

    // Similar to fnaddr4, but writes !0 (this is used by BaldrMonkey).
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("allones_fnaddr4", f_func_addr, 4)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    sink.reloc_external(Reloc::Abs4,
                                        &func.dfg.ext_funcs[func_ref].name,
                                        0);
                    // Write the immediate as `!0` for the benefit of BaldrMonkey.
                    sink.put4(!0);
                "#,
            ),
    );

    // Similar to fnaddr8, but writes !0 (this is used by BaldrMonkey).
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("allones_fnaddr8", f_func_addr, 8)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    sink.reloc_external(Reloc::Abs8,
                                        &func.dfg.ext_funcs[func_ref].name,
                                        0);
                    // Write the immediate as `!0` for the benefit of BaldrMonkey.
                    sink.put8(!0);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("pcrel_fnaddr8", f_func_addr, 5)
            .operands_out(vec![gpr])
            // rex2 gets passed 0 for r/m register because the upper bit of
            // r/m doesn't get decoded when in rip-relative addressing mode.
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(0, out_reg0), sink);
                    modrm_riprel(out_reg0, sink);
                    // The addend adjusts for the difference between the end of the
                    // instruction and the beginning of the immediate field.
                    sink.reloc_external(Reloc::X86PCRel4,
                                        &func.dfg.ext_funcs[func_ref].name,
                                        -4);
                    sink.put4(0);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("got_fnaddr8", f_func_addr, 5)
            .operands_out(vec![gpr])
            // rex2 gets passed 0 for r/m register because the upper bit of
            // r/m doesn't get decoded when in rip-relative addressing mode.
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(0, out_reg0), sink);
                    modrm_riprel(out_reg0, sink);
                    // The addend adjusts for the difference between the end of the
                    // instruction and the beginning of the immediate field.
                    sink.reloc_external(Reloc::X86GOTPCRel4,
                                        &func.dfg.ext_funcs[func_ref].name,
                                        -4);
                    sink.put4(0);
                "#,
            ),
    );

    // XX+rd id with Abs4 globalsym relocation.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("gvaddr4", f_unary_global_value, 4)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    sink.reloc_external(Reloc::Abs4,
                                        &func.global_values[global_value].symbol_name(),
                                        0);
                    sink.put4(0);
                "#,
            ),
    );

    // XX+rd iq with Abs8 globalsym relocation.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("gvaddr8", f_unary_global_value, 8)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    sink.reloc_external(Reloc::Abs8,
                                        &func.global_values[global_value].symbol_name(),
                                        0);
                    sink.put8(0);
                "#,
            ),
    );

    // XX+rd iq with PCRel4 globalsym relocation.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("pcrel_gvaddr8", f_unary_global_value, 5)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(0, out_reg0), sink);
                    modrm_rm(5, out_reg0, sink);
                    // The addend adjusts for the difference between the end of the
                    // instruction and the beginning of the immediate field.
                    sink.reloc_external(Reloc::X86PCRel4,
                                        &func.global_values[global_value].symbol_name(),
                                        -4);
                    sink.put4(0);
                "#,
            ),
    );

    // XX+rd iq with Abs8 globalsym relocation.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("got_gvaddr8", f_unary_global_value, 5)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(0, out_reg0), sink);
                    modrm_rm(5, out_reg0, sink);
                    // The addend adjusts for the difference between the end of the
                    // instruction and the beginning of the immediate field.
                    sink.reloc_external(Reloc::X86GOTPCRel4,
                                        &func.global_values[global_value].symbol_name(),
                                        -4);
                    sink.put4(0);
                "#,
            ),
    );

    // Stack addresses.
    //
    // TODO Alternative forms for 8-bit immediates, when applicable.

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("spaddr4_id", f_stack_load, 6)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    let sp = StackRef::sp(stack_slot, &func.stack_slots);
                    let base = stk_base(sp.base);
                    {{PUT_OP}}(bits, rex2(out_reg0, base), sink);
                    modrm_sib_disp8(out_reg0, sink);
                    sib_noindex(base, sink);
                    let imm : i32 = offset.into();
                    sink.put4(sp.offset.checked_add(imm).unwrap() as u32);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("spaddr8_id", f_stack_load, 6)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    let sp = StackRef::sp(stack_slot, &func.stack_slots);
                    let base = stk_base(sp.base);
                    {{PUT_OP}}(bits, rex2(base, out_reg0), sink);
                    modrm_sib_disp32(out_reg0, sink);
                    sib_noindex(base, sink);
                    let imm : i32 = offset.into();
                    sink.put4(sp.offset.checked_add(imm).unwrap() as u32);
                "#,
            ),
    );

    // Store recipes.

    {
        // Simple stores.
        let format = formats.get(f_store);

        // A predicate asking if the offset is zero.
        let has_no_offset = InstructionPredicate::new_is_field_equal(format, "offset", "0".into());

        // XX /r register-indirect store with no offset.
        let st = recipes.add_template_recipe(
            EncodingRecipeBuilder::new("st", f_store, 1)
                .operands_in(vec![gpr, gpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_or_offset_for_in_reg_1")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                        if needs_sib_byte(in_reg1) {
                            modrm_sib(in_reg0, sink);
                            sib_noindex(in_reg1, sink);
                        } else if needs_offset(in_reg1) {
                            modrm_disp8(in_reg1, in_reg0, sink);
                            sink.put1(0);
                        } else {
                            modrm_rm(in_reg1, in_reg0, sink);
                        }
                    "#,
                ),
        );

        // XX /r register-indirect store with no offset.
        // Only ABCD allowed for stored value. This is for byte stores with no REX.
        recipes.add_template(
            Template::new(
                EncodingRecipeBuilder::new("st_abcd", f_store, 1)
                    .operands_in(vec![abcd, gpr])
                    .inst_predicate(has_no_offset.clone())
                    .clobbers_flags(false)
                    .compute_size("size_plus_maybe_sib_or_offset_for_in_reg_1")
                    .emit(
                        r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                        if needs_sib_byte(in_reg1) {
                            modrm_sib(in_reg0, sink);
                            sib_noindex(in_reg1, sink);
                        } else if needs_offset(in_reg1) {
                            modrm_disp8(in_reg1, in_reg0, sink);
                            sink.put1(0);
                        } else {
                            modrm_rm(in_reg1, in_reg0, sink);
                        }
                    "#,
                    ),
                formats,
                regs,
            )
            .when_prefixed(st),
        );

        // XX /r register-indirect store of FPR with no offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("fst", f_store, 1)
                .operands_in(vec![fpr, gpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_or_offset_for_in_reg_1")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                        if needs_sib_byte(in_reg1) {
                            modrm_sib(in_reg0, sink);
                            sib_noindex(in_reg1, sink);
                        } else if needs_offset(in_reg1) {
                            modrm_disp8(in_reg1, in_reg0, sink);
                            sink.put1(0);
                        } else {
                            modrm_rm(in_reg1, in_reg0, sink);
                        }
                    "#,
                ),
        );

        let has_small_offset = InstructionPredicate::new_is_signed_int(format, "offset", 8, 0);

        // XX /r register-indirect store with 8-bit offset.
        let st_disp8 = recipes.add_template_recipe(
            EncodingRecipeBuilder::new("stDisp8", f_store, 2)
                .operands_in(vec![gpr, gpr])
                .inst_predicate(has_small_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_in_reg_1")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                        if needs_sib_byte(in_reg1) {
                            modrm_sib_disp8(in_reg0, sink);
                            sib_noindex(in_reg1, sink);
                        } else {
                            modrm_disp8(in_reg1, in_reg0, sink);
                        }
                        let offset: i32 = offset.into();
                        sink.put1(offset as u8);
                    "#,
                ),
        );

        // XX /r register-indirect store with 8-bit offset.
        // Only ABCD allowed for stored value. This is for byte stores with no REX.
        recipes.add_template(
            Template::new(
                EncodingRecipeBuilder::new("stDisp8_abcd", f_store, 2)
                    .operands_in(vec![abcd, gpr])
                    .inst_predicate(has_small_offset.clone())
                    .clobbers_flags(false)
                    .compute_size("size_plus_maybe_sib_for_in_reg_1")
                    .emit(
                        r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                        if needs_sib_byte(in_reg1) {
                            modrm_sib_disp8(in_reg0, sink);
                            sib_noindex(in_reg1, sink);
                        } else {
                            modrm_disp8(in_reg1, in_reg0, sink);
                        }
                        let offset: i32 = offset.into();
                        sink.put1(offset as u8);
                    "#,
                    ),
                formats,
                regs,
            )
            .when_prefixed(st_disp8),
        );

        // XX /r register-indirect store with 8-bit offset of FPR.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("fstDisp8", f_store, 2)
                .operands_in(vec![fpr, gpr])
                .inst_predicate(has_small_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_in_reg_1")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                        if needs_sib_byte(in_reg1) {
                            modrm_sib_disp8(in_reg0, sink);
                            sib_noindex(in_reg1, sink);
                        } else {
                            modrm_disp8(in_reg1, in_reg0, sink);
                        }
                        let offset: i32 = offset.into();
                        sink.put1(offset as u8);
                    "#,
                ),
        );

        // XX /r register-indirect store with 32-bit offset.
        let st_disp32 = recipes.add_template_recipe(
            EncodingRecipeBuilder::new("stDisp32", f_store, 5)
                .operands_in(vec![gpr, gpr])
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_in_reg_1")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                        if needs_sib_byte(in_reg1) {
                            modrm_sib_disp32(in_reg0, sink);
                            sib_noindex(in_reg1, sink);
                        } else {
                            modrm_disp32(in_reg1, in_reg0, sink);
                        }
                        let offset: i32 = offset.into();
                        sink.put4(offset as u32);
                    "#,
                ),
        );

        // XX /r register-indirect store with 32-bit offset.
        // Only ABCD allowed for stored value. This is for byte stores with no REX.
        recipes.add_template(
            Template::new(
                EncodingRecipeBuilder::new("stDisp32_abcd", f_store, 5)
                    .operands_in(vec![abcd, gpr])
                    .clobbers_flags(false)
                    .compute_size("size_plus_maybe_sib_for_in_reg_1")
                    .emit(
                        r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                        if needs_sib_byte(in_reg1) {
                            modrm_sib_disp32(in_reg0, sink);
                            sib_noindex(in_reg1, sink);
                        } else {
                            modrm_disp32(in_reg1, in_reg0, sink);
                        }
                        let offset: i32 = offset.into();
                        sink.put4(offset as u32);
                    "#,
                    ),
                formats,
                regs,
            )
            .when_prefixed(st_disp32),
        );

        // XX /r register-indirect store with 32-bit offset of FPR.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("fstDisp32", f_store, 5)
                .operands_in(vec![fpr, gpr])
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_in_reg_1")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                        if needs_sib_byte(in_reg1) {
                            modrm_sib_disp32(in_reg0, sink);
                            sib_noindex(in_reg1, sink);
                        } else {
                            modrm_disp32(in_reg1, in_reg0, sink);
                        }
                        let offset: i32 = offset.into();
                        sink.put4(offset as u32);
                    "#,
                ),
        );
    }

    {
        // Complex stores.
        let format = formats.get(f_store_complex);

        // A predicate asking if the offset is zero.
        let has_no_offset = InstructionPredicate::new_is_field_equal(format, "offset", "0".into());

        // XX /r register-indirect store with index and no offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("stWithIndex", f_store_complex, 2)
                .operands_in(vec![gpr, gpr, gpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_offset_for_in_reg_1")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                        // The else branch always inserts an SIB byte.
                        if needs_offset(in_reg1) {
                            modrm_sib_disp8(in_reg0, sink);
                            sib(0, in_reg2, in_reg1, sink);
                            sink.put1(0);
                        } else {
                            modrm_sib(in_reg0, sink);
                            sib(0, in_reg2, in_reg1, sink);
                        }
                    "#,
                ),
        );

        // XX /r register-indirect store with index and no offset.
        // Only ABCD allowed for stored value. This is for byte stores with no REX.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("stWithIndex_abcd", f_store_complex, 2)
                .operands_in(vec![abcd, gpr, gpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_offset_for_in_reg_1")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                        // The else branch always inserts an SIB byte.
                        if needs_offset(in_reg1) {
                            modrm_sib_disp8(in_reg0, sink);
                            sib(0, in_reg2, in_reg1, sink);
                            sink.put1(0);
                        } else {
                            modrm_sib(in_reg0, sink);
                            sib(0, in_reg2, in_reg1, sink);
                        }
                    "#,
                ),
        );

        // XX /r register-indirect store with index and no offset of FPR.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("fstWithIndex", f_store_complex, 2)
                .operands_in(vec![fpr, gpr, gpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_offset_for_in_reg_1")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                        // The else branch always inserts an SIB byte.
                        if needs_offset(in_reg1) {
                            modrm_sib_disp8(in_reg0, sink);
                            sib(0, in_reg2, in_reg1, sink);
                            sink.put1(0);
                        } else {
                            modrm_sib(in_reg0, sink);
                            sib(0, in_reg2, in_reg1, sink);
                        }
                    "#,
                ),
        );

        let has_small_offset = InstructionPredicate::new_is_signed_int(format, "offset", 8, 0);

        // XX /r register-indirect store with index and 8-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("stWithIndexDisp8", f_store_complex, 3)
                .operands_in(vec![gpr, gpr, gpr])
                .inst_predicate(has_small_offset.clone())
                .clobbers_flags(false)
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                        modrm_sib_disp8(in_reg0, sink);
                        sib(0, in_reg2, in_reg1, sink);
                        let offset: i32 = offset.into();
                        sink.put1(offset as u8);
                    "#,
                ),
        );

        // XX /r register-indirect store with index and 8-bit offset.
        // Only ABCD allowed for stored value. This is for byte stores with no REX.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("stWithIndexDisp8_abcd", f_store_complex, 3)
                .operands_in(vec![abcd, gpr, gpr])
                .inst_predicate(has_small_offset.clone())
                .clobbers_flags(false)
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                        modrm_sib_disp8(in_reg0, sink);
                        sib(0, in_reg2, in_reg1, sink);
                        let offset: i32 = offset.into();
                        sink.put1(offset as u8);
                    "#,
                ),
        );

        // XX /r register-indirect store with index and 8-bit offset of FPR.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("fstWithIndexDisp8", f_store_complex, 3)
                .operands_in(vec![fpr, gpr, gpr])
                .inst_predicate(has_small_offset.clone())
                .clobbers_flags(false)
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                        modrm_sib_disp8(in_reg0, sink);
                        sib(0, in_reg2, in_reg1, sink);
                        let offset: i32 = offset.into();
                        sink.put1(offset as u8);
                    "#,
                ),
        );

        let has_big_offset = InstructionPredicate::new_is_signed_int(format, "offset", 32, 0);

        // XX /r register-indirect store with index and 32-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("stWithIndexDisp32", f_store_complex, 6)
                .operands_in(vec![gpr, gpr, gpr])
                .inst_predicate(has_big_offset.clone())
                .clobbers_flags(false)
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                        modrm_sib_disp32(in_reg0, sink);
                        sib(0, in_reg2, in_reg1, sink);
                        let offset: i32 = offset.into();
                        sink.put4(offset as u32);
                    "#,
                ),
        );

        // XX /r register-indirect store with index and 32-bit offset.
        // Only ABCD allowed for stored value. This is for byte stores with no REX.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("stWithIndexDisp32_abcd", f_store_complex, 6)
                .operands_in(vec![abcd, gpr, gpr])
                .inst_predicate(has_big_offset.clone())
                .clobbers_flags(false)
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                        modrm_sib_disp32(in_reg0, sink);
                        sib(0, in_reg2, in_reg1, sink);
                        let offset: i32 = offset.into();
                        sink.put4(offset as u32);
                    "#,
                ),
        );

        // XX /r register-indirect store with index and 32-bit offset of FPR.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("fstWithIndexDisp32", f_store_complex, 6)
                .operands_in(vec![fpr, gpr, gpr])
                .inst_predicate(has_big_offset.clone())
                .clobbers_flags(false)
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                        modrm_sib_disp32(in_reg0, sink);
                        sib(0, in_reg2, in_reg1, sink);
                        let offset: i32 = offset.into();
                        sink.put4(offset as u32);
                    "#,
                ),
        );
    }

    // Unary spill with SIB and 32-bit displacement.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("spillSib32", f_unary, 6)
            .operands_in(vec![gpr])
            .operands_out(vec![stack_gpr32])
            .clobbers_flags(false)
            .emit(
                r#"
                    sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                    let base = stk_base(out_stk0.base);
                    {{PUT_OP}}(bits, rex2(base, in_reg0), sink);
                    modrm_sib_disp32(in_reg0, sink);
                    sib_noindex(base, sink);
                    sink.put4(out_stk0.offset as u32);
                "#,
            ),
    );

    // Like spillSib32, but targeting an FPR rather than a GPR.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("fspillSib32", f_unary, 6)
            .operands_in(vec![fpr])
            .operands_out(vec![stack_fpr32])
            .clobbers_flags(false)
            .emit(
                r#"
                    sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                    let base = stk_base(out_stk0.base);
                    {{PUT_OP}}(bits, rex2(base, in_reg0), sink);
                    modrm_sib_disp32(in_reg0, sink);
                    sib_noindex(base, sink);
                    sink.put4(out_stk0.offset as u32);
                "#,
            ),
    );

    // Regspill using RSP-relative addressing.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("regspill32", f_reg_spill, 6)
            .operands_in(vec![gpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                    let dst = StackRef::sp(dst, &func.stack_slots);
                    let base = stk_base(dst.base);
                    {{PUT_OP}}(bits, rex2(base, src), sink);
                    modrm_sib_disp32(src, sink);
                    sib_noindex(base, sink);
                    sink.put4(dst.offset as u32);
                "#,
            ),
    );

    // Like regspill32, but targeting an FPR rather than a GPR.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("fregspill32", f_reg_spill, 6)
            .operands_in(vec![fpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                    let dst = StackRef::sp(dst, &func.stack_slots);
                    let base = stk_base(dst.base);
                    {{PUT_OP}}(bits, rex2(base, src), sink);
                    modrm_sib_disp32(src, sink);
                    sib_noindex(base, sink);
                    sink.put4(dst.offset as u32);
                "#,
            ),
    );

    // Load recipes.

    {
        // Simple loads.
        let format = formats.get(f_load);

        // A predicate asking if the offset is zero.
        let has_no_offset = InstructionPredicate::new_is_field_equal(format, "offset", "0".into());

        // XX /r load with no offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("ld", f_load, 1)
                .operands_in(vec![gpr])
                .operands_out(vec![gpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_or_offset_for_in_reg_0")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                        if needs_sib_byte(in_reg0) {
                            modrm_sib(out_reg0, sink);
                            sib_noindex(in_reg0, sink);
                        } else if needs_offset(in_reg0) {
                            modrm_disp8(in_reg0, out_reg0, sink);
                            sink.put1(0);
                        } else {
                            modrm_rm(in_reg0, out_reg0, sink);
                        }
                    "#,
                ),
        );

        // XX /r float load with no offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("fld", f_load, 1)
                .operands_in(vec![gpr])
                .operands_out(vec![fpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_or_offset_for_in_reg_0")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                        if needs_sib_byte(in_reg0) {
                            modrm_sib(out_reg0, sink);
                            sib_noindex(in_reg0, sink);
                        } else if needs_offset(in_reg0) {
                            modrm_disp8(in_reg0, out_reg0, sink);
                            sink.put1(0);
                        } else {
                            modrm_rm(in_reg0, out_reg0, sink);
                        }
                    "#,
                ),
        );

        let has_small_offset = InstructionPredicate::new_is_signed_int(format, "offset", 8, 0);

        // XX /r load with 8-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("ldDisp8", f_load, 2)
                .operands_in(vec![gpr])
                .operands_out(vec![gpr])
                .inst_predicate(has_small_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_in_reg_0")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                        if needs_sib_byte(in_reg0) {
                            modrm_sib_disp8(out_reg0, sink);
                            sib_noindex(in_reg0, sink);
                        } else {
                            modrm_disp8(in_reg0, out_reg0, sink);
                        }
                        let offset: i32 = offset.into();
                        sink.put1(offset as u8);
                    "#,
                ),
        );

        // XX /r float load with 8-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("fldDisp8", f_load, 2)
                .operands_in(vec![gpr])
                .operands_out(vec![fpr])
                .inst_predicate(has_small_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_in_reg_0")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                        if needs_sib_byte(in_reg0) {
                            modrm_sib_disp8(out_reg0, sink);
                            sib_noindex(in_reg0, sink);
                        } else {
                            modrm_disp8(in_reg0, out_reg0, sink);
                        }
                        let offset: i32 = offset.into();
                        sink.put1(offset as u8);
                    "#,
                ),
        );

        let has_big_offset = InstructionPredicate::new_is_signed_int(format, "offset", 32, 0);

        // XX /r load with 32-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("ldDisp32", f_load, 5)
                .operands_in(vec![gpr])
                .operands_out(vec![gpr])
                .inst_predicate(has_big_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_in_reg_0")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                        if needs_sib_byte(in_reg0) {
                            modrm_sib_disp32(out_reg0, sink);
                            sib_noindex(in_reg0, sink);
                        } else {
                            modrm_disp32(in_reg0, out_reg0, sink);
                        }
                        let offset: i32 = offset.into();
                        sink.put4(offset as u32);
                    "#,
                ),
        );

        // XX /r float load with 32-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("fldDisp32", f_load, 5)
                .operands_in(vec![gpr])
                .operands_out(vec![fpr])
                .inst_predicate(has_big_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_in_reg_0")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                        if needs_sib_byte(in_reg0) {
                            modrm_sib_disp32(out_reg0, sink);
                            sib_noindex(in_reg0, sink);
                        } else {
                            modrm_disp32(in_reg0, out_reg0, sink);
                        }
                        let offset: i32 = offset.into();
                        sink.put4(offset as u32);
                    "#,
                ),
        );
    }

    {
        // Complex loads.
        let format = formats.get(f_load_complex);

        // A predicate asking if the offset is zero.
        let has_no_offset = InstructionPredicate::new_is_field_equal(format, "offset", "0".into());

        // XX /r load with index and no offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("ldWithIndex", f_load_complex, 2)
                .operands_in(vec![gpr, gpr])
                .operands_out(vec![gpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_offset_for_in_reg_0")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                        // The else branch always inserts an SIB byte.
                        if needs_offset(in_reg0) {
                            modrm_sib_disp8(out_reg0, sink);
                            sib(0, in_reg1, in_reg0, sink);
                            sink.put1(0);
                        } else {
                            modrm_sib(out_reg0, sink);
                            sib(0, in_reg1, in_reg0, sink);
                        }
                    "#,
                ),
        );

        // XX /r float load with index and no offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("fldWithIndex", f_load_complex, 2)
                .operands_in(vec![gpr, gpr])
                .operands_out(vec![fpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_offset_for_in_reg_0")
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                        // The else branch always inserts an SIB byte.
                        if needs_offset(in_reg0) {
                            modrm_sib_disp8(out_reg0, sink);
                            sib(0, in_reg1, in_reg0, sink);
                            sink.put1(0);
                        } else {
                            modrm_sib(out_reg0, sink);
                            sib(0, in_reg1, in_reg0, sink);
                        }
                    "#,
                ),
        );

        let has_small_offset = InstructionPredicate::new_is_signed_int(format, "offset", 8, 0);

        // XX /r load with index and 8-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("ldWithIndexDisp8", f_load_complex, 3)
                .operands_in(vec![gpr, gpr])
                .operands_out(vec![gpr])
                .inst_predicate(has_small_offset.clone())
                .clobbers_flags(false)
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                        modrm_sib_disp8(out_reg0, sink);
                        sib(0, in_reg1, in_reg0, sink);
                        let offset: i32 = offset.into();
                        sink.put1(offset as u8);
                    "#,
                ),
        );

        // XX /r float load with 8-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("fldWithIndexDisp8", f_load_complex, 3)
                .operands_in(vec![gpr, gpr])
                .operands_out(vec![fpr])
                .inst_predicate(has_small_offset.clone())
                .clobbers_flags(false)
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                        modrm_sib_disp8(out_reg0, sink);
                        sib(0, in_reg1, in_reg0, sink);
                        let offset: i32 = offset.into();
                        sink.put1(offset as u8);
                    "#,
                ),
        );

        let has_big_offset = InstructionPredicate::new_is_signed_int(format, "offset", 32, 0);

        // XX /r load with index and 32-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("ldWithIndexDisp32", f_load_complex, 6)
                .operands_in(vec![gpr, gpr])
                .operands_out(vec![gpr])
                .inst_predicate(has_big_offset.clone())
                .clobbers_flags(false)
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                        modrm_sib_disp32(out_reg0, sink);
                        sib(0, in_reg1, in_reg0, sink);
                        let offset: i32 = offset.into();
                        sink.put4(offset as u32);
                    "#,
                ),
        );

        // XX /r float load with index and 32-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("fldWithIndexDisp32", f_load_complex, 6)
                .operands_in(vec![gpr, gpr])
                .operands_out(vec![fpr])
                .inst_predicate(has_big_offset.clone())
                .clobbers_flags(false)
                .emit(
                    r#"
                        if !flags.notrap() {
                            sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                        }
                        {{PUT_OP}}(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                        modrm_sib_disp32(out_reg0, sink);
                        sib(0, in_reg1, in_reg0, sink);
                        let offset: i32 = offset.into();
                        sink.put4(offset as u32);
                    "#,
                ),
        );
    }

    // Unary fill with SIB and 32-bit displacement.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("fillSib32", f_unary, 6)
            .operands_in(vec![stack_gpr32])
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    let base = stk_base(in_stk0.base);
                    {{PUT_OP}}(bits, rex2(base, out_reg0), sink);
                    modrm_sib_disp32(out_reg0, sink);
                    sib_noindex(base, sink);
                    sink.put4(in_stk0.offset as u32);
                "#,
            ),
    );

    // Like fillSib32, but targeting an FPR rather than a GPR.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("ffillSib32", f_unary, 6)
            .operands_in(vec![stack_fpr32])
            .operands_out(vec![fpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    let base = stk_base(in_stk0.base);
                    {{PUT_OP}}(bits, rex2(base, out_reg0), sink);
                    modrm_sib_disp32(out_reg0, sink);
                    sib_noindex(base, sink);
                    sink.put4(in_stk0.offset as u32);
                "#,
            ),
    );

    // Regfill with RSP-relative 32-bit displacement.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("regfill32", f_reg_fill, 6)
            .operands_in(vec![stack_gpr32])
            .clobbers_flags(false)
            .emit(
                r#"
                    let src = StackRef::sp(src, &func.stack_slots);
                    let base = stk_base(src.base);
                    {{PUT_OP}}(bits, rex2(base, dst), sink);
                    modrm_sib_disp32(dst, sink);
                    sib_noindex(base, sink);
                    sink.put4(src.offset as u32);
                "#,
            ),
    );

    // Like regfill32, but targeting an FPR rather than a GPR.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("fregfill32", f_reg_fill, 6)
            .operands_in(vec![stack_fpr32])
            .clobbers_flags(false)
            .emit(
                r#"
                    let src = StackRef::sp(src, &func.stack_slots);
                    let base = stk_base(src.base);
                    {{PUT_OP}}(bits, rex2(base, dst), sink);
                    modrm_sib_disp32(dst, sink);
                    sib_noindex(base, sink);
                    sink.put4(src.offset as u32);
                "#,
            ),
    );

    // Call/return.

    recipes.add_template_recipe(EncodingRecipeBuilder::new("call_id", f_call, 4).emit(
        r#"
            sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
            {{PUT_OP}}(bits, BASE_REX, sink);
            // The addend adjusts for the difference between the end of the
            // instruction and the beginning of the immediate field.
            sink.reloc_external(Reloc::X86CallPCRel4,
                                &func.dfg.ext_funcs[func_ref].name,
                                -4);
            sink.put4(0);
        "#,
    ));

    recipes.add_template_recipe(EncodingRecipeBuilder::new("call_plt_id", f_call, 4).emit(
        r#"
            sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
            {{PUT_OP}}(bits, BASE_REX, sink);
            sink.reloc_external(Reloc::X86CallPLTRel4,
                                &func.dfg.ext_funcs[func_ref].name,
                                -4);
            sink.put4(0);
        "#,
    ));

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("call_r", f_call_indirect, 1)
            .operands_in(vec![gpr])
            .emit(
                r#"
                    sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                    {{PUT_OP}}(bits, rex1(in_reg0), sink);
                    modrm_r_bits(in_reg0, bits, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("ret", f_multiary, 0).emit("{{PUT_OP}}(bits, BASE_REX, sink);"),
    );

    // Branches.

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("jmpb", f_jump, 1)
            .branch_range((1, 8))
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, BASE_REX, sink);
                    disp1(destination, func, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("jmpd", f_jump, 4)
            .branch_range((4, 32))
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, BASE_REX, sink);
                    disp4(destination, func, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("brib", f_branch_int, 1)
            .operands_in(vec![reg_rflags])
            .branch_range((1, 8))
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits | icc2opc(cond), BASE_REX, sink);
                    disp1(destination, func, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("brid", f_branch_int, 4)
            .operands_in(vec![reg_rflags])
            .branch_range((4, 32))
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits | icc2opc(cond), BASE_REX, sink);
                    disp4(destination, func, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("brfb", f_branch_float, 1)
            .operands_in(vec![reg_rflags])
            .branch_range((1, 8))
            .clobbers_flags(false)
            .inst_predicate(supported_floatccs_predicate(
                &supported_floatccs,
                formats.get(f_branch_float),
            ))
            .emit(
                r#"
                    {{PUT_OP}}(bits | fcc2opc(cond), BASE_REX, sink);
                    disp1(destination, func, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("brfd", f_branch_float, 4)
            .operands_in(vec![reg_rflags])
            .branch_range((4, 32))
            .clobbers_flags(false)
            .inst_predicate(supported_floatccs_predicate(
                &supported_floatccs,
                formats.get(f_branch_float),
            ))
            .emit(
                r#"
                    {{PUT_OP}}(bits | fcc2opc(cond), BASE_REX, sink);
                    disp4(destination, func, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("indirect_jmp", f_indirect_jump, 1)
            .operands_in(vec![gpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex1(in_reg0), sink);
                    modrm_r_bits(in_reg0, bits, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("jt_entry", f_branch_table_entry, 2)
            .operands_in(vec![gpr, gpr])
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .inst_predicate(valid_scale(formats.get(f_branch_table_entry)))
            .compute_size("size_plus_maybe_offset_for_in_reg_1")
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex3(in_reg1, out_reg0, in_reg0), sink);
                    if needs_offset(in_reg1) {
                        modrm_sib_disp8(out_reg0, sink);
                        sib(imm.trailing_zeros() as u8, in_reg0, in_reg1, sink);
                        sink.put1(0);
                    } else {
                        modrm_sib(out_reg0, sink);
                        sib(imm.trailing_zeros() as u8, in_reg0, in_reg1, sink);
                    }
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("vconst", f_unary_imm128, 5)
            .operands_out(vec![fpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(0, out_reg0), sink);
                    modrm_riprel(out_reg0, sink);
                    const_disp4(imm, func, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("jt_base", f_branch_table_base, 5)
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(0, out_reg0), sink);
                    modrm_riprel(out_reg0, sink);

                    // No reloc is needed here as the jump table is emitted directly after
                    // the function body.
                    jt_disp4(table, func, sink);
                "#,
            ),
    );

    // Test flags and set a register.
    //
    // These setCC instructions only set the low 8 bits, and they can only write ABCD registers
    // without a REX prefix.
    //
    // Other instruction encodings accepting `b1` inputs have the same constraints and only look at
    // the low 8 bits of the input register.

    let seti = recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("seti", f_int_cond, 1)
                .operands_in(vec![reg_rflags])
                .operands_out(vec![gpr])
                .clobbers_flags(false)
                .emit(
                    r#"
                    {{PUT_OP}}(bits | icc2opc(cond), rex1(out_reg0), sink);
                    modrm_r_bits(out_reg0, bits, sink);
                "#,
                ),
            formats,
            regs,
        )
        .requires_prefix(true),
    );

    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("seti_abcd", f_int_cond, 1)
                .operands_in(vec![reg_rflags])
                .operands_out(vec![abcd])
                .clobbers_flags(false)
                .emit(
                    r#"
                    {{PUT_OP}}(bits | icc2opc(cond), rex1(out_reg0), sink);
                    modrm_r_bits(out_reg0, bits, sink);
                "#,
                ),
            formats,
            regs,
        )
        .when_prefixed(seti),
    );

    let setf = recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("setf", f_float_cond, 1)
                .operands_in(vec![reg_rflags])
                .operands_out(vec![gpr])
                .clobbers_flags(false)
                .emit(
                    r#"
                    {{PUT_OP}}(bits | fcc2opc(cond), rex1(out_reg0), sink);
                    modrm_r_bits(out_reg0, bits, sink);
                "#,
                ),
            formats,
            regs,
        )
        .requires_prefix(true),
    );

    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("setf_abcd", f_float_cond, 1)
                .operands_in(vec![reg_rflags])
                .operands_out(vec![abcd])
                .clobbers_flags(false)
                .emit(
                    r#"
                    {{PUT_OP}}(bits | fcc2opc(cond), rex1(out_reg0), sink);
                    modrm_r_bits(out_reg0, bits, sink);
                "#,
                ),
            formats,
            regs,
        )
        .when_prefixed(setf),
    );

    // Conditional move (a.k.a integer select)
    // (maybe-REX.W) 0F 4x modrm(r,r)
    // 1 byte, modrm(r,r), is after the opcode
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("cmov", f_int_select, 1)
            .operands_in(vec![
                OperandConstraint::FixedReg(reg_rflags),
                OperandConstraint::RegClass(gpr),
                OperandConstraint::RegClass(gpr),
            ])
            .operands_out(vec![2])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits | icc2opc(cond), rex2(in_reg1, in_reg2), sink);
                    modrm_rr(in_reg1, in_reg2, sink);
                "#,
            ),
    );

    // Bit scan forwards and reverse
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("bsf_and_bsr", f_unary, 1)
            .operands_in(vec![gpr])
            .operands_out(vec![
                OperandConstraint::RegClass(gpr),
                OperandConstraint::FixedReg(reg_rflags),
            ])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                    modrm_rr(in_reg0, out_reg0, sink);
                "#,
            ),
    );

    // Arithematic with flag I/O.

    // XX /r, MR form. Add two GPR registers and set carry flag.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rout", f_binary, 1)
            .operands_in(vec![gpr, gpr])
            .operands_out(vec![
                OperandConstraint::TiedInput(0),
                OperandConstraint::FixedReg(reg_rflags),
            ])
            .clobbers_flags(true)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, in_reg1), sink);
                    modrm_rr(in_reg0, in_reg1, sink);
                "#,
            ),
    );

    // XX /r, MR form. Add two GPR registers and get carry flag.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rin", f_ternary, 1)
            .operands_in(vec![
                OperandConstraint::RegClass(gpr),
                OperandConstraint::RegClass(gpr),
                OperandConstraint::FixedReg(reg_rflags),
            ])
            .operands_out(vec![0])
            .clobbers_flags(true)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, in_reg1), sink);
                    modrm_rr(in_reg0, in_reg1, sink);
                "#,
            ),
    );

    // XX /r, MR form. Add two GPR registers with carry flag.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rio", f_ternary, 1)
            .operands_in(vec![
                OperandConstraint::RegClass(gpr),
                OperandConstraint::RegClass(gpr),
                OperandConstraint::FixedReg(reg_rflags),
            ])
            .operands_out(vec![
                OperandConstraint::TiedInput(0),
                OperandConstraint::FixedReg(reg_rflags),
            ])
            .clobbers_flags(true)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, in_reg1), sink);
                    modrm_rr(in_reg0, in_reg1, sink);
                "#,
            ),
    );

    // Compare and set flags.

    // XX /r, MR form. Compare two GPR registers and set flags.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rcmp", f_binary, 1)
            .operands_in(vec![gpr, gpr])
            .operands_out(vec![reg_rflags])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, in_reg1), sink);
                    modrm_rr(in_reg0, in_reg1, sink);
                "#,
            ),
    );

    // Same as rcmp, but second operand is the stack pointer.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rcmp_sp", f_unary, 1)
            .operands_in(vec![gpr])
            .operands_out(vec![reg_rflags])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, RU::rsp.into()), sink);
                    modrm_rr(in_reg0, RU::rsp.into(), sink);
                "#,
            ),
    );

    // XX /r, RM form. Compare two FPR registers and set flags.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("fcmp", f_binary, 1)
            .operands_in(vec![fpr, fpr])
            .operands_out(vec![reg_rflags])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                    modrm_rr(in_reg1, in_reg0, sink);
                "#,
            ),
    );

    {
        let format = formats.get(f_binary_imm);

        let has_small_offset = InstructionPredicate::new_is_signed_int(format, "imm", 8, 0);

        // XX /n, MI form with imm8.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("rcmp_ib", f_binary_imm, 2)
                .operands_in(vec![gpr])
                .operands_out(vec![reg_rflags])
                .inst_predicate(has_small_offset)
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex1(in_reg0), sink);
                        modrm_r_bits(in_reg0, bits, sink);
                        let imm: i64 = imm.into();
                        sink.put1(imm as u8);
                    "#,
                ),
        );

        let has_big_offset = InstructionPredicate::new_is_signed_int(format, "imm", 32, 0);

        // XX /n, MI form with imm32.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("rcmp_id", f_binary_imm, 5)
                .operands_in(vec![gpr])
                .operands_out(vec![reg_rflags])
                .inst_predicate(has_big_offset)
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex1(in_reg0), sink);
                        modrm_r_bits(in_reg0, bits, sink);
                        let imm: i64 = imm.into();
                        sink.put4(imm as u32);
                    "#,
                ),
        );
    }

    // Test-and-branch.
    //
    // This recipe represents the macro fusion of a test and a conditional branch.
    // This serves two purposes:
    //
    // 1. Guarantee that the test and branch get scheduled next to each other so
    //    macro fusion is guaranteed to be possible.
    // 2. Hide the status flags from Cranelift which doesn't currently model flags.
    //
    // The encoding bits affect both the test and the branch instruction:
    //
    // Bits 0-7 are the Jcc opcode.
    // Bits 8-15 control the test instruction which always has opcode byte 0x85.

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("tjccb", f_branch, 1 + 2)
            .operands_in(vec![gpr])
            .branch_range((3, 8))
            .emit(
                r#"
                    // test r, r.
                    {{PUT_OP}}((bits & 0xff00) | 0x85, rex2(in_reg0, in_reg0), sink);
                    modrm_rr(in_reg0, in_reg0, sink);
                    // Jcc instruction.
                    sink.put1(bits as u8);
                    disp1(destination, func, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("tjccd", f_branch, 1 + 6)
            .operands_in(vec![gpr])
            .branch_range((7, 32))
            .emit(
                r#"
                    // test r, r.
                    {{PUT_OP}}((bits & 0xff00) | 0x85, rex2(in_reg0, in_reg0), sink);
                    modrm_rr(in_reg0, in_reg0, sink);
                    // Jcc instruction.
                    sink.put1(0x0f);
                    sink.put1(bits as u8);
                    disp4(destination, func, sink);
                "#,
            ),
    );

    // 8-bit test-and-branch.

    let t8jccb = recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("t8jccb", f_branch, 1 + 2)
                .operands_in(vec![gpr])
                .branch_range((3, 8))
                .emit(
                    r#"
                    // test8 r, r.
                    {{PUT_OP}}((bits & 0xff00) | 0x84, rex2(in_reg0, in_reg0), sink);
                    modrm_rr(in_reg0, in_reg0, sink);
                    // Jcc instruction.
                    sink.put1(bits as u8);
                    disp1(destination, func, sink);
                "#,
                ),
            formats,
            regs,
        )
        .requires_prefix(true),
    );

    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("t8jccb_abcd", f_branch, 1 + 2)
                .operands_in(vec![abcd])
                .branch_range((3, 8))
                .emit(
                    r#"
                    // test8 r, r.
                    {{PUT_OP}}((bits & 0xff00) | 0x84, rex2(in_reg0, in_reg0), sink);
                    modrm_rr(in_reg0, in_reg0, sink);
                    // Jcc instruction.
                    sink.put1(bits as u8);
                    disp1(destination, func, sink);
                "#,
                ),
            formats,
            regs,
        )
        .when_prefixed(t8jccb),
    );

    let t8jccd = recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("t8jccd", f_branch, 1 + 6)
                .operands_in(vec![gpr])
                .branch_range((7, 32))
                .emit(
                    r#"
                    // test8 r, r.
                    {{PUT_OP}}((bits & 0xff00) | 0x84, rex2(in_reg0, in_reg0), sink);
                    modrm_rr(in_reg0, in_reg0, sink);
                    // Jcc instruction.
                    sink.put1(0x0f);
                    sink.put1(bits as u8);
                    disp4(destination, func, sink);
                "#,
                ),
            formats,
            regs,
        )
        .requires_prefix(true),
    );

    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("t8jccd_abcd", f_branch, 1 + 6)
                .operands_in(vec![abcd])
                .branch_range((7, 32))
                .emit(
                    r#"
                    // test8 r, r.
                    {{PUT_OP}}((bits & 0xff00) | 0x84, rex2(in_reg0, in_reg0), sink);
                    modrm_rr(in_reg0, in_reg0, sink);
                    // Jcc instruction.
                    sink.put1(0x0f);
                    sink.put1(bits as u8);
                    disp4(destination, func, sink);
                "#,
                ),
            formats,
            regs,
        )
        .when_prefixed(t8jccd),
    );

    // Worst case test-and-branch recipe for brz.b1 and brnz.b1 in 32-bit mode.
    // The register allocator can't handle a branch instruction with constrained
    // operands like the t8jccd_abcd above. This variant can accept the b1 opernd in
    // any register, but is is larger because it uses a 32-bit test instruction with
    // a 0xff immediate.

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("t8jccd_long", f_branch, 5 + 6)
            .operands_in(vec![gpr])
            .branch_range((11, 32))
            .emit(
                r#"
                    // test32 r, 0xff.
                    {{PUT_OP}}((bits & 0xff00) | 0xf7, rex1(in_reg0), sink);
                    modrm_r_bits(in_reg0, bits, sink);
                    sink.put4(0xff);
                    // Jcc instruction.
                    sink.put1(0x0f);
                    sink.put1(bits as u8);
                    disp4(destination, func, sink);
                "#,
            ),
    );

    // Comparison that produces a `b1` result in a GPR.
    //
    // This is a macro of a `cmp` instruction followed by a `setCC` instruction.
    //
    // TODO This is not a great solution because:
    //
    // - The cmp+setcc combination is not recognized by CPU's macro fusion.
    // - The 64-bit encoding has issues with REX prefixes. The `cmp` and `setCC`
    //   instructions may need a REX independently.
    // - Modeling CPU flags in the type system would be better.
    //
    // Since the `setCC` instructions only write an 8-bit register, we use that as
    // our `b1` representation: A `b1` value is represented as a GPR where the low 8
    // bits are known to be 0 or 1. The high bits are undefined.
    //
    // This bandaid macro doesn't support a REX prefix for the final `setCC`
    // instruction, so it is limited to the `ABCD` register class for booleans.
    // The omission of a `when_prefixed` alternative is deliberate here.

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("icscc", f_int_compare, 1 + 3)
            .operands_in(vec![gpr, gpr])
            .operands_out(vec![abcd])
            .emit(
                r#"
                    // Comparison instruction.
                    {{PUT_OP}}(bits, rex2(in_reg0, in_reg1), sink);
                    modrm_rr(in_reg0, in_reg1, sink);
                    // `setCC` instruction, no REX.
                    use crate::ir::condcodes::IntCC::*;
                    let setcc = match cond {
                        Equal => 0x94,
                        NotEqual => 0x95,
                        SignedLessThan => 0x9c,
                        SignedGreaterThanOrEqual => 0x9d,
                        SignedGreaterThan => 0x9f,
                        SignedLessThanOrEqual => 0x9e,
                        UnsignedLessThan => 0x92,
                        UnsignedGreaterThanOrEqual => 0x93,
                        UnsignedGreaterThan => 0x97,
                        UnsignedLessThanOrEqual => 0x96,
                    };
                    sink.put1(0x0f);
                    sink.put1(setcc);
                    modrm_rr(out_reg0, 0, sink);
                "#,
            ),
    );

    {
        let format = formats.get(f_int_compare_imm);

        let is_small_imm = InstructionPredicate::new_is_signed_int(format, "imm", 8, 0);

        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("icscc_ib", f_int_compare_imm, 2 + 3)
                .operands_in(vec![gpr])
                .operands_out(vec![abcd])
                .inst_predicate(is_small_imm)
                .emit(
                    r#"
                        // Comparison instruction.
                        {{PUT_OP}}(bits, rex1(in_reg0), sink);
                        modrm_r_bits(in_reg0, bits, sink);
                        let imm: i64 = imm.into();
                        sink.put1(imm as u8);
                        // `setCC` instruction, no REX.
                        use crate::ir::condcodes::IntCC::*;
                        let setcc = match cond {
                            Equal => 0x94,
                            NotEqual => 0x95,
                            SignedLessThan => 0x9c,
                            SignedGreaterThanOrEqual => 0x9d,
                            SignedGreaterThan => 0x9f,
                            SignedLessThanOrEqual => 0x9e,
                            UnsignedLessThan => 0x92,
                            UnsignedGreaterThanOrEqual => 0x93,
                            UnsignedGreaterThan => 0x97,
                            UnsignedLessThanOrEqual => 0x96,
                        };
                        sink.put1(0x0f);
                        sink.put1(setcc);
                        modrm_rr(out_reg0, 0, sink);
                    "#,
                ),
        );

        let is_big_imm = InstructionPredicate::new_is_signed_int(format, "imm", 32, 0);

        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("icscc_id", f_int_compare_imm, 5 + 3)
                .operands_in(vec![gpr])
                .operands_out(vec![abcd])
                .inst_predicate(is_big_imm)
                .emit(
                    r#"
                        // Comparison instruction.
                        {{PUT_OP}}(bits, rex1(in_reg0), sink);
                        modrm_r_bits(in_reg0, bits, sink);
                        let imm: i64 = imm.into();
                        sink.put4(imm as u32);
                        // `setCC` instruction, no REX.
                        use crate::ir::condcodes::IntCC::*;
                        let setcc = match cond {
                            Equal => 0x94,
                            NotEqual => 0x95,
                            SignedLessThan => 0x9c,
                            SignedGreaterThanOrEqual => 0x9d,
                            SignedGreaterThan => 0x9f,
                            SignedLessThanOrEqual => 0x9e,
                            UnsignedLessThan => 0x92,
                            UnsignedGreaterThanOrEqual => 0x93,
                            UnsignedGreaterThan => 0x97,
                            UnsignedLessThanOrEqual => 0x96,
                        };
                        sink.put1(0x0f);
                        sink.put1(setcc);
                        modrm_rr(out_reg0, 0, sink);
                    "#,
                ),
        );
    }

    // Make a FloatCompare instruction predicate with the supported condition codes.
    //
    // Same thing for floating point.
    //
    // The ucomiss/ucomisd instructions set the FLAGS bits CF/PF/CF like this:
    //
    //    ZPC OSA
    // UN 111 000
    // GT 000 000
    // LT 001 000
    // EQ 100 000
    //
    // Not all floating point condition codes are supported.
    // The omission of a `when_prefixed` alternative is deliberate here.

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("fcscc", f_float_compare, 1 + 3)
            .operands_in(vec![fpr, fpr])
            .operands_out(vec![abcd])
            .inst_predicate(supported_floatccs_predicate(
                &supported_floatccs,
                formats.get(f_float_compare),
            ))
            .emit(
                r#"
                    // Comparison instruction.
                    {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                    modrm_rr(in_reg1, in_reg0, sink);
                    // `setCC` instruction, no REX.
                    use crate::ir::condcodes::FloatCC::*;
                    let setcc = match cond {
                        Ordered                    => 0x9b, // EQ|LT|GT => setnp (P=0)
                        Unordered                  => 0x9a, // UN       => setp  (P=1)
                        OrderedNotEqual            => 0x95, // LT|GT    => setne (Z=0),
                        UnorderedOrEqual           => 0x94, // UN|EQ    => sete  (Z=1)
                        GreaterThan                => 0x97, // GT       => seta  (C=0&Z=0)
                        GreaterThanOrEqual         => 0x93, // GT|EQ    => setae (C=0)
                        UnorderedOrLessThan        => 0x92, // UN|LT    => setb  (C=1)
                        UnorderedOrLessThanOrEqual => 0x96, // UN|LT|EQ => setbe (Z=1|C=1)
                        Equal |                       // EQ
                        NotEqual |                    // UN|LT|GT
                        LessThan |                    // LT
                        LessThanOrEqual |             // LT|EQ
                        UnorderedOrGreaterThan |      // UN|GT
                        UnorderedOrGreaterThanOrEqual // UN|GT|EQ
                        => panic!("{} not supported by fcscc", cond),
                    };
                    sink.put1(0x0f);
                    sink.put1(setcc);
                    modrm_rr(out_reg0, 0, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("is_zero", f_unary, 2 + 2)
            .operands_in(vec![gpr])
            .operands_out(vec![abcd])
            .emit(
                r#"
                    // Test instruction.
                    {{PUT_OP}}(bits, rex2(in_reg0, in_reg0), sink);
                    modrm_rr(in_reg0, in_reg0, sink);
                    // Check ZF = 1 flag to see if register holds 0.
                    sink.put1(0x0f);
                    sink.put1(0x94);
                    modrm_rr(out_reg0, 0, sink);
                "#,
            ),
    );

    recipes.add_recipe(EncodingRecipeBuilder::new("safepoint", f_multiary, 0).emit(
        r#"
            sink.add_stackmap(args, func, isa);
        "#,
    ));

    recipes
}

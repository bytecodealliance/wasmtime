//! Encoding recipes for x86/x86_64.
use std::rc::Rc;

use cranelift_codegen_shared::isa::x86::EncodingBits;

use crate::cdsl::ast::Literal;
use crate::cdsl::formats::InstructionFormat;
use crate::cdsl::instructions::InstructionPredicate;
use crate::cdsl::recipes::{
    EncodingRecipe, EncodingRecipeBuilder, OperandConstraint, Register, Stack,
};
use crate::cdsl::regs::IsaRegs;
use crate::cdsl::settings::SettingGroup;
use crate::shared::Definitions as SharedDefinitions;

use crate::isa::x86::opcodes;

/// Helper data structure to create recipes and template recipes.
/// It contains all the recipes and recipe templates that might be used in the encodings crate of
/// this same directory.
pub(crate) struct RecipeGroup<'builder> {
    /// Memoized registers description, to pass it to builders later.
    regs: &'builder IsaRegs,

    /// All the recipes explicitly created in this file. This is different from the final set of
    /// recipes, which is definitive only once encodings have generated new recipes on the fly.
    recipes: Vec<EncodingRecipe>,

    /// All the recipe templates created in this file.
    templates: Vec<Rc<Template<'builder>>>,
}

impl<'builder> RecipeGroup<'builder> {
    fn new(regs: &'builder IsaRegs) -> Self {
        Self {
            regs,
            recipes: Vec::new(),
            templates: Vec::new(),
        }
    }
    fn add_recipe(&mut self, recipe: EncodingRecipeBuilder) {
        self.recipes.push(recipe.build());
    }
    fn add_template_recipe(&mut self, recipe: EncodingRecipeBuilder) -> Rc<Template<'builder>> {
        let template = Rc::new(Template::new(recipe, self.regs));
        self.templates.push(template.clone());
        template
    }
    fn add_template_inferred(
        &mut self,
        recipe: EncodingRecipeBuilder,
        infer_function: &'static str,
    ) -> Rc<Template<'builder>> {
        let template =
            Rc::new(Template::new(recipe, self.regs).inferred_rex_compute_size(infer_function));
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
            .find(|recipe| recipe.name == name)
            .unwrap_or_else(|| panic!("unknown recipe name: {}. Try template?", name))
    }
    pub fn template(&self, name: &str) -> &Template {
        self.templates
            .iter()
            .find(|recipe| recipe.name() == name)
            .unwrap_or_else(|| panic!("unknown template name: {}. Try recipe?", name))
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
    let enc = EncodingBits::new(op_bytes, rrr, w);
    (enc.prefix().recipe_name_prefix(), enc.bits())
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

fn replace_evex_constraints(
    _: &IsaRegs,
    constraints: Vec<OperandConstraint>,
) -> Vec<OperandConstraint> {
    constraints
        .into_iter()
        .map(|constraint| match constraint {
            OperandConstraint::RegClass(rc_index) => {
                // FIXME(#1306) this should be able to upgrade the register class to FPR32 as in
                // `replace_nonrex_constraints` above, e.g. When FPR32 is re-added, add back in the
                // rc_index conversion to FPR32. In the meantime, this is effectively a no-op
                // conversion--the register class stays the same.
                OperandConstraint::RegClass(rc_index)
            }
            _ => constraint,
        })
        .collect()
}

/// Specifies how the prefix (e.g. REX) is emitted by a Recipe.
#[derive(Copy, Clone, PartialEq)]
pub enum RecipePrefixKind {
    /// The REX emission behavior is not hardcoded for the Recipe
    /// and may be overridden when using the Template.
    Unspecified,

    /// The Recipe must hardcode the non-emission of the REX prefix.
    NeverEmitRex,

    /// The Recipe must hardcode the emission of the REX prefix.
    AlwaysEmitRex,

    /// The Recipe should infer the emission of the REX.RXB bits from registers,
    /// and the REX.W bit from the EncodingBits.
    ///
    /// Because such a Recipe has a non-constant instruction size, it must have
    /// a special `compute_size` handler for the inferrable-REX case.
    InferRex,

    /// The Recipe must hardcode the emission of an EVEX prefix.
    Evex,
}

impl Default for RecipePrefixKind {
    fn default() -> Self {
        Self::Unspecified
    }
}

/// Previously called a TailRecipe in the Python meta language, this allows to create multiple
/// variants of a single base EncodingRecipe (rex prefix, specialized w/rrr bits, different
/// opcodes). It serves as a prototype of an EncodingRecipe, which is then used when actually creating
/// Encodings, in encodings.rs. This is an idiosyncrasy of the x86 meta-language, and could be
/// reconsidered later.
#[derive(Clone)]
pub(crate) struct Template<'builder> {
    /// Description of registers, used in the build() method.
    regs: &'builder IsaRegs,

    /// The recipe template, which is to be specialized (by copy).
    recipe: EncodingRecipeBuilder,

    /// How is the REX prefix emitted?
    rex_kind: RecipePrefixKind,

    /// Function for `compute_size()` when REX is inferrable.
    inferred_rex_compute_size: Option<&'static str>,

    /// Other recipe to use when REX-prefixed.
    when_prefixed: Option<Rc<Template<'builder>>>,

    // Parameters passed in the EncodingBits.
    /// Value of the W bit (0 or 1), stored in the EncodingBits.
    w_bit: u16,
    /// Value of the RRR bits (between 0 and 0b111).
    rrr_bits: u16,
    /// Opcode bytes.
    op_bytes: &'static [u8],
}

impl<'builder> Template<'builder> {
    fn new(recipe: EncodingRecipeBuilder, regs: &'builder IsaRegs) -> Self {
        Self {
            regs,
            recipe,
            rex_kind: RecipePrefixKind::default(),
            inferred_rex_compute_size: None,
            when_prefixed: None,
            w_bit: 0,
            rrr_bits: 0,
            op_bytes: &opcodes::EMPTY,
        }
    }

    fn name(&self) -> &str {
        &self.recipe.name
    }
    fn rex_kind(self, kind: RecipePrefixKind) -> Self {
        Self {
            rex_kind: kind,
            ..self
        }
    }
    fn inferred_rex_compute_size(self, function: &'static str) -> Self {
        Self {
            inferred_rex_compute_size: Some(function),
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
    pub fn opcodes(&self, op_bytes: &'static [u8]) -> Self {
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
        assert!(
            self.rex_kind != RecipePrefixKind::AlwaysEmitRex,
            "Template requires REX prefix."
        );
        let mut copy = self.clone();
        copy.rex_kind = RecipePrefixKind::NeverEmitRex;
        copy
    }
    pub fn rex(&self) -> Self {
        assert!(
            self.rex_kind != RecipePrefixKind::NeverEmitRex,
            "Template requires no REX prefix."
        );
        if let Some(prefixed) = &self.when_prefixed {
            let mut ret = prefixed.rex();
            // Forward specialized parameters.
            ret.op_bytes = self.op_bytes;
            ret.w_bit = self.w_bit;
            ret.rrr_bits = self.rrr_bits;
            return ret;
        }
        let mut copy = self.clone();
        copy.rex_kind = RecipePrefixKind::AlwaysEmitRex;
        copy
    }
    pub fn infer_rex(&self) -> Self {
        assert!(
            self.rex_kind != RecipePrefixKind::NeverEmitRex,
            "Template requires no REX prefix."
        );
        assert!(
            self.when_prefixed.is_none(),
            "infer_rex used with when_prefixed()."
        );
        let mut copy = self.clone();
        copy.rex_kind = RecipePrefixKind::InferRex;
        copy
    }

    pub fn build(mut self) -> (EncodingRecipe, u16) {
        let (opcode, bits) = decode_opcodes(&self.op_bytes, self.rrr_bits, self.w_bit);

        let (recipe_name, size_addendum) = match self.rex_kind {
            RecipePrefixKind::Unspecified | RecipePrefixKind::NeverEmitRex => {
                // Ensure the operands are limited to non-REX constraints.
                let operands_in = self.recipe.operands_in.unwrap_or_default();
                self.recipe.operands_in = Some(replace_nonrex_constraints(self.regs, operands_in));
                let operands_out = self.recipe.operands_out.unwrap_or_default();
                self.recipe.operands_out =
                    Some(replace_nonrex_constraints(self.regs, operands_out));

                (opcode.into(), self.op_bytes.len() as u64)
            }
            RecipePrefixKind::AlwaysEmitRex => {
                ("Rex".to_string() + opcode, self.op_bytes.len() as u64 + 1)
            }
            RecipePrefixKind::InferRex => {
                assert_eq!(self.w_bit, 0, "A REX.W bit always requires a REX prefix; avoid using `infer_rex().w()` and use `rex().w()` instead.");
                // Hook up the right function for inferred compute_size().
                assert!(
                    self.inferred_rex_compute_size.is_some(),
                    "InferRex recipe '{}' needs an inferred_rex_compute_size function.",
                    &self.recipe.name
                );
                self.recipe.compute_size = self.inferred_rex_compute_size;

                ("DynRex".to_string() + opcode, self.op_bytes.len() as u64)
            }
            RecipePrefixKind::Evex => {
                // Allow the operands to expand limits to EVEX constraints.
                let operands_in = self.recipe.operands_in.unwrap_or_default();
                self.recipe.operands_in = Some(replace_evex_constraints(self.regs, operands_in));
                let operands_out = self.recipe.operands_out.unwrap_or_default();
                self.recipe.operands_out = Some(replace_evex_constraints(self.regs, operands_out));

                ("Evex".to_string() + opcode, 4 + 1)
            }
        };

        self.recipe.base_size += size_addendum;

        // Branch ranges are relative to the end of the instruction.
        // For InferRex, the range should be the minimum, assuming no REX.
        if let Some(range) = self.recipe.branch_range.as_mut() {
            range.inst_size += size_addendum;
        }

        self.recipe.emit = replace_put_op(self.recipe.emit, &recipe_name);
        self.recipe.name = recipe_name + &self.recipe.name;

        (self.recipe.build(), bits)
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
    let reg_xmm0 = Register::new(fpr, regs.regunit_by_name(fpr, "xmm0"));

    // Stack operand with a 32-bit signed displacement from either RBP or RSP.
    let stack_gpr32 = Stack::new(gpr);
    let stack_fpr32 = Stack::new(fpr);

    let formats = &shared_defs.formats;

    // Predicates shorthands.
    let use_sse41 = settings.predicate_by_name("use_sse41");

    // Definitions.
    let mut recipes = RecipeGroup::new(regs);

    // A null unary instruction that takes a GPR register. Can be used for identity copies and
    // no-op conversions.
    recipes.add_recipe(
        EncodingRecipeBuilder::new("null", &formats.unary, 0)
            .operands_in(vec![gpr])
            .operands_out(vec![0])
            .emit(""),
    );
    recipes.add_recipe(
        EncodingRecipeBuilder::new("null_fpr", &formats.unary, 0)
            .operands_in(vec![fpr])
            .operands_out(vec![0])
            .emit(""),
    );
    recipes.add_recipe(
        EncodingRecipeBuilder::new("stacknull", &formats.unary, 0)
            .operands_in(vec![stack_gpr32])
            .operands_out(vec![stack_gpr32])
            .emit(""),
    );

    recipes.add_recipe(
        EncodingRecipeBuilder::new("get_pinned_reg", &formats.nullary, 0)
            .operands_out(vec![reg_r15])
            .emit(""),
    );
    // umr with a fixed register output that's r15.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("set_pinned_reg", &formats.unary, 1)
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
        EncodingRecipeBuilder::new("fillnull", &formats.unary, 0)
            .operands_in(vec![stack_gpr32])
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .emit(""),
    );
    recipes.add_recipe(
        EncodingRecipeBuilder::new("ffillnull", &formats.unary, 0)
            .operands_in(vec![stack_gpr32])
            .operands_out(vec![fpr])
            .clobbers_flags(false)
            .emit(""),
    );

    recipes.add_recipe(
        EncodingRecipeBuilder::new("debugtrap", &formats.nullary, 1).emit("sink.put1(0xcc);"),
    );

    // XX opcode, no ModR/M.
    recipes.add_template_recipe(EncodingRecipeBuilder::new("trap", &formats.trap, 0).emit(
        r#"
            sink.trap(code, func.srclocs[inst]);
            {{PUT_OP}}(bits, BASE_REX, sink);
        "#,
    ));

    // Macro: conditional jump over a ud2.
    recipes.add_recipe(
        EncodingRecipeBuilder::new("trapif", &formats.int_cond_trap, 4)
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
        EncodingRecipeBuilder::new("trapff", &formats.float_cond_trap, 4)
            .operands_in(vec![reg_rflags])
            .clobbers_flags(false)
            .inst_predicate(supported_floatccs_predicate(
                &supported_floatccs,
                &*formats.float_cond_trap,
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
    recipes.add_template_inferred(
        EncodingRecipeBuilder::new("rr", &formats.binary, 1)
            .operands_in(vec![gpr, gpr])
            .operands_out(vec![0])
            .emit(
                r#"
                        {{PUT_OP}}(bits, rex2(in_reg0, in_reg1), sink);
                        modrm_rr(in_reg0, in_reg1, sink);
                    "#,
            ),
        "size_with_inferred_rex_for_inreg0_inreg1",
    );

    // XX /r with operands swapped. (RM form).
    recipes.add_template_inferred(
        EncodingRecipeBuilder::new("rrx", &formats.binary, 1)
            .operands_in(vec![gpr, gpr])
            .operands_out(vec![0])
            .emit(
                r#"
                        {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                        modrm_rr(in_reg1, in_reg0, sink);
                    "#,
            ),
        "size_with_inferred_rex_for_inreg0_inreg1",
    );

    // XX /r with FPR ins and outs. A form.
    recipes.add_template_inferred(
        EncodingRecipeBuilder::new("fa", &formats.binary, 1)
            .operands_in(vec![fpr, fpr])
            .operands_out(vec![0])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                    modrm_rr(in_reg1, in_reg0, sink);
                "#,
            ),
        "size_with_inferred_rex_for_inreg0_inreg1",
    );

    // XX /r with FPR ins and outs. A form with input operands swapped.
    recipes.add_template_inferred(
        EncodingRecipeBuilder::new("fax", &formats.binary, 1)
            .operands_in(vec![fpr, fpr])
            .operands_out(vec![1])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, in_reg1), sink);
                    modrm_rr(in_reg0, in_reg1, sink);
                "#,
            ),
        // The operand order does not matter for calculating whether a REX prefix is needed.
        "size_with_inferred_rex_for_inreg0_inreg1",
    );

    // XX /r with FPR ins and outs. A form with a byte immediate.
    {
        recipes.add_template_inferred(
            EncodingRecipeBuilder::new("fa_ib", &formats.ternary_imm8, 2)
                .operands_in(vec![fpr, fpr])
                .operands_out(vec![0])
                .inst_predicate(InstructionPredicate::new_is_unsigned_int(
                    &*formats.ternary_imm8,
                    "imm",
                    8,
                    0,
                ))
                .emit(
                    r#"
                    {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                    modrm_rr(in_reg1, in_reg0, sink);
                    let imm: i64 = imm.into();
                    sink.put1(imm as u8);
                "#,
                ),
            "size_with_inferred_rex_for_inreg0_inreg1",
        );
    }

    // XX /n for a unary operation with extension bits.
    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("ur", &formats.unary, 1)
                .operands_in(vec![gpr])
                .operands_out(vec![0])
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex1(in_reg0), sink);
                        modrm_r_bits(in_reg0, bits, sink);
                    "#,
                ),
            regs,
        )
        .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0"),
    );

    // XX /r, but for a unary operator with separate input/output register, like
    // copies. MR form, preserving flags.
    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("umr", &formats.unary, 1)
                .operands_in(vec![gpr])
                .operands_out(vec![gpr])
                .clobbers_flags(false)
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex2(out_reg0, in_reg0), sink);
                        modrm_rr(out_reg0, in_reg0, sink);
                    "#,
                ),
            regs,
        )
        .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0_outreg0"),
    );

    // Same as umr, but with FPR -> GPR registers.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rfumr", &formats.unary, 1)
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
        EncodingRecipeBuilder::new("umr_reg_to_ssa", &formats.copy_to_ssa, 1)
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
        EncodingRecipeBuilder::new("urm", &formats.unary, 1)
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
        EncodingRecipeBuilder::new("urm_noflags", &formats.unary, 1)
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
            EncodingRecipeBuilder::new("urm_noflags_abcd", &formats.unary, 1)
                .operands_in(vec![abcd])
                .operands_out(vec![gpr])
                .clobbers_flags(false)
                .emit(
                    r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                    modrm_rr(in_reg0, out_reg0, sink);
                "#,
                ),
            regs,
        )
        .when_prefixed(urm_noflags),
    );

    // XX /r, RM form, FPR -> FPR.
    recipes.add_template_inferred(
        EncodingRecipeBuilder::new("furm", &formats.unary, 1)
            .operands_in(vec![fpr])
            .operands_out(vec![fpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                    modrm_rr(in_reg0, out_reg0, sink);
                "#,
            ),
        "size_with_inferred_rex_for_inreg0_outreg0",
    );

    // Same as furm, but with the source register specified directly.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("furm_reg_to_ssa", &formats.copy_to_ssa, 1)
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
    recipes.add_template_inferred(
        EncodingRecipeBuilder::new("frurm", &formats.unary, 1)
            .operands_in(vec![gpr])
            .operands_out(vec![fpr])
            .clobbers_flags(false)
            .emit(
                r#"
                        {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                        modrm_rr(in_reg0, out_reg0, sink);
                    "#,
            ),
        "size_with_inferred_rex_for_inreg0_outreg0",
    );

    // XX /r, RM form, FPR -> GPR.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rfurm", &formats.unary, 1)
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
        EncodingRecipeBuilder::new("furmi_rnd", &formats.unary, 2)
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
        EncodingRecipeBuilder::new("rmov", &formats.reg_move, 1)
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
        EncodingRecipeBuilder::new("frmov", &formats.reg_move, 1)
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
        EncodingRecipeBuilder::new("rc", &formats.binary, 1)
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
    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("div", &formats.ternary, 1)
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
            regs,
        )
        .inferred_rex_compute_size("size_with_inferred_rex_for_inreg2"),
    );

    // XX /n for {s,u}mulx: inputs in %rax, r. Outputs in %rdx(hi):%rax(lo)
    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("mulx", &formats.binary, 1)
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
            regs,
        )
        .inferred_rex_compute_size("size_with_inferred_rex_for_inreg1"),
    );

    // XX /r for BLEND* instructions
    recipes.add_template_inferred(
        EncodingRecipeBuilder::new("blend", &formats.ternary, 1)
            .operands_in(vec![
                OperandConstraint::FixedReg(reg_xmm0),
                OperandConstraint::RegClass(fpr),
                OperandConstraint::RegClass(fpr),
            ])
            .operands_out(vec![2])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg1, in_reg2), sink);
                    modrm_rr(in_reg1, in_reg2, sink);
                "#,
            ),
        "size_with_inferred_rex_for_inreg1_inreg2",
    );

    // XX /n ib with 8-bit immediate sign-extended.
    {
        recipes.add_template_inferred(
            EncodingRecipeBuilder::new("r_ib", &formats.binary_imm64, 2)
                .operands_in(vec![gpr])
                .operands_out(vec![0])
                .inst_predicate(InstructionPredicate::new_is_signed_int(
                    &*formats.binary_imm64,
                    "imm",
                    8,
                    0,
                ))
                .emit(
                    r#"
                            {{PUT_OP}}(bits, rex1(in_reg0), sink);
                            modrm_r_bits(in_reg0, bits, sink);
                            let imm: i64 = imm.into();
                            sink.put1(imm as u8);
                        "#,
                ),
            "size_with_inferred_rex_for_inreg0",
        );

        recipes.add_template_inferred(
            EncodingRecipeBuilder::new("f_ib", &formats.binary_imm64, 2)
                .operands_in(vec![fpr])
                .operands_out(vec![0])
                .inst_predicate(InstructionPredicate::new_is_signed_int(
                    &*formats.binary_imm64,
                    "imm",
                    8,
                    0,
                ))
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex1(in_reg0), sink);
                        modrm_r_bits(in_reg0, bits, sink);
                        let imm: i64 = imm.into();
                        sink.put1(imm as u8);
                    "#,
                ),
            "size_with_inferred_rex_for_inreg0",
        );

        // XX /n id with 32-bit immediate sign-extended.
        recipes.add_template(
            Template::new(
                EncodingRecipeBuilder::new("r_id", &formats.binary_imm64, 5)
                    .operands_in(vec![gpr])
                    .operands_out(vec![0])
                    .inst_predicate(InstructionPredicate::new_is_signed_int(
                        &*formats.binary_imm64,
                        "imm",
                        32,
                        0,
                    ))
                    .emit(
                        r#"
                            {{PUT_OP}}(bits, rex1(in_reg0), sink);
                            modrm_r_bits(in_reg0, bits, sink);
                            let imm: i64 = imm.into();
                            sink.put4(imm as u32);
                        "#,
                    ),
                regs,
            )
            .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0"),
        );
    }

    // XX /r ib with 8-bit unsigned immediate (e.g. for pshufd)
    {
        recipes.add_template_inferred(
            EncodingRecipeBuilder::new("r_ib_unsigned_fpr", &formats.binary_imm8, 2)
                .operands_in(vec![fpr])
                .operands_out(vec![fpr])
                .inst_predicate(InstructionPredicate::new_is_unsigned_int(
                    &*formats.binary_imm8,
                    "imm",
                    8,
                    0,
                ))
                .emit(
                    r#"
                    {{PUT_OP}}(bits, rex2(in_reg0, out_reg0), sink);
                    modrm_rr(in_reg0, out_reg0, sink);
                    let imm: i64 = imm.into();
                    sink.put1(imm as u8);
                "#,
                ),
            "size_with_inferred_rex_for_inreg0_outreg0",
        );
    }

    // XX /r ib with 8-bit unsigned immediate (e.g. for extractlane)
    {
        recipes.add_template_inferred(
            EncodingRecipeBuilder::new("r_ib_unsigned_gpr", &formats.binary_imm8, 2)
                .operands_in(vec![fpr])
                .operands_out(vec![gpr])
                .inst_predicate(InstructionPredicate::new_is_unsigned_int(
                    &*formats.binary_imm8, "imm", 8, 0,
                ))
                .emit(
                    r#"
                    {{PUT_OP}}(bits, rex2(out_reg0, in_reg0), sink);
                    modrm_rr(out_reg0, in_reg0, sink); // note the flipped register in the ModR/M byte
                    let imm: i64 = imm.into();
                    sink.put1(imm as u8);
                "#,
                ), "size_with_inferred_rex_for_inreg0_outreg0"
        );
    }

    // XX /r ib with 8-bit unsigned immediate (e.g. for insertlane)
    {
        recipes.add_template_inferred(
            EncodingRecipeBuilder::new("r_ib_unsigned_r", &formats.ternary_imm8, 2)
                .operands_in(vec![fpr, gpr])
                .operands_out(vec![0])
                .inst_predicate(InstructionPredicate::new_is_unsigned_int(
                    &*formats.ternary_imm8,
                    "imm",
                    8,
                    0,
                ))
                .emit(
                    r#"
                    {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                    modrm_rr(in_reg1, in_reg0, sink);
                    let imm: i64 = imm.into();
                    sink.put1(imm as u8);
                "#,
                ),
            "size_with_inferred_rex_for_inreg0_inreg1",
        );
    }

    {
        // XX /n id with 32-bit immediate sign-extended. UnaryImm version.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("u_id", &formats.unary_imm, 5)
                .operands_out(vec![gpr])
                .inst_predicate(InstructionPredicate::new_is_signed_int(
                    &*formats.unary_imm,
                    "imm",
                    32,
                    0,
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
        EncodingRecipeBuilder::new("pu_id", &formats.unary_imm, 4)
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
        EncodingRecipeBuilder::new("pu_id_bool", &formats.unary_bool, 4)
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
        EncodingRecipeBuilder::new("pu_id_ref", &formats.nullary, 4)
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
        EncodingRecipeBuilder::new("pu_iq", &formats.unary_imm, 8)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    let imm: i64 = imm.into();
                    sink.put8(imm as u64);
                "#,
            ),
    );

    // XX+rd id unary with zero immediate.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("u_id_z", &formats.unary_imm, 1)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(out_reg0, out_reg0), sink);
                    modrm_rr(out_reg0, out_reg0, sink);
                "#,
            ),
    );

    // XX /n Unary with floating point 32-bit immediate equal to zero.
    {
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("f32imm_z", &formats.unary_ieee32, 1)
                .operands_out(vec![fpr])
                .inst_predicate(InstructionPredicate::new_is_zero_32bit_float(
                    &*formats.unary_ieee32,
                    "imm",
                ))
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
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("f64imm_z", &formats.unary_ieee64, 1)
                .operands_out(vec![fpr])
                .inst_predicate(InstructionPredicate::new_is_zero_64bit_float(
                    &*formats.unary_ieee64,
                    "imm",
                ))
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex2(out_reg0, out_reg0), sink);
                        modrm_rr(out_reg0, out_reg0, sink);
                    "#,
                ),
        );
    }

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("pushq", &formats.unary, 0)
            .operands_in(vec![gpr])
            .emit(
                r#"
                    sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                    {{PUT_OP}}(bits | (in_reg0 & 7), rex1(in_reg0), sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("popq", &formats.nullary, 0)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                "#,
            ),
    );

    // XX /r, for regmove instructions.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("copysp", &formats.copy_special, 1)
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(dst, src), sink);
                    modrm_rr(dst, src, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("adjustsp", &formats.unary, 1)
            .operands_in(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(RU::rsp.into(), in_reg0), sink);
                    modrm_rr(RU::rsp.into(), in_reg0, sink);
                "#,
            ),
    );

    {
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("adjustsp_ib", &formats.unary_imm, 2)
                .inst_predicate(InstructionPredicate::new_is_signed_int(
                    &*formats.unary_imm,
                    "imm",
                    8,
                    0,
                ))
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
            EncodingRecipeBuilder::new("adjustsp_id", &formats.unary_imm, 5)
                .inst_predicate(InstructionPredicate::new_is_signed_int(
                    &*formats.unary_imm,
                    "imm",
                    32,
                    0,
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
        EncodingRecipeBuilder::new("fnaddr4", &formats.func_addr, 4)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    sink.reloc_external(func.srclocs[inst],
                                        Reloc::Abs4,
                                        &func.dfg.ext_funcs[func_ref].name,
                                        0);
                    sink.put4(0);
                "#,
            ),
    );

    // XX+rd iq with Abs8 function relocation.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("fnaddr8", &formats.func_addr, 8)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    sink.reloc_external(func.srclocs[inst],
                                        Reloc::Abs8,
                                        &func.dfg.ext_funcs[func_ref].name,
                                        0);
                    sink.put8(0);
                "#,
            ),
    );

    // Similar to fnaddr4, but writes !0 (this is used by BaldrMonkey).
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("allones_fnaddr4", &formats.func_addr, 4)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    sink.reloc_external(func.srclocs[inst],
                                        Reloc::Abs4,
                                        &func.dfg.ext_funcs[func_ref].name,
                                        0);
                    // Write the immediate as `!0` for the benefit of BaldrMonkey.
                    sink.put4(!0);
                "#,
            ),
    );

    // Similar to fnaddr8, but writes !0 (this is used by BaldrMonkey).
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("allones_fnaddr8", &formats.func_addr, 8)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    sink.reloc_external(func.srclocs[inst],
                                        Reloc::Abs8,
                                        &func.dfg.ext_funcs[func_ref].name,
                                        0);
                    // Write the immediate as `!0` for the benefit of BaldrMonkey.
                    sink.put8(!0);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("pcrel_fnaddr8", &formats.func_addr, 5)
            .operands_out(vec![gpr])
            // rex2 gets passed 0 for r/m register because the upper bit of
            // r/m doesn't get decoded when in rip-relative addressing mode.
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(0, out_reg0), sink);
                    modrm_riprel(out_reg0, sink);
                    // The addend adjusts for the difference between the end of the
                    // instruction and the beginning of the immediate field.
                    sink.reloc_external(func.srclocs[inst],
                                        Reloc::X86PCRel4,
                                        &func.dfg.ext_funcs[func_ref].name,
                                        -4);
                    sink.put4(0);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("got_fnaddr8", &formats.func_addr, 5)
            .operands_out(vec![gpr])
            // rex2 gets passed 0 for r/m register because the upper bit of
            // r/m doesn't get decoded when in rip-relative addressing mode.
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(0, out_reg0), sink);
                    modrm_riprel(out_reg0, sink);
                    // The addend adjusts for the difference between the end of the
                    // instruction and the beginning of the immediate field.
                    sink.reloc_external(func.srclocs[inst],
                                        Reloc::X86GOTPCRel4,
                                        &func.dfg.ext_funcs[func_ref].name,
                                        -4);
                    sink.put4(0);
                "#,
            ),
    );

    // XX+rd id with Abs4 globalsym relocation.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("gvaddr4", &formats.unary_global_value, 4)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    sink.reloc_external(func.srclocs[inst],
                                        Reloc::Abs4,
                                        &func.global_values[global_value].symbol_name(),
                                        0);
                    sink.put4(0);
                "#,
            ),
    );

    // XX+rd iq with Abs8 globalsym relocation.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("gvaddr8", &formats.unary_global_value, 8)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                    sink.reloc_external(func.srclocs[inst],
                                        Reloc::Abs8,
                                        &func.global_values[global_value].symbol_name(),
                                        0);
                    sink.put8(0);
                "#,
            ),
    );

    // XX+rd iq with PCRel4 globalsym relocation.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("pcrel_gvaddr8", &formats.unary_global_value, 5)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(0, out_reg0), sink);
                    modrm_rm(5, out_reg0, sink);
                    // The addend adjusts for the difference between the end of the
                    // instruction and the beginning of the immediate field.
                    sink.reloc_external(func.srclocs[inst],
                                        Reloc::X86PCRel4,
                                        &func.global_values[global_value].symbol_name(),
                                        -4);
                    sink.put4(0);
                "#,
            ),
    );

    // XX+rd iq with Abs8 globalsym relocation.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("got_gvaddr8", &formats.unary_global_value, 5)
            .operands_out(vec![gpr])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(0, out_reg0), sink);
                    modrm_rm(5, out_reg0, sink);
                    // The addend adjusts for the difference between the end of the
                    // instruction and the beginning of the immediate field.
                    sink.reloc_external(func.srclocs[inst],
                                        Reloc::X86GOTPCRel4,
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
        EncodingRecipeBuilder::new("spaddr_id", &formats.stack_load, 6)
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

    // Constant addresses.

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("const_addr", &formats.unary_const, 5)
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(0, out_reg0), sink);
                    modrm_riprel(out_reg0, sink);
                    const_disp4(constant_handle, func, sink);
                "#,
            ),
    );

    // Store recipes.

    {
        // Simple stores.

        // A predicate asking if the offset is zero.
        let has_no_offset =
            InstructionPredicate::new_is_field_equal(&*formats.store, "offset", "0".into());

        // XX /r register-indirect store with no offset.
        let st = recipes.add_template_recipe(
            EncodingRecipeBuilder::new("st", &formats.store, 1)
                .operands_in(vec![gpr, gpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_or_offset_for_inreg_1")
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
                EncodingRecipeBuilder::new("st_abcd", &formats.store, 1)
                    .operands_in(vec![abcd, gpr])
                    .inst_predicate(has_no_offset.clone())
                    .clobbers_flags(false)
                    .compute_size("size_plus_maybe_sib_or_offset_for_inreg_1")
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
                regs,
            )
            .when_prefixed(st),
        );

        // XX /r register-indirect store of FPR with no offset.
        recipes.add_template_inferred(
            EncodingRecipeBuilder::new("fst", &formats.store, 1)
                .operands_in(vec![fpr, gpr])
                .inst_predicate(has_no_offset)
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_or_offset_for_inreg_1")
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
            "size_plus_maybe_sib_or_offset_inreg1_plus_rex_prefix_for_inreg0_inreg1",
        );

        let has_small_offset =
            InstructionPredicate::new_is_signed_int(&*formats.store, "offset", 8, 0);

        // XX /r register-indirect store with 8-bit offset.
        let st_disp8 = recipes.add_template_recipe(
            EncodingRecipeBuilder::new("stDisp8", &formats.store, 2)
                .operands_in(vec![gpr, gpr])
                .inst_predicate(has_small_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_inreg_1")
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
                EncodingRecipeBuilder::new("stDisp8_abcd", &formats.store, 2)
                    .operands_in(vec![abcd, gpr])
                    .inst_predicate(has_small_offset.clone())
                    .clobbers_flags(false)
                    .compute_size("size_plus_maybe_sib_for_inreg_1")
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
                regs,
            )
            .when_prefixed(st_disp8),
        );

        // XX /r register-indirect store with 8-bit offset of FPR.
        recipes.add_template_inferred(
            EncodingRecipeBuilder::new("fstDisp8", &formats.store, 2)
                .operands_in(vec![fpr, gpr])
                .inst_predicate(has_small_offset)
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_inreg_1")
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
            "size_plus_maybe_sib_inreg1_plus_rex_prefix_for_inreg0_inreg1",
        );

        // XX /r register-indirect store with 32-bit offset.
        let st_disp32 = recipes.add_template_recipe(
            EncodingRecipeBuilder::new("stDisp32", &formats.store, 5)
                .operands_in(vec![gpr, gpr])
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_inreg_1")
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
                EncodingRecipeBuilder::new("stDisp32_abcd", &formats.store, 5)
                    .operands_in(vec![abcd, gpr])
                    .clobbers_flags(false)
                    .compute_size("size_plus_maybe_sib_for_inreg_1")
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
                regs,
            )
            .when_prefixed(st_disp32),
        );

        // XX /r register-indirect store with 32-bit offset of FPR.
        recipes.add_template_inferred(
            EncodingRecipeBuilder::new("fstDisp32", &formats.store, 5)
                .operands_in(vec![fpr, gpr])
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_inreg_1")
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
            "size_plus_maybe_sib_inreg1_plus_rex_prefix_for_inreg0_inreg1",
        );
    }

    {
        // Complex stores.

        // A predicate asking if the offset is zero.
        let has_no_offset =
            InstructionPredicate::new_is_field_equal(&*formats.store_complex, "offset", "0".into());

        // XX /r register-indirect store with index and no offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("stWithIndex", &formats.store_complex, 2)
                .operands_in(vec![gpr, gpr, gpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_offset_for_inreg_1")
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
            EncodingRecipeBuilder::new("stWithIndex_abcd", &formats.store_complex, 2)
                .operands_in(vec![abcd, gpr, gpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_offset_for_inreg_1")
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
            EncodingRecipeBuilder::new("fstWithIndex", &formats.store_complex, 2)
                .operands_in(vec![fpr, gpr, gpr])
                .inst_predicate(has_no_offset)
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_offset_for_inreg_1")
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

        let has_small_offset =
            InstructionPredicate::new_is_signed_int(&*formats.store_complex, "offset", 8, 0);

        // XX /r register-indirect store with index and 8-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("stWithIndexDisp8", &formats.store_complex, 3)
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
            EncodingRecipeBuilder::new("stWithIndexDisp8_abcd", &formats.store_complex, 3)
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
            EncodingRecipeBuilder::new("fstWithIndexDisp8", &formats.store_complex, 3)
                .operands_in(vec![fpr, gpr, gpr])
                .inst_predicate(has_small_offset)
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

        let has_big_offset =
            InstructionPredicate::new_is_signed_int(&*formats.store_complex, "offset", 32, 0);

        // XX /r register-indirect store with index and 32-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("stWithIndexDisp32", &formats.store_complex, 6)
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
            EncodingRecipeBuilder::new("stWithIndexDisp32_abcd", &formats.store_complex, 6)
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
            EncodingRecipeBuilder::new("fstWithIndexDisp32", &formats.store_complex, 6)
                .operands_in(vec![fpr, gpr, gpr])
                .inst_predicate(has_big_offset)
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
        EncodingRecipeBuilder::new("spillSib32", &formats.unary, 6)
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
        EncodingRecipeBuilder::new("fspillSib32", &formats.unary, 6)
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
        EncodingRecipeBuilder::new("regspill32", &formats.reg_spill, 6)
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
        EncodingRecipeBuilder::new("fregspill32", &formats.reg_spill, 6)
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

        // A predicate asking if the offset is zero.
        let has_no_offset =
            InstructionPredicate::new_is_field_equal(&*formats.load, "offset", "0".into());

        // XX /r load with no offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("ld", &formats.load, 1)
                .operands_in(vec![gpr])
                .operands_out(vec![gpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_or_offset_for_inreg_0")
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
        recipes.add_template_inferred(
            EncodingRecipeBuilder::new("fld", &formats.load, 1)
                .operands_in(vec![gpr])
                .operands_out(vec![fpr])
                .inst_predicate(has_no_offset)
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_or_offset_for_inreg_0")
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
            "size_plus_maybe_sib_or_offset_for_inreg_0_plus_rex_prefix_for_inreg0_outreg0",
        );

        let has_small_offset =
            InstructionPredicate::new_is_signed_int(&*formats.load, "offset", 8, 0);

        // XX /r load with 8-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("ldDisp8", &formats.load, 2)
                .operands_in(vec![gpr])
                .operands_out(vec![gpr])
                .inst_predicate(has_small_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_inreg_0")
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
        recipes.add_template_inferred(
            EncodingRecipeBuilder::new("fldDisp8", &formats.load, 2)
                .operands_in(vec![gpr])
                .operands_out(vec![fpr])
                .inst_predicate(has_small_offset)
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_inreg_0")
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
            "size_plus_maybe_sib_for_inreg_0_plus_rex_prefix_for_inreg0_outreg0",
        );

        let has_big_offset =
            InstructionPredicate::new_is_signed_int(&*formats.load, "offset", 32, 0);

        // XX /r load with 32-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("ldDisp32", &formats.load, 5)
                .operands_in(vec![gpr])
                .operands_out(vec![gpr])
                .inst_predicate(has_big_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_inreg_0")
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
        recipes.add_template_inferred(
            EncodingRecipeBuilder::new("fldDisp32", &formats.load, 5)
                .operands_in(vec![gpr])
                .operands_out(vec![fpr])
                .inst_predicate(has_big_offset)
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_sib_for_inreg_0")
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
            "size_plus_maybe_sib_for_inreg_0_plus_rex_prefix_for_inreg0_outreg0",
        );
    }

    {
        // Complex loads.

        // A predicate asking if the offset is zero.
        let has_no_offset =
            InstructionPredicate::new_is_field_equal(&*formats.load_complex, "offset", "0".into());

        // XX /r load with index and no offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("ldWithIndex", &formats.load_complex, 2)
                .operands_in(vec![gpr, gpr])
                .operands_out(vec![gpr])
                .inst_predicate(has_no_offset.clone())
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_offset_for_inreg_0")
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
            EncodingRecipeBuilder::new("fldWithIndex", &formats.load_complex, 2)
                .operands_in(vec![gpr, gpr])
                .operands_out(vec![fpr])
                .inst_predicate(has_no_offset)
                .clobbers_flags(false)
                .compute_size("size_plus_maybe_offset_for_inreg_0")
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

        let has_small_offset =
            InstructionPredicate::new_is_signed_int(&*formats.load_complex, "offset", 8, 0);

        // XX /r load with index and 8-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("ldWithIndexDisp8", &formats.load_complex, 3)
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
            EncodingRecipeBuilder::new("fldWithIndexDisp8", &formats.load_complex, 3)
                .operands_in(vec![gpr, gpr])
                .operands_out(vec![fpr])
                .inst_predicate(has_small_offset)
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

        let has_big_offset =
            InstructionPredicate::new_is_signed_int(&*formats.load_complex, "offset", 32, 0);

        // XX /r load with index and 32-bit offset.
        recipes.add_template_recipe(
            EncodingRecipeBuilder::new("ldWithIndexDisp32", &formats.load_complex, 6)
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
            EncodingRecipeBuilder::new("fldWithIndexDisp32", &formats.load_complex, 6)
                .operands_in(vec![gpr, gpr])
                .operands_out(vec![fpr])
                .inst_predicate(has_big_offset)
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
        EncodingRecipeBuilder::new("fillSib32", &formats.unary, 6)
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
        EncodingRecipeBuilder::new("ffillSib32", &formats.unary, 6)
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
        EncodingRecipeBuilder::new("regfill32", &formats.reg_fill, 6)
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
        EncodingRecipeBuilder::new("fregfill32", &formats.reg_fill, 6)
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

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("call_id", &formats.call, 4).emit(
            r#"
            sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
            {{PUT_OP}}(bits, BASE_REX, sink);
            // The addend adjusts for the difference between the end of the
            // instruction and the beginning of the immediate field.
            sink.reloc_external(func.srclocs[inst],
                                Reloc::X86CallPCRel4,
                                &func.dfg.ext_funcs[func_ref].name,
                                -4);
            sink.put4(0);
            sink.add_call_site(opcode, func.srclocs[inst]);
        "#,
        ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("call_plt_id", &formats.call, 4).emit(
            r#"
            sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
            {{PUT_OP}}(bits, BASE_REX, sink);
            sink.reloc_external(func.srclocs[inst],
                                Reloc::X86CallPLTRel4,
                                &func.dfg.ext_funcs[func_ref].name,
                                -4);
            sink.put4(0);
            sink.add_call_site(opcode, func.srclocs[inst]);
        "#,
        ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("call_r", &formats.call_indirect, 1)
            .operands_in(vec![gpr])
            .emit(
                r#"
                    sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                    {{PUT_OP}}(bits, rex1(in_reg0), sink);
                    modrm_r_bits(in_reg0, bits, sink);
                    sink.add_call_site(opcode, func.srclocs[inst]);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("ret", &formats.multiary, 0)
            .emit("{{PUT_OP}}(bits, BASE_REX, sink);"),
    );

    // Branches.

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("jmpb", &formats.jump, 1)
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
        EncodingRecipeBuilder::new("jmpd", &formats.jump, 4)
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
        EncodingRecipeBuilder::new("brib", &formats.branch_int, 1)
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
        EncodingRecipeBuilder::new("brid", &formats.branch_int, 4)
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
        EncodingRecipeBuilder::new("brfb", &formats.branch_float, 1)
            .operands_in(vec![reg_rflags])
            .branch_range((1, 8))
            .clobbers_flags(false)
            .inst_predicate(supported_floatccs_predicate(
                &supported_floatccs,
                &*formats.branch_float,
            ))
            .emit(
                r#"
                    {{PUT_OP}}(bits | fcc2opc(cond), BASE_REX, sink);
                    disp1(destination, func, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("brfd", &formats.branch_float, 4)
            .operands_in(vec![reg_rflags])
            .branch_range((4, 32))
            .clobbers_flags(false)
            .inst_predicate(supported_floatccs_predicate(
                &supported_floatccs,
                &*formats.branch_float,
            ))
            .emit(
                r#"
                    {{PUT_OP}}(bits | fcc2opc(cond), BASE_REX, sink);
                    disp4(destination, func, sink);
                "#,
            ),
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("indirect_jmp", &formats.indirect_jump, 1)
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
        EncodingRecipeBuilder::new("jt_entry", &formats.branch_table_entry, 2)
            .operands_in(vec![gpr, gpr])
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .inst_predicate(valid_scale(&*formats.branch_table_entry))
            .compute_size("size_plus_maybe_offset_for_inreg_1")
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

    recipes.add_template_inferred(
        EncodingRecipeBuilder::new("vconst", &formats.unary_const, 5)
            .operands_out(vec![fpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(0, out_reg0), sink);
                    modrm_riprel(out_reg0, sink);
                    const_disp4(constant_handle, func, sink);
                "#,
            ),
        "size_with_inferred_rex_for_outreg0",
    );

    recipes.add_template_inferred(
        EncodingRecipeBuilder::new("vconst_optimized", &formats.unary_const, 1)
            .operands_out(vec![fpr])
            .clobbers_flags(false)
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(out_reg0, out_reg0), sink);
                    modrm_rr(out_reg0, out_reg0, sink);
                "#,
            ),
        "size_with_inferred_rex_for_outreg0",
    );

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("jt_base", &formats.branch_table_base, 5)
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
            EncodingRecipeBuilder::new("seti", &formats.int_cond, 1)
                .operands_in(vec![reg_rflags])
                .operands_out(vec![gpr])
                .clobbers_flags(false)
                .emit(
                    r#"
                    {{PUT_OP}}(bits | icc2opc(cond), rex1(out_reg0), sink);
                    modrm_r_bits(out_reg0, bits, sink);
                "#,
                ),
            regs,
        )
        .rex_kind(RecipePrefixKind::AlwaysEmitRex),
    );

    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("seti_abcd", &formats.int_cond, 1)
                .operands_in(vec![reg_rflags])
                .operands_out(vec![abcd])
                .clobbers_flags(false)
                .emit(
                    r#"
                    {{PUT_OP}}(bits | icc2opc(cond), rex1(out_reg0), sink);
                    modrm_r_bits(out_reg0, bits, sink);
                "#,
                ),
            regs,
        )
        .when_prefixed(seti),
    );

    let setf = recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("setf", &formats.float_cond, 1)
                .operands_in(vec![reg_rflags])
                .operands_out(vec![gpr])
                .clobbers_flags(false)
                .emit(
                    r#"
                    {{PUT_OP}}(bits | fcc2opc(cond), rex1(out_reg0), sink);
                    modrm_r_bits(out_reg0, bits, sink);
                "#,
                ),
            regs,
        )
        .rex_kind(RecipePrefixKind::AlwaysEmitRex),
    );

    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("setf_abcd", &formats.float_cond, 1)
                .operands_in(vec![reg_rflags])
                .operands_out(vec![abcd])
                .clobbers_flags(false)
                .emit(
                    r#"
                    {{PUT_OP}}(bits | fcc2opc(cond), rex1(out_reg0), sink);
                    modrm_r_bits(out_reg0, bits, sink);
                "#,
                ),
            regs,
        )
        .when_prefixed(setf),
    );

    // Conditional move (a.k.a integer select)
    // (maybe-REX.W) 0F 4x modrm(r,r)
    // 1 byte, modrm(r,r), is after the opcode
    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("cmov", &formats.int_select, 1)
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
            regs,
        )
        .inferred_rex_compute_size("size_with_inferred_rex_for_cmov"),
    );

    // Bit scan forwards and reverse
    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("bsf_and_bsr", &formats.unary, 1)
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
            regs,
        )
        .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0_outreg0"),
    );

    // Arithematic with flag I/O.

    // XX /r, MR form. Add two GPR registers and set carry flag.
    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("rout", &formats.binary, 1)
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
            regs,
        )
        .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0_inreg1"),
    );

    // XX /r, MR form. Add two GPR registers and get carry flag.
    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("rin", &formats.ternary, 1)
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
            regs,
        )
        .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0_inreg1"),
    );

    // XX /r, MR form. Add two GPR registers with carry flag.
    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("rio", &formats.ternary, 1)
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
            regs,
        )
        .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0_inreg1"),
    );

    // Compare and set flags.

    // XX /r, MR form. Compare two GPR registers and set flags.
    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("rcmp", &formats.binary, 1)
                .operands_in(vec![gpr, gpr])
                .operands_out(vec![reg_rflags])
                .emit(
                    r#"
                        {{PUT_OP}}(bits, rex2(in_reg0, in_reg1), sink);
                        modrm_rr(in_reg0, in_reg1, sink);
                    "#,
                ),
            regs,
        )
        .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0_inreg1"),
    );

    // Same as rcmp, but second operand is the stack pointer.
    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("rcmp_sp", &formats.unary, 1)
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
    recipes.add_template_inferred(
        EncodingRecipeBuilder::new("fcmp", &formats.binary, 1)
            .operands_in(vec![fpr, fpr])
            .operands_out(vec![reg_rflags])
            .emit(
                r#"
                    {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                    modrm_rr(in_reg1, in_reg0, sink);
                "#,
            ),
        "size_with_inferred_rex_for_inreg0_inreg1",
    );

    {
        let has_small_offset =
            InstructionPredicate::new_is_signed_int(&*formats.binary_imm64, "imm", 8, 0);

        // XX /n, MI form with imm8.
        recipes.add_template(
            Template::new(
                EncodingRecipeBuilder::new("rcmp_ib", &formats.binary_imm64, 2)
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
                regs,
            )
            .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0"),
        );

        let has_big_offset =
            InstructionPredicate::new_is_signed_int(&*formats.binary_imm64, "imm", 32, 0);

        // XX /n, MI form with imm32.
        recipes.add_template(
            Template::new(
                EncodingRecipeBuilder::new("rcmp_id", &formats.binary_imm64, 5)
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
                regs,
            )
            .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0"),
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

    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("tjccb", &formats.branch, 1 + 2)
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
            regs,
        )
        .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0"),
    );

    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("tjccd", &formats.branch, 1 + 6)
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
            regs,
        )
        .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0"),
    );

    // 8-bit test-and-branch.

    let t8jccb = recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("t8jccb", &formats.branch, 1 + 2)
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
            regs,
        )
        .rex_kind(RecipePrefixKind::AlwaysEmitRex),
    );

    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("t8jccb_abcd", &formats.branch, 1 + 2)
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
            regs,
        )
        .when_prefixed(t8jccb),
    );

    let t8jccd = recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("t8jccd", &formats.branch, 1 + 6)
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
            regs,
        )
        .rex_kind(RecipePrefixKind::AlwaysEmitRex),
    );

    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("t8jccd_abcd", &formats.branch, 1 + 6)
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
        EncodingRecipeBuilder::new("t8jccd_long", &formats.branch, 5 + 6)
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

    recipes.add_template(
        Template::new(
            EncodingRecipeBuilder::new("icscc", &formats.int_compare, 1 + 3)
                .operands_in(vec![gpr, gpr])
                .operands_out(vec![abcd])
                .emit(
                    r#"
                        // Comparison instruction.
                        {{PUT_OP}}(bits, rex2(in_reg0, in_reg1), sink);
                        modrm_rr(in_reg0, in_reg1, sink);
                        // `setCC` instruction, no REX.
                        let setcc = 0x90 | icc2opc(cond);
                        sink.put1(0x0f);
                        sink.put1(setcc as u8);
                        modrm_rr(out_reg0, 0, sink);
                    "#,
                ),
            regs,
        )
        .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0_inreg1"),
    );

    recipes.add_template_inferred(
        EncodingRecipeBuilder::new("icscc_fpr", &formats.int_compare, 1)
            .operands_in(vec![fpr, fpr])
            .operands_out(vec![0])
            .emit(
                r#"
                    // Comparison instruction.
                    {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                    modrm_rr(in_reg1, in_reg0, sink);
                "#,
            ),
        "size_with_inferred_rex_for_inreg0_inreg1",
    );

    {
        let is_small_imm =
            InstructionPredicate::new_is_signed_int(&*formats.int_compare_imm, "imm", 8, 0);

        recipes.add_template(
            Template::new(
                EncodingRecipeBuilder::new("icscc_ib", &formats.int_compare_imm, 2 + 3)
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
                            let setcc = 0x90 | icc2opc(cond);
                            sink.put1(0x0f);
                            sink.put1(setcc as u8);
                            modrm_rr(out_reg0, 0, sink);
                        "#,
                    ),
                regs,
            )
            .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0"),
        );

        let is_big_imm =
            InstructionPredicate::new_is_signed_int(&*formats.int_compare_imm, "imm", 32, 0);

        recipes.add_template(
            Template::new(
                EncodingRecipeBuilder::new("icscc_id", &formats.int_compare_imm, 5 + 3)
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
                            let setcc = 0x90 | icc2opc(cond);
                            sink.put1(0x0f);
                            sink.put1(setcc as u8);
                            modrm_rr(out_reg0, 0, sink);
                        "#,
                    ),
                regs,
            )
            .inferred_rex_compute_size("size_with_inferred_rex_for_inreg0"),
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
        EncodingRecipeBuilder::new("fcscc", &formats.float_compare, 1 + 3)
            .operands_in(vec![fpr, fpr])
            .operands_out(vec![abcd])
            .inst_predicate(supported_floatccs_predicate(
                &supported_floatccs,
                &*formats.float_compare,
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

    {
        let supported_floatccs: Vec<Literal> = ["eq", "lt", "le", "uno", "ne", "uge", "ugt", "ord"]
            .iter()
            .map(|name| Literal::enumerator_for(floatcc, name))
            .collect();
        recipes.add_template_inferred(
            EncodingRecipeBuilder::new("pfcmp", &formats.float_compare, 2)
                .operands_in(vec![fpr, fpr])
                .operands_out(vec![0])
                .inst_predicate(supported_floatccs_predicate(
                    &supported_floatccs[..],
                    &*formats.float_compare,
                ))
                .emit(
                    r#"
                    // Comparison instruction.
                    {{PUT_OP}}(bits, rex2(in_reg1, in_reg0), sink);
                    modrm_rr(in_reg1, in_reg0, sink);
                    // Add immediate byte indicating what type of comparison.
                    use crate::ir::condcodes::FloatCC::*;
                    let imm = match cond {
                        Equal                      => 0x00,
                        LessThan                   => 0x01,
                        LessThanOrEqual            => 0x02,
                        Unordered                  => 0x03,
                        NotEqual                   => 0x04,
                        UnorderedOrGreaterThanOrEqual => 0x05,
                        UnorderedOrGreaterThan => 0x06,
                        Ordered                    => 0x07,
                        _ => panic!("{} not supported by pfcmp", cond),
                    };
                    sink.put1(imm);
                "#,
                ),
            "size_with_inferred_rex_for_inreg0_inreg1",
        );
    }

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("is_zero", &formats.unary, 2 + 2)
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

    recipes.add_template_recipe(
        EncodingRecipeBuilder::new("is_invalid", &formats.unary, 2 + 3)
            .operands_in(vec![gpr])
            .operands_out(vec![abcd])
            .emit(
                r#"
                    // Comparison instruction.
                    {{PUT_OP}}(bits, rex1(in_reg0), sink);
                    modrm_r_bits(in_reg0, bits, sink);
                    sink.put1(0xff);
                    // `setCC` instruction, no REX.
                    use crate::ir::condcodes::IntCC::*;
                    let setcc = 0x90 | icc2opc(Equal);
                    sink.put1(0x0f);
                    sink.put1(setcc as u8);
                    modrm_rr(out_reg0, 0, sink);
                "#,
            ),
    );

    recipes.add_recipe(
        EncodingRecipeBuilder::new("safepoint", &formats.multiary, 0).emit(
            r#"
                sink.add_stackmap(args, func, isa);
            "#,
        ),
    );

    // Both `elf_tls_get_addr` and `macho_tls_get_addr` require all caller-saved registers to be spilled.
    // This is currently special cased in `regalloc/spilling.rs` in the `visit_inst` function.

    recipes.add_recipe(
        EncodingRecipeBuilder::new("elf_tls_get_addr", &formats.unary_global_value, 16)
            // FIXME Correct encoding for non rax registers
            .operands_out(vec![reg_rax])
            .emit(
                r#"
                    // output %rax
                    // clobbers %rdi

                    // Those data16 prefixes are necessary to pad to 16 bytes.

                    // data16 lea gv@tlsgd(%rip),%rdi
                    sink.put1(0x66); // data16
                    sink.put1(0b01001000); // rex.w
                    const LEA: u8 = 0x8d;
                    sink.put1(LEA); // lea
                    modrm_riprel(0b111/*out_reg0*/, sink); // 0x3d
                    sink.reloc_external(func.srclocs[inst],
                                        Reloc::ElfX86_64TlsGd,
                                        &func.global_values[global_value].symbol_name(),
                                        -4);
                    sink.put4(0);

                    // data16 data16 callq __tls_get_addr-4
                    sink.put1(0x66); // data16
                    sink.put1(0x66); // data16
                    sink.put1(0b01001000); // rex.w
                    sink.put1(0xe8); // call
                    sink.reloc_external(func.srclocs[inst],
                                        Reloc::X86CallPLTRel4,
                                        &ExternalName::LibCall(LibCall::ElfTlsGetAddr),
                                        -4);
                    sink.put4(0);
                "#,
            ),
    );

    recipes.add_recipe(
        EncodingRecipeBuilder::new("macho_tls_get_addr", &formats.unary_global_value, 9)
            // FIXME Correct encoding for non rax registers
            .operands_out(vec![reg_rax])
            .emit(
                r#"
                    // output %rax
                    // clobbers %rdi

                    // movq gv@tlv(%rip), %rdi
                    sink.put1(0x48); // rex
                    sink.put1(0x8b); // mov
                    modrm_riprel(0b111/*out_reg0*/, sink); // 0x3d
                    sink.reloc_external(func.srclocs[inst],
                                        Reloc::MachOX86_64Tlv,
                                        &func.global_values[global_value].symbol_name(),
                                        -4);
                    sink.put4(0);

                    // callq *(%rdi)
                    sink.put1(0xff);
                    sink.put1(0x17);
                "#,
            ),
    );

    recipes.add_template(
        Template::new(
        EncodingRecipeBuilder::new("evex_reg_vvvv_rm_128", &formats.binary, 1)
            .operands_in(vec![fpr, fpr])
            .operands_out(vec![fpr])
            .emit(
                r#"
                // instruction encoding operands: reg (op1, w), vvvv (op2, r), rm (op3, r)
                // this maps to:                  out_reg0,     in_reg0,       in_reg1
                let context = EvexContext::Other { length: EvexVectorLength::V128 };
                let masking = EvexMasking::None;
                put_evex(bits, out_reg0, in_reg0, in_reg1, context, masking, sink); // params: reg, vvvv, rm
                modrm_rr(in_reg1, out_reg0, sink); // params: rm, reg
                "#,
            ),
        regs).rex_kind(RecipePrefixKind::Evex)
    );

    recipes
}

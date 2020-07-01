#![allow(non_snake_case)]

use cranelift_codegen_shared::condcodes::IntCC;
use std::collections::HashMap;

use crate::cdsl::encodings::{Encoding, EncodingBuilder};
use crate::cdsl::instructions::{
    vector, Bindable, Immediate, InstSpec, Instruction, InstructionGroup, InstructionPredicate,
    InstructionPredicateNode, InstructionPredicateRegistry,
};
use crate::cdsl::recipes::{EncodingRecipe, EncodingRecipeNumber, Recipes};
use crate::cdsl::settings::{SettingGroup, SettingPredicateNumber};
use crate::cdsl::types::{LaneType, ValueType};
use crate::shared::types::Bool::{B1, B16, B32, B64, B8};
use crate::shared::types::Float::{F32, F64};
use crate::shared::types::Int::{I16, I32, I64, I8};
use crate::shared::types::Reference::{R32, R64};
use crate::shared::Definitions as SharedDefinitions;

use crate::isa::x86::opcodes::*;

use super::recipes::{RecipeGroup, Template};
use crate::cdsl::instructions::BindParameter::Any;

pub(crate) struct PerCpuModeEncodings {
    pub enc32: Vec<Encoding>,
    pub enc64: Vec<Encoding>,
    pub recipes: Recipes,
    recipes_by_name: HashMap<String, EncodingRecipeNumber>,
    pub inst_pred_reg: InstructionPredicateRegistry,
}

impl PerCpuModeEncodings {
    fn new() -> Self {
        Self {
            enc32: Vec::new(),
            enc64: Vec::new(),
            recipes: Recipes::new(),
            recipes_by_name: HashMap::new(),
            inst_pred_reg: InstructionPredicateRegistry::new(),
        }
    }

    fn add_recipe(&mut self, recipe: EncodingRecipe) -> EncodingRecipeNumber {
        if let Some(found_index) = self.recipes_by_name.get(&recipe.name) {
            assert!(
                self.recipes[*found_index] == recipe,
                format!(
                    "trying to insert different recipes with a same name ({})",
                    recipe.name
                )
            );
            *found_index
        } else {
            let recipe_name = recipe.name.clone();
            let index = self.recipes.push(recipe);
            self.recipes_by_name.insert(recipe_name, index);
            index
        }
    }

    fn make_encoding<T>(
        &mut self,
        inst: InstSpec,
        template: Template,
        builder_closure: T,
    ) -> Encoding
    where
        T: FnOnce(EncodingBuilder) -> EncodingBuilder,
    {
        let (recipe, bits) = template.build();
        let recipe_number = self.add_recipe(recipe);
        let builder = EncodingBuilder::new(inst, recipe_number, bits);
        builder_closure(builder).build(&self.recipes, &mut self.inst_pred_reg)
    }

    fn enc32_func<T>(&mut self, inst: impl Into<InstSpec>, template: Template, builder_closure: T)
    where
        T: FnOnce(EncodingBuilder) -> EncodingBuilder,
    {
        let encoding = self.make_encoding(inst.into(), template, builder_closure);
        self.enc32.push(encoding);
    }
    fn enc32(&mut self, inst: impl Into<InstSpec>, template: Template) {
        self.enc32_func(inst, template, |x| x);
    }
    fn enc32_isap(
        &mut self,
        inst: impl Into<InstSpec>,
        template: Template,
        isap: SettingPredicateNumber,
    ) {
        self.enc32_func(inst, template, |encoding| encoding.isa_predicate(isap));
    }
    fn enc32_instp(
        &mut self,
        inst: impl Into<InstSpec>,
        template: Template,
        instp: InstructionPredicateNode,
    ) {
        self.enc32_func(inst, template, |encoding| encoding.inst_predicate(instp));
    }
    fn enc32_rec(&mut self, inst: impl Into<InstSpec>, recipe: &EncodingRecipe, bits: u16) {
        let recipe_number = self.add_recipe(recipe.clone());
        let builder = EncodingBuilder::new(inst.into(), recipe_number, bits);
        let encoding = builder.build(&self.recipes, &mut self.inst_pred_reg);
        self.enc32.push(encoding);
    }

    fn enc64_func<T>(&mut self, inst: impl Into<InstSpec>, template: Template, builder_closure: T)
    where
        T: FnOnce(EncodingBuilder) -> EncodingBuilder,
    {
        let encoding = self.make_encoding(inst.into(), template, builder_closure);
        self.enc64.push(encoding);
    }
    fn enc64(&mut self, inst: impl Into<InstSpec>, template: Template) {
        self.enc64_func(inst, template, |x| x);
    }
    fn enc64_isap(
        &mut self,
        inst: impl Into<InstSpec>,
        template: Template,
        isap: SettingPredicateNumber,
    ) {
        self.enc64_func(inst, template, |encoding| encoding.isa_predicate(isap));
    }
    fn enc64_instp(
        &mut self,
        inst: impl Into<InstSpec>,
        template: Template,
        instp: InstructionPredicateNode,
    ) {
        self.enc64_func(inst, template, |encoding| encoding.inst_predicate(instp));
    }
    fn enc64_rec(&mut self, inst: impl Into<InstSpec>, recipe: &EncodingRecipe, bits: u16) {
        let recipe_number = self.add_recipe(recipe.clone());
        let builder = EncodingBuilder::new(inst.into(), recipe_number, bits);
        let encoding = builder.build(&self.recipes, &mut self.inst_pred_reg);
        self.enc64.push(encoding);
    }

    /// Adds I32/I64 encodings as appropriate for a typed instruction.
    /// The REX prefix is always inferred at runtime.
    ///
    /// Add encodings for `inst.i32` to X86_32.
    /// Add encodings for `inst.i32` to X86_64 with optional, inferred REX.
    /// Add encodings for `inst.i64` to X86_64 with a REX.W prefix.
    fn enc_i32_i64(&mut self, inst: impl Into<InstSpec>, template: Template) {
        let inst: InstSpec = inst.into();

        // I32 on x86: no REX prefix.
        self.enc32(inst.bind(I32), template.infer_rex());

        // I32 on x86_64: REX.W unset; REX.RXB determined at runtime from registers.
        self.enc64(inst.bind(I32), template.infer_rex());

        // I64 on x86_64: REX.W set; REX.RXB determined at runtime from registers.
        self.enc64(inst.bind(I64), template.rex().w());
    }

    /// Adds I32/I64 encodings as appropriate for a typed instruction.
    /// All variants of REX prefix are explicitly emitted, not inferred.
    ///
    /// Add encodings for `inst.i32` to X86_32.
    /// Add encodings for `inst.i32` to X86_64 with and without REX.
    /// Add encodings for `inst.i64` to X86_64 with and without REX.
    fn enc_i32_i64_explicit_rex(&mut self, inst: impl Into<InstSpec>, template: Template) {
        let inst: InstSpec = inst.into();
        self.enc32(inst.bind(I32), template.nonrex());

        // REX-less encoding must come after REX encoding so we don't use it by default.
        // Otherwise reg-alloc would never use r8 and up.
        self.enc64(inst.bind(I32), template.rex());
        self.enc64(inst.bind(I32), template.nonrex());
        self.enc64(inst.bind(I64), template.rex().w());
    }

    /// Adds B32/B64 encodings as appropriate for a typed instruction.
    /// The REX prefix is always inferred at runtime.
    ///
    /// Adds encoding for `inst.b32` to X86_32.
    /// Adds encoding for `inst.b32` to X86_64 with optional, inferred REX.
    /// Adds encoding for `inst.b64` to X86_64 with a REX.W prefix.
    fn enc_b32_b64(&mut self, inst: impl Into<InstSpec>, template: Template) {
        let inst: InstSpec = inst.into();

        // B32 on x86: no REX prefix.
        self.enc32(inst.bind(B32), template.infer_rex());

        // B32 on x86_64: REX.W unset; REX.RXB determined at runtime from registers.
        self.enc64(inst.bind(B32), template.infer_rex());

        // B64 on x86_64: REX.W set; REX.RXB determined at runtime from registers.
        self.enc64(inst.bind(B64), template.rex().w());
    }

    /// Add encodings for `inst.i32` to X86_32.
    /// Add encodings for `inst.i32` to X86_64 with a REX prefix.
    /// Add encodings for `inst.i64` to X86_64 with a REX.W prefix.
    fn enc_i32_i64_rex_only(&mut self, inst: impl Into<InstSpec>, template: Template) {
        let inst: InstSpec = inst.into();
        self.enc32(inst.bind(I32), template.nonrex());
        self.enc64(inst.bind(I32), template.rex());
        self.enc64(inst.bind(I64), template.rex().w());
    }

    /// Add encodings for `inst.i32` to X86_32.
    /// Add encodings for `inst.i32` to X86_64 with and without REX.
    /// Add encodings for `inst.i64` to X86_64 with a REX.W prefix.
    fn enc_i32_i64_instp(
        &mut self,
        inst: &Instruction,
        template: Template,
        instp: InstructionPredicateNode,
    ) {
        self.enc32_func(inst.bind(I32), template.nonrex(), |builder| {
            builder.inst_predicate(instp.clone())
        });

        // REX-less encoding must come after REX encoding so we don't use it by default. Otherwise
        // reg-alloc would never use r8 and up.
        self.enc64_func(inst.bind(I32), template.rex(), |builder| {
            builder.inst_predicate(instp.clone())
        });
        self.enc64_func(inst.bind(I32), template.nonrex(), |builder| {
            builder.inst_predicate(instp.clone())
        });
        self.enc64_func(inst.bind(I64), template.rex().w(), |builder| {
            builder.inst_predicate(instp)
        });
    }

    /// Add encodings for `inst.r32` to X86_32.
    /// Add encodings for `inst.r64` to X86_64 with a REX.W prefix.
    fn enc_r32_r64_rex_only(&mut self, inst: impl Into<InstSpec>, template: Template) {
        let inst: InstSpec = inst.into();
        self.enc32(inst.bind(R32), template.nonrex());
        self.enc64(inst.bind(R64), template.rex().w());
    }

    fn enc_r32_r64_ld_st(&mut self, inst: &Instruction, w_bit: bool, template: Template) {
        self.enc32(inst.clone().bind(R32).bind(Any), template.clone());

        // REX-less encoding must come after REX encoding so we don't use it by
        // default. Otherwise reg-alloc would never use r8 and up.
        self.enc64(inst.clone().bind(R32).bind(Any), template.clone().rex());
        self.enc64(inst.clone().bind(R32).bind(Any), template.clone());

        if w_bit {
            self.enc64(inst.clone().bind(R64).bind(Any), template.rex().w());
        } else {
            self.enc64(inst.clone().bind(R64).bind(Any), template.clone().rex());
            self.enc64(inst.clone().bind(R64).bind(Any), template);
        }
    }

    /// Add encodings for `inst` to X86_64 with and without a REX prefix.
    fn enc_x86_64(&mut self, inst: impl Into<InstSpec> + Clone, template: Template) {
        // See above comment about the ordering of rex vs non-rex encodings.
        self.enc64(inst.clone(), template.rex());
        self.enc64(inst, template);
    }

    /// Add encodings for `inst` to X86_64 with and without a REX prefix.
    fn enc_x86_64_instp(
        &mut self,
        inst: impl Clone + Into<InstSpec>,
        template: Template,
        instp: InstructionPredicateNode,
    ) {
        // See above comment about the ordering of rex vs non-rex encodings.
        self.enc64_func(inst.clone(), template.rex(), |builder| {
            builder.inst_predicate(instp.clone())
        });
        self.enc64_func(inst, template, |builder| builder.inst_predicate(instp));
    }
    fn enc_x86_64_isap(
        &mut self,
        inst: impl Clone + Into<InstSpec>,
        template: Template,
        isap: SettingPredicateNumber,
    ) {
        // See above comment about the ordering of rex vs non-rex encodings.
        self.enc64_isap(inst.clone(), template.rex(), isap);
        self.enc64_isap(inst, template, isap);
    }

    /// Add all three encodings for `inst`:
    /// - X86_32
    /// - X86_64 with and without the REX prefix.
    fn enc_both(&mut self, inst: impl Clone + Into<InstSpec>, template: Template) {
        self.enc32(inst.clone(), template.clone());
        self.enc_x86_64(inst, template);
    }
    fn enc_both_isap(
        &mut self,
        inst: impl Clone + Into<InstSpec>,
        template: Template,
        isap: SettingPredicateNumber,
    ) {
        self.enc32_isap(inst.clone(), template.clone(), isap);
        self.enc_x86_64_isap(inst, template, isap);
    }
    fn enc_both_instp(
        &mut self,
        inst: impl Clone + Into<InstSpec>,
        template: Template,
        instp: InstructionPredicateNode,
    ) {
        self.enc32_instp(inst.clone(), template.clone(), instp.clone());
        self.enc_x86_64_instp(inst, template, instp);
    }

    /// Add two encodings for `inst`:
    /// - X86_32, no REX prefix, since this is not valid in 32-bit mode.
    /// - X86_64, dynamically infer the REX prefix.
    fn enc_both_inferred(&mut self, inst: impl Clone + Into<InstSpec>, template: Template) {
        self.enc32(inst.clone(), template.clone());
        self.enc64(inst, template.infer_rex());
    }
    fn enc_both_inferred_maybe_isap(
        &mut self,
        inst: impl Clone + Into<InstSpec>,
        template: Template,
        isap: Option<SettingPredicateNumber>,
    ) {
        self.enc32_maybe_isap(inst.clone(), template.clone(), isap);
        self.enc64_maybe_isap(inst, template.infer_rex(), isap);
    }

    /// Add two encodings for `inst`:
    /// - X86_32
    /// - X86_64 with the REX prefix.
    fn enc_both_rex_only(&mut self, inst: impl Clone + Into<InstSpec>, template: Template) {
        self.enc32(inst.clone(), template.clone());
        self.enc64(inst, template.rex());
    }

    /// Add encodings for `inst.i32` to X86_32.
    /// Add encodings for `inst.i32` to X86_64 with and without REX.
    /// Add encodings for `inst.i64` to X86_64 with a REX prefix, using the `w_bit`
    /// argument to determine whether or not to set the REX.W bit.
    fn enc_i32_i64_ld_st(&mut self, inst: &Instruction, w_bit: bool, template: Template) {
        self.enc32(inst.clone().bind(I32).bind(Any), template.clone());

        // REX-less encoding must come after REX encoding so we don't use it by
        // default. Otherwise reg-alloc would never use r8 and up.
        self.enc64(inst.clone().bind(I32).bind(Any), template.clone().rex());
        self.enc64(inst.clone().bind(I32).bind(Any), template.clone());

        if w_bit {
            self.enc64(inst.clone().bind(I64).bind(Any), template.rex().w());
        } else {
            self.enc64(inst.clone().bind(I64).bind(Any), template.clone().rex());
            self.enc64(inst.clone().bind(I64).bind(Any), template);
        }
    }

    /// Add the same encoding/recipe pairing to both X86_32 and X86_64
    fn enc_32_64_rec(
        &mut self,
        inst: impl Clone + Into<InstSpec>,
        recipe: &EncodingRecipe,
        bits: u16,
    ) {
        self.enc32_rec(inst.clone(), recipe, bits);
        self.enc64_rec(inst, recipe, bits);
    }

    /// Add the same encoding to both X86_32 and X86_64; assumes configuration (e.g. REX, operand binding) has already happened
    fn enc_32_64_func<T>(
        &mut self,
        inst: impl Clone + Into<InstSpec>,
        template: Template,
        builder_closure: T,
    ) where
        T: FnOnce(EncodingBuilder) -> EncodingBuilder,
    {
        let encoding = self.make_encoding(inst.into(), template, builder_closure);
        self.enc32.push(encoding.clone());
        self.enc64.push(encoding);
    }

    /// Add the same encoding to both X86_32 and X86_64; assumes configuration (e.g. REX, operand
    /// binding) has already happened.
    fn enc_32_64_maybe_isap(
        &mut self,
        inst: impl Clone + Into<InstSpec>,
        template: Template,
        isap: Option<SettingPredicateNumber>,
    ) {
        self.enc32_maybe_isap(inst.clone(), template.clone(), isap);
        self.enc64_maybe_isap(inst, template, isap);
    }

    fn enc32_maybe_isap(
        &mut self,
        inst: impl Into<InstSpec>,
        template: Template,
        isap: Option<SettingPredicateNumber>,
    ) {
        match isap {
            None => self.enc32(inst, template),
            Some(isap) => self.enc32_isap(inst, template, isap),
        }
    }

    fn enc64_maybe_isap(
        &mut self,
        inst: impl Into<InstSpec>,
        template: Template,
        isap: Option<SettingPredicateNumber>,
    ) {
        match isap {
            None => self.enc64(inst, template),
            Some(isap) => self.enc64_isap(inst, template, isap),
        }
    }
}

// Definitions.

#[inline(never)]
fn define_moves(e: &mut PerCpuModeEncodings, shared_defs: &SharedDefinitions, r: &RecipeGroup) {
    let shared = &shared_defs.instructions;
    let formats = &shared_defs.formats;

    // Shorthands for instructions.
    let bconst = shared.by_name("bconst");
    let bint = shared.by_name("bint");
    let copy = shared.by_name("copy");
    let copy_special = shared.by_name("copy_special");
    let copy_to_ssa = shared.by_name("copy_to_ssa");
    let get_pinned_reg = shared.by_name("get_pinned_reg");
    let iconst = shared.by_name("iconst");
    let ireduce = shared.by_name("ireduce");
    let regmove = shared.by_name("regmove");
    let sextend = shared.by_name("sextend");
    let set_pinned_reg = shared.by_name("set_pinned_reg");
    let uextend = shared.by_name("uextend");

    // Shorthands for recipes.
    let rec_copysp = r.template("copysp");
    let rec_furm_reg_to_ssa = r.template("furm_reg_to_ssa");
    let rec_get_pinned_reg = r.recipe("get_pinned_reg");
    let rec_null = r.recipe("null");
    let rec_pu_id = r.template("pu_id");
    let rec_pu_id_bool = r.template("pu_id_bool");
    let rec_pu_iq = r.template("pu_iq");
    let rec_rmov = r.template("rmov");
    let rec_set_pinned_reg = r.template("set_pinned_reg");
    let rec_u_id = r.template("u_id");
    let rec_u_id_z = r.template("u_id_z");
    let rec_umr = r.template("umr");
    let rec_umr_reg_to_ssa = r.template("umr_reg_to_ssa");
    let rec_urm_noflags = r.template("urm_noflags");
    let rec_urm_noflags_abcd = r.template("urm_noflags_abcd");

    // The pinned reg is fixed to a certain value entirely user-controlled, so it generates nothing!
    e.enc64_rec(get_pinned_reg.bind(I64), rec_get_pinned_reg, 0);
    e.enc_x86_64(
        set_pinned_reg.bind(I64),
        rec_set_pinned_reg.opcodes(&MOV_STORE).rex().w(),
    );

    e.enc_i32_i64(copy, rec_umr.opcodes(&MOV_STORE));
    e.enc_r32_r64_rex_only(copy, rec_umr.opcodes(&MOV_STORE));
    e.enc_both(copy.bind(B1), rec_umr.opcodes(&MOV_STORE));
    e.enc_both(copy.bind(I8), rec_umr.opcodes(&MOV_STORE));
    e.enc_both(copy.bind(I16), rec_umr.opcodes(&MOV_STORE));

    // TODO For x86-64, only define REX forms for now, since we can't describe the
    // special regunit immediate operands with the current constraint language.
    for &ty in &[I8, I16, I32] {
        e.enc32(regmove.bind(ty), rec_rmov.opcodes(&MOV_STORE));
        e.enc64(regmove.bind(ty), rec_rmov.opcodes(&MOV_STORE).rex());
    }
    for &ty in &[B8, B16, B32] {
        e.enc32(regmove.bind(ty), rec_rmov.opcodes(&MOV_STORE));
        e.enc64(regmove.bind(ty), rec_rmov.opcodes(&MOV_STORE).rex());
    }
    e.enc64(regmove.bind(I64), rec_rmov.opcodes(&MOV_STORE).rex().w());
    e.enc_both(regmove.bind(B1), rec_rmov.opcodes(&MOV_STORE));
    e.enc_both(regmove.bind(I8), rec_rmov.opcodes(&MOV_STORE));
    e.enc32(regmove.bind(R32), rec_rmov.opcodes(&MOV_STORE));
    e.enc64(regmove.bind(R32), rec_rmov.opcodes(&MOV_STORE).rex());
    e.enc64(regmove.bind(R64), rec_rmov.opcodes(&MOV_STORE).rex().w());

    // Immediate constants.
    e.enc32(iconst.bind(I32), rec_pu_id.opcodes(&MOV_IMM));

    e.enc64(iconst.bind(I32), rec_pu_id.rex().opcodes(&MOV_IMM));
    e.enc64(iconst.bind(I32), rec_pu_id.opcodes(&MOV_IMM));

    // The 32-bit immediate movl also zero-extends to 64 bits.
    let is_unsigned_int32 =
        InstructionPredicate::new_is_unsigned_int(&*formats.unary_imm, "imm", 32, 0);

    e.enc64_func(
        iconst.bind(I64),
        rec_pu_id.opcodes(&MOV_IMM).rex(),
        |encoding| encoding.inst_predicate(is_unsigned_int32.clone()),
    );
    e.enc64_func(iconst.bind(I64), rec_pu_id.opcodes(&MOV_IMM), |encoding| {
        encoding.inst_predicate(is_unsigned_int32)
    });

    // Sign-extended 32-bit immediate.
    e.enc64(
        iconst.bind(I64),
        rec_u_id.rex().opcodes(&MOV_IMM_SIGNEXTEND).rrr(0).w(),
    );

    // Finally, the MOV_IMM opcode takes an 8-byte immediate with a REX.W prefix.
    e.enc64(iconst.bind(I64), rec_pu_iq.opcodes(&MOV_IMM).rex().w());

    // Bool constants (uses MOV)
    for &ty in &[B1, B8, B16, B32] {
        e.enc_both(bconst.bind(ty), rec_pu_id_bool.opcodes(&MOV_IMM));
    }
    e.enc64(bconst.bind(B64), rec_pu_id_bool.opcodes(&MOV_IMM).rex());

    let is_zero_int = InstructionPredicate::new_is_zero_int(&formats.unary_imm, "imm");
    e.enc_both_instp(
        iconst.bind(I8),
        rec_u_id_z.opcodes(&XORB),
        is_zero_int.clone(),
    );

    // You may expect that i16 encodings would have an 0x66 prefix on the opcode to indicate that
    // encodings should be on 16-bit operands (f.ex, "xor %ax, %ax"). Cranelift currently does not
    // know that it can drop the 0x66 prefix and clear the upper half of a 32-bit register in these
    // scenarios, so we explicitly select a wider but permissible opcode.
    //
    // This effectively formalizes the i16->i32 widening that Cranelift performs when there isn't
    // an appropriate i16 encoding available.
    e.enc_both_instp(
        iconst.bind(I16),
        rec_u_id_z.opcodes(&XOR),
        is_zero_int.clone(),
    );
    e.enc_both_instp(
        iconst.bind(I32),
        rec_u_id_z.opcodes(&XOR),
        is_zero_int.clone(),
    );
    e.enc_x86_64_instp(iconst.bind(I64), rec_u_id_z.opcodes(&XOR), is_zero_int);

    // Numerical conversions.

    // Reducing an integer is a no-op.
    e.enc32_rec(ireduce.bind(I8).bind(I16), rec_null, 0);
    e.enc32_rec(ireduce.bind(I8).bind(I32), rec_null, 0);
    e.enc32_rec(ireduce.bind(I16).bind(I32), rec_null, 0);

    e.enc64_rec(ireduce.bind(I8).bind(I16), rec_null, 0);
    e.enc64_rec(ireduce.bind(I8).bind(I32), rec_null, 0);
    e.enc64_rec(ireduce.bind(I16).bind(I32), rec_null, 0);
    e.enc64_rec(ireduce.bind(I8).bind(I64), rec_null, 0);
    e.enc64_rec(ireduce.bind(I16).bind(I64), rec_null, 0);
    e.enc64_rec(ireduce.bind(I32).bind(I64), rec_null, 0);

    // TODO: Add encodings for cbw, cwde, cdqe, which are sign-extending
    // instructions for %al/%ax/%eax to %ax/%eax/%rax.

    // movsbl
    e.enc32(
        sextend.bind(I32).bind(I8),
        rec_urm_noflags_abcd.opcodes(&MOVSX_BYTE),
    );
    e.enc64(
        sextend.bind(I32).bind(I8),
        rec_urm_noflags.opcodes(&MOVSX_BYTE).rex(),
    );
    e.enc64(
        sextend.bind(I32).bind(I8),
        rec_urm_noflags_abcd.opcodes(&MOVSX_BYTE),
    );

    // movswl
    e.enc32(
        sextend.bind(I32).bind(I16),
        rec_urm_noflags.opcodes(&MOVSX_WORD),
    );
    e.enc64(
        sextend.bind(I32).bind(I16),
        rec_urm_noflags.opcodes(&MOVSX_WORD).rex(),
    );
    e.enc64(
        sextend.bind(I32).bind(I16),
        rec_urm_noflags.opcodes(&MOVSX_WORD),
    );

    // movsbq
    e.enc64(
        sextend.bind(I64).bind(I8),
        rec_urm_noflags.opcodes(&MOVSX_BYTE).rex().w(),
    );

    // movswq
    e.enc64(
        sextend.bind(I64).bind(I16),
        rec_urm_noflags.opcodes(&MOVSX_WORD).rex().w(),
    );

    // movslq
    e.enc64(
        sextend.bind(I64).bind(I32),
        rec_urm_noflags.opcodes(&MOVSXD).rex().w(),
    );

    // movzbl
    e.enc32(
        uextend.bind(I32).bind(I8),
        rec_urm_noflags_abcd.opcodes(&MOVZX_BYTE),
    );
    e.enc64(
        uextend.bind(I32).bind(I8),
        rec_urm_noflags.opcodes(&MOVZX_BYTE).rex(),
    );
    e.enc64(
        uextend.bind(I32).bind(I8),
        rec_urm_noflags_abcd.opcodes(&MOVZX_BYTE),
    );

    // movzwl
    e.enc32(
        uextend.bind(I32).bind(I16),
        rec_urm_noflags.opcodes(&MOVZX_WORD),
    );
    e.enc64(
        uextend.bind(I32).bind(I16),
        rec_urm_noflags.opcodes(&MOVZX_WORD).rex(),
    );
    e.enc64(
        uextend.bind(I32).bind(I16),
        rec_urm_noflags.opcodes(&MOVZX_WORD),
    );

    // movzbq, encoded as movzbl because it's equivalent and shorter.
    e.enc64(
        uextend.bind(I64).bind(I8),
        rec_urm_noflags.opcodes(&MOVZX_BYTE).rex(),
    );
    e.enc64(
        uextend.bind(I64).bind(I8),
        rec_urm_noflags_abcd.opcodes(&MOVZX_BYTE),
    );

    // movzwq, encoded as movzwl because it's equivalent and shorter
    e.enc64(
        uextend.bind(I64).bind(I16),
        rec_urm_noflags.opcodes(&MOVZX_WORD).rex(),
    );
    e.enc64(
        uextend.bind(I64).bind(I16),
        rec_urm_noflags.opcodes(&MOVZX_WORD),
    );

    // A 32-bit register copy clears the high 32 bits.
    e.enc64(
        uextend.bind(I64).bind(I32),
        rec_umr.opcodes(&MOV_STORE).rex(),
    );
    e.enc64(uextend.bind(I64).bind(I32), rec_umr.opcodes(&MOV_STORE));

    // Convert bool to int.
    //
    // This assumes that b1 is represented as an 8-bit low register with the value 0
    // or 1.
    //
    // Encode movzbq as movzbl, because it's equivalent and shorter.
    for &to in &[I8, I16, I32, I64] {
        for &from in &[B1, B8] {
            e.enc64(
                bint.bind(to).bind(from),
                rec_urm_noflags.opcodes(&MOVZX_BYTE).rex(),
            );
            e.enc64(
                bint.bind(to).bind(from),
                rec_urm_noflags_abcd.opcodes(&MOVZX_BYTE),
            );
            if to != I64 {
                e.enc32(
                    bint.bind(to).bind(from),
                    rec_urm_noflags_abcd.opcodes(&MOVZX_BYTE),
                );
            }
        }
    }
    for (to, from) in &[(I16, B16), (I32, B32), (I64, B64)] {
        e.enc_both(
            bint.bind(*to).bind(*from),
            rec_urm_noflags_abcd.opcodes(&MOVZX_BYTE),
        );
    }

    // Copy Special
    // For x86-64, only define REX forms for now, since we can't describe the
    // special regunit immediate operands with the current constraint language.
    e.enc64(copy_special, rec_copysp.opcodes(&MOV_STORE).rex().w());
    e.enc32(copy_special, rec_copysp.opcodes(&MOV_STORE));

    // Copy to SSA.  These have to be done with special _rex_only encoders, because the standard
    // machinery for deciding whether a REX.{RXB} prefix is needed doesn't take into account
    // the source register, which is specified directly in the instruction.
    e.enc_i32_i64_rex_only(copy_to_ssa, rec_umr_reg_to_ssa.opcodes(&MOV_STORE));
    e.enc_r32_r64_rex_only(copy_to_ssa, rec_umr_reg_to_ssa.opcodes(&MOV_STORE));
    e.enc_both_rex_only(copy_to_ssa.bind(B1), rec_umr_reg_to_ssa.opcodes(&MOV_STORE));
    e.enc_both_rex_only(copy_to_ssa.bind(I8), rec_umr_reg_to_ssa.opcodes(&MOV_STORE));
    e.enc_both_rex_only(
        copy_to_ssa.bind(I16),
        rec_umr_reg_to_ssa.opcodes(&MOV_STORE),
    );
    e.enc_both_rex_only(
        copy_to_ssa.bind(F64),
        rec_furm_reg_to_ssa.opcodes(&MOVSD_LOAD),
    );
    e.enc_both_rex_only(
        copy_to_ssa.bind(F32),
        rec_furm_reg_to_ssa.opcodes(&MOVSS_LOAD),
    );
}

#[inline(never)]
fn define_memory(
    e: &mut PerCpuModeEncodings,
    shared_defs: &SharedDefinitions,
    x86: &InstructionGroup,
    r: &RecipeGroup,
) {
    let shared = &shared_defs.instructions;
    let formats = &shared_defs.formats;

    // Shorthands for instructions.
    let adjust_sp_down = shared.by_name("adjust_sp_down");
    let adjust_sp_down_imm = shared.by_name("adjust_sp_down_imm");
    let adjust_sp_up_imm = shared.by_name("adjust_sp_up_imm");
    let copy_nop = shared.by_name("copy_nop");
    let fill = shared.by_name("fill");
    let fill_nop = shared.by_name("fill_nop");
    let istore16 = shared.by_name("istore16");
    let istore16_complex = shared.by_name("istore16_complex");
    let istore32 = shared.by_name("istore32");
    let istore32_complex = shared.by_name("istore32_complex");
    let istore8 = shared.by_name("istore8");
    let istore8_complex = shared.by_name("istore8_complex");
    let load = shared.by_name("load");
    let load_complex = shared.by_name("load_complex");
    let regfill = shared.by_name("regfill");
    let regspill = shared.by_name("regspill");
    let sload16 = shared.by_name("sload16");
    let sload16_complex = shared.by_name("sload16_complex");
    let sload32 = shared.by_name("sload32");
    let sload32_complex = shared.by_name("sload32_complex");
    let sload8 = shared.by_name("sload8");
    let sload8_complex = shared.by_name("sload8_complex");
    let spill = shared.by_name("spill");
    let store = shared.by_name("store");
    let store_complex = shared.by_name("store_complex");
    let uload16 = shared.by_name("uload16");
    let uload16_complex = shared.by_name("uload16_complex");
    let uload32 = shared.by_name("uload32");
    let uload32_complex = shared.by_name("uload32_complex");
    let uload8 = shared.by_name("uload8");
    let uload8_complex = shared.by_name("uload8_complex");
    let x86_pop = x86.by_name("x86_pop");
    let x86_push = x86.by_name("x86_push");

    // Shorthands for recipes.
    let rec_adjustsp = r.template("adjustsp");
    let rec_adjustsp_ib = r.template("adjustsp_ib");
    let rec_adjustsp_id = r.template("adjustsp_id");
    let rec_ffillnull = r.recipe("ffillnull");
    let rec_fillnull = r.recipe("fillnull");
    let rec_fillSib32 = r.template("fillSib32");
    let rec_ld = r.template("ld");
    let rec_ldDisp32 = r.template("ldDisp32");
    let rec_ldDisp8 = r.template("ldDisp8");
    let rec_ldWithIndex = r.template("ldWithIndex");
    let rec_ldWithIndexDisp32 = r.template("ldWithIndexDisp32");
    let rec_ldWithIndexDisp8 = r.template("ldWithIndexDisp8");
    let rec_popq = r.template("popq");
    let rec_pushq = r.template("pushq");
    let rec_regfill32 = r.template("regfill32");
    let rec_regspill32 = r.template("regspill32");
    let rec_spillSib32 = r.template("spillSib32");
    let rec_st = r.template("st");
    let rec_stacknull = r.recipe("stacknull");
    let rec_stDisp32 = r.template("stDisp32");
    let rec_stDisp32_abcd = r.template("stDisp32_abcd");
    let rec_stDisp8 = r.template("stDisp8");
    let rec_stDisp8_abcd = r.template("stDisp8_abcd");
    let rec_stWithIndex = r.template("stWithIndex");
    let rec_stWithIndexDisp32 = r.template("stWithIndexDisp32");
    let rec_stWithIndexDisp32_abcd = r.template("stWithIndexDisp32_abcd");
    let rec_stWithIndexDisp8 = r.template("stWithIndexDisp8");
    let rec_stWithIndexDisp8_abcd = r.template("stWithIndexDisp8_abcd");
    let rec_stWithIndex_abcd = r.template("stWithIndex_abcd");
    let rec_st_abcd = r.template("st_abcd");

    // Loads and stores.
    let is_load_complex_length_two =
        InstructionPredicate::new_length_equals(&*formats.load_complex, 2);

    for recipe in &[rec_ldWithIndex, rec_ldWithIndexDisp8, rec_ldWithIndexDisp32] {
        e.enc_i32_i64_instp(
            load_complex,
            recipe.opcodes(&MOV_LOAD),
            is_load_complex_length_two.clone(),
        );
        e.enc_x86_64_instp(
            uload32_complex,
            recipe.opcodes(&MOV_LOAD),
            is_load_complex_length_two.clone(),
        );

        e.enc64_instp(
            sload32_complex,
            recipe.opcodes(&MOVSXD).rex().w(),
            is_load_complex_length_two.clone(),
        );

        e.enc_i32_i64_instp(
            uload16_complex,
            recipe.opcodes(&MOVZX_WORD),
            is_load_complex_length_two.clone(),
        );
        e.enc_i32_i64_instp(
            sload16_complex,
            recipe.opcodes(&MOVSX_WORD),
            is_load_complex_length_two.clone(),
        );

        e.enc_i32_i64_instp(
            uload8_complex,
            recipe.opcodes(&MOVZX_BYTE),
            is_load_complex_length_two.clone(),
        );

        e.enc_i32_i64_instp(
            sload8_complex,
            recipe.opcodes(&MOVSX_BYTE),
            is_load_complex_length_two.clone(),
        );
    }

    let is_store_complex_length_three =
        InstructionPredicate::new_length_equals(&*formats.store_complex, 3);

    for recipe in &[rec_stWithIndex, rec_stWithIndexDisp8, rec_stWithIndexDisp32] {
        e.enc_i32_i64_instp(
            store_complex,
            recipe.opcodes(&MOV_STORE),
            is_store_complex_length_three.clone(),
        );
        e.enc_x86_64_instp(
            istore32_complex,
            recipe.opcodes(&MOV_STORE),
            is_store_complex_length_three.clone(),
        );
        e.enc_both_instp(
            istore16_complex.bind(I32),
            recipe.opcodes(&MOV_STORE_16),
            is_store_complex_length_three.clone(),
        );
        e.enc_x86_64_instp(
            istore16_complex.bind(I64),
            recipe.opcodes(&MOV_STORE_16),
            is_store_complex_length_three.clone(),
        );
    }

    for recipe in &[
        rec_stWithIndex_abcd,
        rec_stWithIndexDisp8_abcd,
        rec_stWithIndexDisp32_abcd,
    ] {
        e.enc_both_instp(
            istore8_complex.bind(I32),
            recipe.opcodes(&MOV_BYTE_STORE),
            is_store_complex_length_three.clone(),
        );
        e.enc_x86_64_instp(
            istore8_complex.bind(I64),
            recipe.opcodes(&MOV_BYTE_STORE),
            is_store_complex_length_three.clone(),
        );
    }

    for recipe in &[rec_st, rec_stDisp8, rec_stDisp32] {
        e.enc_i32_i64_ld_st(store, true, recipe.opcodes(&MOV_STORE));
        e.enc_r32_r64_ld_st(store, true, recipe.opcodes(&MOV_STORE));
        e.enc_x86_64(istore32.bind(I64).bind(Any), recipe.opcodes(&MOV_STORE));
        e.enc_i32_i64_ld_st(istore16, false, recipe.opcodes(&MOV_STORE_16));
    }

    // Byte stores are more complicated because the registers they can address
    // depends of the presence of a REX prefix. The st*_abcd recipes fall back to
    // the corresponding st* recipes when a REX prefix is applied.

    for recipe in &[rec_st_abcd, rec_stDisp8_abcd, rec_stDisp32_abcd] {
        e.enc_both(istore8.bind(I32).bind(Any), recipe.opcodes(&MOV_BYTE_STORE));
        e.enc_x86_64(istore8.bind(I64).bind(Any), recipe.opcodes(&MOV_BYTE_STORE));
    }

    e.enc_i32_i64_explicit_rex(spill, rec_spillSib32.opcodes(&MOV_STORE));
    e.enc_i32_i64_explicit_rex(regspill, rec_regspill32.opcodes(&MOV_STORE));
    e.enc_r32_r64_rex_only(spill, rec_spillSib32.opcodes(&MOV_STORE));
    e.enc_r32_r64_rex_only(regspill, rec_regspill32.opcodes(&MOV_STORE));

    // Use a 32-bit write for spilling `b1`, `i8` and `i16` to avoid
    // constraining the permitted registers.
    // See MIN_SPILL_SLOT_SIZE which makes this safe.

    e.enc_both(spill.bind(B1), rec_spillSib32.opcodes(&MOV_STORE));
    e.enc_both(regspill.bind(B1), rec_regspill32.opcodes(&MOV_STORE));
    for &ty in &[I8, I16] {
        e.enc_both(spill.bind(ty), rec_spillSib32.opcodes(&MOV_STORE));
        e.enc_both(regspill.bind(ty), rec_regspill32.opcodes(&MOV_STORE));
    }

    for recipe in &[rec_ld, rec_ldDisp8, rec_ldDisp32] {
        e.enc_i32_i64_ld_st(load, true, recipe.opcodes(&MOV_LOAD));
        e.enc_r32_r64_ld_st(load, true, recipe.opcodes(&MOV_LOAD));
        e.enc_x86_64(uload32.bind(I64), recipe.opcodes(&MOV_LOAD));
        e.enc64(sload32.bind(I64), recipe.opcodes(&MOVSXD).rex().w());
        e.enc_i32_i64_ld_st(uload16, true, recipe.opcodes(&MOVZX_WORD));
        e.enc_i32_i64_ld_st(sload16, true, recipe.opcodes(&MOVSX_WORD));
        e.enc_i32_i64_ld_st(uload8, true, recipe.opcodes(&MOVZX_BYTE));
        e.enc_i32_i64_ld_st(sload8, true, recipe.opcodes(&MOVSX_BYTE));
    }

    e.enc_i32_i64_explicit_rex(fill, rec_fillSib32.opcodes(&MOV_LOAD));
    e.enc_i32_i64_explicit_rex(regfill, rec_regfill32.opcodes(&MOV_LOAD));
    e.enc_r32_r64_rex_only(fill, rec_fillSib32.opcodes(&MOV_LOAD));
    e.enc_r32_r64_rex_only(regfill, rec_regfill32.opcodes(&MOV_LOAD));

    // No-op fills, created by late-stage redundant-fill removal.
    for &ty in &[I64, I32, I16, I8] {
        e.enc64_rec(fill_nop.bind(ty), rec_fillnull, 0);
        e.enc32_rec(fill_nop.bind(ty), rec_fillnull, 0);
    }
    e.enc64_rec(fill_nop.bind(B1), rec_fillnull, 0);
    e.enc32_rec(fill_nop.bind(B1), rec_fillnull, 0);
    for &ty in &[F64, F32] {
        e.enc64_rec(fill_nop.bind(ty), rec_ffillnull, 0);
        e.enc32_rec(fill_nop.bind(ty), rec_ffillnull, 0);
    }

    // Load 32 bits from `b1`, `i8` and `i16` spill slots. See `spill.b1` above.

    e.enc_both(fill.bind(B1), rec_fillSib32.opcodes(&MOV_LOAD));
    e.enc_both(regfill.bind(B1), rec_regfill32.opcodes(&MOV_LOAD));
    for &ty in &[I8, I16] {
        e.enc_both(fill.bind(ty), rec_fillSib32.opcodes(&MOV_LOAD));
        e.enc_both(regfill.bind(ty), rec_regfill32.opcodes(&MOV_LOAD));
    }

    // Push and Pop.
    e.enc32(x86_push.bind(I32), rec_pushq.opcodes(&PUSH_REG));
    e.enc_x86_64(x86_push.bind(I64), rec_pushq.opcodes(&PUSH_REG));

    e.enc32(x86_pop.bind(I32), rec_popq.opcodes(&POP_REG));
    e.enc_x86_64(x86_pop.bind(I64), rec_popq.opcodes(&POP_REG));

    // Stack-slot-to-the-same-stack-slot copy, which is guaranteed to turn
    // into a no-op.
    // The same encoding is generated for both the 64- and 32-bit architectures.
    for &ty in &[I64, I32, I16, I8] {
        e.enc64_rec(copy_nop.bind(ty), rec_stacknull, 0);
        e.enc32_rec(copy_nop.bind(ty), rec_stacknull, 0);
    }
    for &ty in &[F64, F32] {
        e.enc64_rec(copy_nop.bind(ty), rec_stacknull, 0);
        e.enc32_rec(copy_nop.bind(ty), rec_stacknull, 0);
    }

    // Adjust SP down by a dynamic value (or up, with a negative operand).
    e.enc32(adjust_sp_down.bind(I32), rec_adjustsp.opcodes(&SUB));
    e.enc64(
        adjust_sp_down.bind(I64),
        rec_adjustsp.opcodes(&SUB).rex().w(),
    );

    // Adjust SP up by an immediate (or down, with a negative immediate).
    e.enc32(adjust_sp_up_imm, rec_adjustsp_ib.opcodes(&CMP_IMM8));
    e.enc32(adjust_sp_up_imm, rec_adjustsp_id.opcodes(&CMP_IMM));
    e.enc64(
        adjust_sp_up_imm,
        rec_adjustsp_ib.opcodes(&CMP_IMM8).rex().w(),
    );
    e.enc64(
        adjust_sp_up_imm,
        rec_adjustsp_id.opcodes(&CMP_IMM).rex().w(),
    );

    // Adjust SP down by an immediate (or up, with a negative immediate).
    e.enc32(
        adjust_sp_down_imm,
        rec_adjustsp_ib.opcodes(&CMP_IMM8).rrr(5),
    );
    e.enc32(adjust_sp_down_imm, rec_adjustsp_id.opcodes(&CMP_IMM).rrr(5));
    e.enc64(
        adjust_sp_down_imm,
        rec_adjustsp_ib.opcodes(&CMP_IMM8).rrr(5).rex().w(),
    );
    e.enc64(
        adjust_sp_down_imm,
        rec_adjustsp_id.opcodes(&CMP_IMM).rrr(5).rex().w(),
    );
}

#[inline(never)]
fn define_fpu_moves(e: &mut PerCpuModeEncodings, shared_defs: &SharedDefinitions, r: &RecipeGroup) {
    let shared = &shared_defs.instructions;

    // Shorthands for instructions.
    let bitcast = shared.by_name("bitcast");
    let copy = shared.by_name("copy");
    let regmove = shared.by_name("regmove");

    // Shorthands for recipes.
    let rec_frmov = r.template("frmov");
    let rec_frurm = r.template("frurm");
    let rec_furm = r.template("furm");
    let rec_rfumr = r.template("rfumr");

    // Floating-point moves.
    // movd
    e.enc_both(
        bitcast.bind(F32).bind(I32),
        rec_frurm.opcodes(&MOVD_LOAD_XMM),
    );
    e.enc_both(
        bitcast.bind(I32).bind(F32),
        rec_rfumr.opcodes(&MOVD_STORE_XMM),
    );

    // movq
    e.enc64(
        bitcast.bind(F64).bind(I64),
        rec_frurm.opcodes(&MOVD_LOAD_XMM).rex().w(),
    );
    e.enc64(
        bitcast.bind(I64).bind(F64),
        rec_rfumr.opcodes(&MOVD_STORE_XMM).rex().w(),
    );

    // movaps
    e.enc_both(copy.bind(F32), rec_furm.opcodes(&MOVAPS_LOAD));
    e.enc_both(copy.bind(F64), rec_furm.opcodes(&MOVAPS_LOAD));

    // TODO For x86-64, only define REX forms for now, since we can't describe the special regunit
    // immediate operands with the current constraint language.
    e.enc32(regmove.bind(F32), rec_frmov.opcodes(&MOVAPS_LOAD));
    e.enc64(regmove.bind(F32), rec_frmov.opcodes(&MOVAPS_LOAD).rex());

    // TODO For x86-64, only define REX forms for now, since we can't describe the special regunit
    // immediate operands with the current constraint language.
    e.enc32(regmove.bind(F64), rec_frmov.opcodes(&MOVAPS_LOAD));
    e.enc64(regmove.bind(F64), rec_frmov.opcodes(&MOVAPS_LOAD).rex());
}

#[inline(never)]
fn define_fpu_memory(
    e: &mut PerCpuModeEncodings,
    shared_defs: &SharedDefinitions,
    r: &RecipeGroup,
) {
    let shared = &shared_defs.instructions;

    // Shorthands for instructions.
    let fill = shared.by_name("fill");
    let load = shared.by_name("load");
    let load_complex = shared.by_name("load_complex");
    let regfill = shared.by_name("regfill");
    let regspill = shared.by_name("regspill");
    let spill = shared.by_name("spill");
    let store = shared.by_name("store");
    let store_complex = shared.by_name("store_complex");

    // Shorthands for recipes.
    let rec_ffillSib32 = r.template("ffillSib32");
    let rec_fld = r.template("fld");
    let rec_fldDisp32 = r.template("fldDisp32");
    let rec_fldDisp8 = r.template("fldDisp8");
    let rec_fldWithIndex = r.template("fldWithIndex");
    let rec_fldWithIndexDisp32 = r.template("fldWithIndexDisp32");
    let rec_fldWithIndexDisp8 = r.template("fldWithIndexDisp8");
    let rec_fregfill32 = r.template("fregfill32");
    let rec_fregspill32 = r.template("fregspill32");
    let rec_fspillSib32 = r.template("fspillSib32");
    let rec_fst = r.template("fst");
    let rec_fstDisp32 = r.template("fstDisp32");
    let rec_fstDisp8 = r.template("fstDisp8");
    let rec_fstWithIndex = r.template("fstWithIndex");
    let rec_fstWithIndexDisp32 = r.template("fstWithIndexDisp32");
    let rec_fstWithIndexDisp8 = r.template("fstWithIndexDisp8");

    // Float loads and stores.
    e.enc_both(load.bind(F32).bind(Any), rec_fld.opcodes(&MOVSS_LOAD));
    e.enc_both(load.bind(F32).bind(Any), rec_fldDisp8.opcodes(&MOVSS_LOAD));
    e.enc_both(load.bind(F32).bind(Any), rec_fldDisp32.opcodes(&MOVSS_LOAD));

    e.enc_both(
        load_complex.bind(F32),
        rec_fldWithIndex.opcodes(&MOVSS_LOAD),
    );
    e.enc_both(
        load_complex.bind(F32),
        rec_fldWithIndexDisp8.opcodes(&MOVSS_LOAD),
    );
    e.enc_both(
        load_complex.bind(F32),
        rec_fldWithIndexDisp32.opcodes(&MOVSS_LOAD),
    );

    e.enc_both(load.bind(F64).bind(Any), rec_fld.opcodes(&MOVSD_LOAD));
    e.enc_both(load.bind(F64).bind(Any), rec_fldDisp8.opcodes(&MOVSD_LOAD));
    e.enc_both(load.bind(F64).bind(Any), rec_fldDisp32.opcodes(&MOVSD_LOAD));

    e.enc_both(
        load_complex.bind(F64),
        rec_fldWithIndex.opcodes(&MOVSD_LOAD),
    );
    e.enc_both(
        load_complex.bind(F64),
        rec_fldWithIndexDisp8.opcodes(&MOVSD_LOAD),
    );
    e.enc_both(
        load_complex.bind(F64),
        rec_fldWithIndexDisp32.opcodes(&MOVSD_LOAD),
    );

    e.enc_both(store.bind(F32).bind(Any), rec_fst.opcodes(&MOVSS_STORE));
    e.enc_both(
        store.bind(F32).bind(Any),
        rec_fstDisp8.opcodes(&MOVSS_STORE),
    );
    e.enc_both(
        store.bind(F32).bind(Any),
        rec_fstDisp32.opcodes(&MOVSS_STORE),
    );

    e.enc_both(
        store_complex.bind(F32),
        rec_fstWithIndex.opcodes(&MOVSS_STORE),
    );
    e.enc_both(
        store_complex.bind(F32),
        rec_fstWithIndexDisp8.opcodes(&MOVSS_STORE),
    );
    e.enc_both(
        store_complex.bind(F32),
        rec_fstWithIndexDisp32.opcodes(&MOVSS_STORE),
    );

    e.enc_both(store.bind(F64).bind(Any), rec_fst.opcodes(&MOVSD_STORE));
    e.enc_both(
        store.bind(F64).bind(Any),
        rec_fstDisp8.opcodes(&MOVSD_STORE),
    );
    e.enc_both(
        store.bind(F64).bind(Any),
        rec_fstDisp32.opcodes(&MOVSD_STORE),
    );

    e.enc_both(
        store_complex.bind(F64),
        rec_fstWithIndex.opcodes(&MOVSD_STORE),
    );
    e.enc_both(
        store_complex.bind(F64),
        rec_fstWithIndexDisp8.opcodes(&MOVSD_STORE),
    );
    e.enc_both(
        store_complex.bind(F64),
        rec_fstWithIndexDisp32.opcodes(&MOVSD_STORE),
    );

    e.enc_both(fill.bind(F32), rec_ffillSib32.opcodes(&MOVSS_LOAD));
    e.enc_both(regfill.bind(F32), rec_fregfill32.opcodes(&MOVSS_LOAD));
    e.enc_both(fill.bind(F64), rec_ffillSib32.opcodes(&MOVSD_LOAD));
    e.enc_both(regfill.bind(F64), rec_fregfill32.opcodes(&MOVSD_LOAD));

    e.enc_both(spill.bind(F32), rec_fspillSib32.opcodes(&MOVSS_STORE));
    e.enc_both(regspill.bind(F32), rec_fregspill32.opcodes(&MOVSS_STORE));
    e.enc_both(spill.bind(F64), rec_fspillSib32.opcodes(&MOVSD_STORE));
    e.enc_both(regspill.bind(F64), rec_fregspill32.opcodes(&MOVSD_STORE));
}

#[inline(never)]
fn define_fpu_ops(
    e: &mut PerCpuModeEncodings,
    shared_defs: &SharedDefinitions,
    settings: &SettingGroup,
    x86: &InstructionGroup,
    r: &RecipeGroup,
) {
    let shared = &shared_defs.instructions;
    let formats = &shared_defs.formats;

    // Shorthands for instructions.
    let ceil = shared.by_name("ceil");
    let f32const = shared.by_name("f32const");
    let f64const = shared.by_name("f64const");
    let fadd = shared.by_name("fadd");
    let fcmp = shared.by_name("fcmp");
    let fcvt_from_sint = shared.by_name("fcvt_from_sint");
    let fdemote = shared.by_name("fdemote");
    let fdiv = shared.by_name("fdiv");
    let ffcmp = shared.by_name("ffcmp");
    let floor = shared.by_name("floor");
    let fmul = shared.by_name("fmul");
    let fpromote = shared.by_name("fpromote");
    let fsub = shared.by_name("fsub");
    let nearest = shared.by_name("nearest");
    let sqrt = shared.by_name("sqrt");
    let trunc = shared.by_name("trunc");
    let x86_cvtt2si = x86.by_name("x86_cvtt2si");
    let x86_fmax = x86.by_name("x86_fmax");
    let x86_fmin = x86.by_name("x86_fmin");

    // Shorthands for recipes.
    let rec_f32imm_z = r.template("f32imm_z");
    let rec_f64imm_z = r.template("f64imm_z");
    let rec_fa = r.template("fa");
    let rec_fcmp = r.template("fcmp");
    let rec_fcscc = r.template("fcscc");
    let rec_frurm = r.template("frurm");
    let rec_furm = r.template("furm");
    let rec_furmi_rnd = r.template("furmi_rnd");
    let rec_rfurm = r.template("rfurm");

    // Predicates shorthands.
    let use_sse41 = settings.predicate_by_name("use_sse41");

    // Floating-point constants equal to 0.0 can be encoded using either `xorps` or `xorpd`, for
    // 32-bit and 64-bit floats respectively.
    let is_zero_32_bit_float =
        InstructionPredicate::new_is_zero_32bit_float(&*formats.unary_ieee32, "imm");
    e.enc32_instp(
        f32const,
        rec_f32imm_z.opcodes(&XORPS),
        is_zero_32_bit_float.clone(),
    );

    let is_zero_64_bit_float =
        InstructionPredicate::new_is_zero_64bit_float(&*formats.unary_ieee64, "imm");
    e.enc32_instp(
        f64const,
        rec_f64imm_z.opcodes(&XORPD),
        is_zero_64_bit_float.clone(),
    );

    e.enc_x86_64_instp(f32const, rec_f32imm_z.opcodes(&XORPS), is_zero_32_bit_float);
    e.enc_x86_64_instp(f64const, rec_f64imm_z.opcodes(&XORPD), is_zero_64_bit_float);

    // cvtsi2ss
    e.enc_i32_i64(fcvt_from_sint.bind(F32), rec_frurm.opcodes(&CVTSI2SS));

    // cvtsi2sd
    e.enc_i32_i64(fcvt_from_sint.bind(F64), rec_frurm.opcodes(&CVTSI2SD));

    // cvtss2sd
    e.enc_both(fpromote.bind(F64).bind(F32), rec_furm.opcodes(&CVTSS2SD));

    // cvtsd2ss
    e.enc_both(fdemote.bind(F32).bind(F64), rec_furm.opcodes(&CVTSD2SS));

    // cvttss2si
    e.enc_both(
        x86_cvtt2si.bind(I32).bind(F32),
        rec_rfurm.opcodes(&CVTTSS2SI),
    );
    e.enc64(
        x86_cvtt2si.bind(I64).bind(F32),
        rec_rfurm.opcodes(&CVTTSS2SI).rex().w(),
    );

    // cvttsd2si
    e.enc_both(
        x86_cvtt2si.bind(I32).bind(F64),
        rec_rfurm.opcodes(&CVTTSD2SI),
    );
    e.enc64(
        x86_cvtt2si.bind(I64).bind(F64),
        rec_rfurm.opcodes(&CVTTSD2SI).rex().w(),
    );

    // Exact square roots.
    e.enc_both(sqrt.bind(F32), rec_furm.opcodes(&SQRTSS));
    e.enc_both(sqrt.bind(F64), rec_furm.opcodes(&SQRTSD));

    // Rounding. The recipe looks at the opcode to pick an immediate.
    for inst in &[nearest, floor, ceil, trunc] {
        e.enc_both_isap(inst.bind(F32), rec_furmi_rnd.opcodes(&ROUNDSS), use_sse41);
        e.enc_both_isap(inst.bind(F64), rec_furmi_rnd.opcodes(&ROUNDSD), use_sse41);
    }

    // Binary arithmetic ops.
    e.enc_both(fadd.bind(F32), rec_fa.opcodes(&ADDSS));
    e.enc_both(fadd.bind(F64), rec_fa.opcodes(&ADDSD));

    e.enc_both(fsub.bind(F32), rec_fa.opcodes(&SUBSS));
    e.enc_both(fsub.bind(F64), rec_fa.opcodes(&SUBSD));

    e.enc_both(fmul.bind(F32), rec_fa.opcodes(&MULSS));
    e.enc_both(fmul.bind(F64), rec_fa.opcodes(&MULSD));

    e.enc_both(fdiv.bind(F32), rec_fa.opcodes(&DIVSS));
    e.enc_both(fdiv.bind(F64), rec_fa.opcodes(&DIVSD));

    e.enc_both(x86_fmin.bind(F32), rec_fa.opcodes(&MINSS));
    e.enc_both(x86_fmin.bind(F64), rec_fa.opcodes(&MINSD));

    e.enc_both(x86_fmax.bind(F32), rec_fa.opcodes(&MAXSS));
    e.enc_both(x86_fmax.bind(F64), rec_fa.opcodes(&MAXSD));

    // Comparisons.
    //
    // This only covers the condition codes in `supported_floatccs`, the rest are
    // handled by legalization patterns.
    e.enc_both(fcmp.bind(F32), rec_fcscc.opcodes(&UCOMISS));
    e.enc_both(fcmp.bind(F64), rec_fcscc.opcodes(&UCOMISD));
    e.enc_both(ffcmp.bind(F32), rec_fcmp.opcodes(&UCOMISS));
    e.enc_both(ffcmp.bind(F64), rec_fcmp.opcodes(&UCOMISD));
}

#[inline(never)]
fn define_alu(
    e: &mut PerCpuModeEncodings,
    shared_defs: &SharedDefinitions,
    settings: &SettingGroup,
    x86: &InstructionGroup,
    r: &RecipeGroup,
) {
    let shared = &shared_defs.instructions;

    // Shorthands for instructions.
    let clz = shared.by_name("clz");
    let ctz = shared.by_name("ctz");
    let icmp = shared.by_name("icmp");
    let icmp_imm = shared.by_name("icmp_imm");
    let ifcmp = shared.by_name("ifcmp");
    let ifcmp_imm = shared.by_name("ifcmp_imm");
    let ifcmp_sp = shared.by_name("ifcmp_sp");
    let ishl = shared.by_name("ishl");
    let ishl_imm = shared.by_name("ishl_imm");
    let popcnt = shared.by_name("popcnt");
    let rotl = shared.by_name("rotl");
    let rotl_imm = shared.by_name("rotl_imm");
    let rotr = shared.by_name("rotr");
    let rotr_imm = shared.by_name("rotr_imm");
    let selectif = shared.by_name("selectif");
    let sshr = shared.by_name("sshr");
    let sshr_imm = shared.by_name("sshr_imm");
    let trueff = shared.by_name("trueff");
    let trueif = shared.by_name("trueif");
    let ushr = shared.by_name("ushr");
    let ushr_imm = shared.by_name("ushr_imm");
    let x86_bsf = x86.by_name("x86_bsf");
    let x86_bsr = x86.by_name("x86_bsr");

    // Shorthands for recipes.
    let rec_bsf_and_bsr = r.template("bsf_and_bsr");
    let rec_cmov = r.template("cmov");
    let rec_icscc = r.template("icscc");
    let rec_icscc_ib = r.template("icscc_ib");
    let rec_icscc_id = r.template("icscc_id");
    let rec_rcmp = r.template("rcmp");
    let rec_rcmp_ib = r.template("rcmp_ib");
    let rec_rcmp_id = r.template("rcmp_id");
    let rec_rcmp_sp = r.template("rcmp_sp");
    let rec_rc = r.template("rc");
    let rec_setf_abcd = r.template("setf_abcd");
    let rec_seti_abcd = r.template("seti_abcd");
    let rec_urm = r.template("urm");

    // Predicates shorthands.
    let use_popcnt = settings.predicate_by_name("use_popcnt");
    let use_lzcnt = settings.predicate_by_name("use_lzcnt");
    let use_bmi1 = settings.predicate_by_name("use_bmi1");

    let band = shared.by_name("band");
    let band_imm = shared.by_name("band_imm");
    let band_not = shared.by_name("band_not");
    let bnot = shared.by_name("bnot");
    let bor = shared.by_name("bor");
    let bor_imm = shared.by_name("bor_imm");
    let bxor = shared.by_name("bxor");
    let bxor_imm = shared.by_name("bxor_imm");
    let iadd = shared.by_name("iadd");
    let iadd_ifcarry = shared.by_name("iadd_ifcarry");
    let iadd_ifcin = shared.by_name("iadd_ifcin");
    let iadd_ifcout = shared.by_name("iadd_ifcout");
    let iadd_imm = shared.by_name("iadd_imm");
    let imul = shared.by_name("imul");
    let isub = shared.by_name("isub");
    let isub_ifbin = shared.by_name("isub_ifbin");
    let isub_ifborrow = shared.by_name("isub_ifborrow");
    let isub_ifbout = shared.by_name("isub_ifbout");
    let x86_sdivmodx = x86.by_name("x86_sdivmodx");
    let x86_smulx = x86.by_name("x86_smulx");
    let x86_udivmodx = x86.by_name("x86_udivmodx");
    let x86_umulx = x86.by_name("x86_umulx");

    let rec_div = r.template("div");
    let rec_fa = r.template("fa");
    let rec_fax = r.template("fax");
    let rec_mulx = r.template("mulx");
    let rec_r_ib = r.template("r_ib");
    let rec_r_id = r.template("r_id");
    let rec_rin = r.template("rin");
    let rec_rio = r.template("rio");
    let rec_rout = r.template("rout");
    let rec_rr = r.template("rr");
    let rec_rrx = r.template("rrx");
    let rec_ur = r.template("ur");

    e.enc_i32_i64(iadd, rec_rr.opcodes(&ADD));
    e.enc_i32_i64(iadd_ifcout, rec_rout.opcodes(&ADD));
    e.enc_i32_i64(iadd_ifcin, rec_rin.opcodes(&ADC));
    e.enc_i32_i64(iadd_ifcarry, rec_rio.opcodes(&ADC));
    e.enc_i32_i64(iadd_imm, rec_r_ib.opcodes(&ADD_IMM8_SIGN_EXTEND).rrr(0));
    e.enc_i32_i64(iadd_imm, rec_r_id.opcodes(&ADD_IMM).rrr(0));

    e.enc_i32_i64(isub, rec_rr.opcodes(&SUB));
    e.enc_i32_i64(isub_ifbout, rec_rout.opcodes(&SUB));
    e.enc_i32_i64(isub_ifbin, rec_rin.opcodes(&SBB));
    e.enc_i32_i64(isub_ifborrow, rec_rio.opcodes(&SBB));

    e.enc_i32_i64(band, rec_rr.opcodes(&AND));
    e.enc_b32_b64(band, rec_rr.opcodes(&AND));

    // TODO: band_imm.i64 with an unsigned 32-bit immediate can be encoded as band_imm.i32. Can
    // even use the single-byte immediate for 0xffff_ffXX masks.

    e.enc_i32_i64(band_imm, rec_r_ib.opcodes(&AND_IMM8_SIGN_EXTEND).rrr(4));
    e.enc_i32_i64(band_imm, rec_r_id.opcodes(&AND_IMM).rrr(4));

    e.enc_i32_i64(bor, rec_rr.opcodes(&OR));
    e.enc_b32_b64(bor, rec_rr.opcodes(&OR));
    e.enc_i32_i64(bor_imm, rec_r_ib.opcodes(&OR_IMM8_SIGN_EXTEND).rrr(1));
    e.enc_i32_i64(bor_imm, rec_r_id.opcodes(&OR_IMM).rrr(1));

    e.enc_i32_i64(bxor, rec_rr.opcodes(&XOR));
    e.enc_b32_b64(bxor, rec_rr.opcodes(&XOR));
    e.enc_i32_i64(bxor_imm, rec_r_ib.opcodes(&XOR_IMM8_SIGN_EXTEND).rrr(6));
    e.enc_i32_i64(bxor_imm, rec_r_id.opcodes(&XOR_IMM).rrr(6));

    // x86 has a bitwise not instruction NOT.
    e.enc_i32_i64(bnot, rec_ur.opcodes(&NOT).rrr(2));
    e.enc_b32_b64(bnot, rec_ur.opcodes(&NOT).rrr(2));
    e.enc_both(bnot.bind(B1), rec_ur.opcodes(&NOT).rrr(2));

    // Also add a `b1` encodings for the logic instructions.
    // TODO: Should this be done with 8-bit instructions? It would improve partial register
    // dependencies.
    e.enc_both(band.bind(B1), rec_rr.opcodes(&AND));
    e.enc_both(bor.bind(B1), rec_rr.opcodes(&OR));
    e.enc_both(bxor.bind(B1), rec_rr.opcodes(&XOR));

    e.enc_i32_i64(imul, rec_rrx.opcodes(&IMUL));
    e.enc_i32_i64(x86_sdivmodx, rec_div.opcodes(&IDIV).rrr(7));
    e.enc_i32_i64(x86_udivmodx, rec_div.opcodes(&DIV).rrr(6));

    e.enc_i32_i64(x86_smulx, rec_mulx.opcodes(&IMUL_RDX_RAX).rrr(5));
    e.enc_i32_i64(x86_umulx, rec_mulx.opcodes(&MUL).rrr(4));

    // Binary bitwise ops.
    //
    // The F64 version is intentionally encoded using the single-precision opcode:
    // the operation is identical and the encoding is one byte shorter.
    e.enc_both(band.bind(F32), rec_fa.opcodes(&ANDPS));
    e.enc_both(band.bind(F64), rec_fa.opcodes(&ANDPS));

    e.enc_both(bor.bind(F32), rec_fa.opcodes(&ORPS));
    e.enc_both(bor.bind(F64), rec_fa.opcodes(&ORPS));

    e.enc_both(bxor.bind(F32), rec_fa.opcodes(&XORPS));
    e.enc_both(bxor.bind(F64), rec_fa.opcodes(&XORPS));

    // The `andnps(x,y)` instruction computes `~x&y`, while band_not(x,y)` is `x&~y.
    e.enc_both(band_not.bind(F32), rec_fax.opcodes(&ANDNPS));
    e.enc_both(band_not.bind(F64), rec_fax.opcodes(&ANDNPS));

    // Shifts and rotates.
    // Note that the dynamic shift amount is only masked by 5 or 6 bits; the 8-bit
    // and 16-bit shifts would need explicit masking.

    for &(inst, rrr) in &[(rotl, 0), (rotr, 1), (ishl, 4), (ushr, 5), (sshr, 7)] {
        // Cannot use enc_i32_i64 for this pattern because instructions require
        // to bind any.
        e.enc32(inst.bind(I32).bind(I8), rec_rc.opcodes(&ROTATE_CL).rrr(rrr));
        e.enc32(
            inst.bind(I32).bind(I16),
            rec_rc.opcodes(&ROTATE_CL).rrr(rrr),
        );
        e.enc32(
            inst.bind(I32).bind(I32),
            rec_rc.opcodes(&ROTATE_CL).rrr(rrr),
        );
        e.enc64(
            inst.bind(I64).bind(Any),
            rec_rc.opcodes(&ROTATE_CL).rrr(rrr).rex().w(),
        );
        e.enc64(
            inst.bind(I32).bind(Any),
            rec_rc.opcodes(&ROTATE_CL).rrr(rrr).rex(),
        );
        e.enc64(
            inst.bind(I32).bind(Any),
            rec_rc.opcodes(&ROTATE_CL).rrr(rrr),
        );
    }

    e.enc_i32_i64(rotl_imm, rec_r_ib.opcodes(&ROTATE_IMM8).rrr(0));
    e.enc_i32_i64(rotr_imm, rec_r_ib.opcodes(&ROTATE_IMM8).rrr(1));
    e.enc_i32_i64(ishl_imm, rec_r_ib.opcodes(&ROTATE_IMM8).rrr(4));
    e.enc_i32_i64(ushr_imm, rec_r_ib.opcodes(&ROTATE_IMM8).rrr(5));
    e.enc_i32_i64(sshr_imm, rec_r_ib.opcodes(&ROTATE_IMM8).rrr(7));

    // Population count.
    e.enc32_isap(popcnt.bind(I32), rec_urm.opcodes(&POPCNT), use_popcnt);
    e.enc64_isap(
        popcnt.bind(I64),
        rec_urm.opcodes(&POPCNT).rex().w(),
        use_popcnt,
    );
    e.enc64_isap(popcnt.bind(I32), rec_urm.opcodes(&POPCNT).rex(), use_popcnt);
    e.enc64_isap(popcnt.bind(I32), rec_urm.opcodes(&POPCNT), use_popcnt);

    // Count leading zero bits.
    e.enc32_isap(clz.bind(I32), rec_urm.opcodes(&LZCNT), use_lzcnt);
    e.enc64_isap(clz.bind(I64), rec_urm.opcodes(&LZCNT).rex().w(), use_lzcnt);
    e.enc64_isap(clz.bind(I32), rec_urm.opcodes(&LZCNT).rex(), use_lzcnt);
    e.enc64_isap(clz.bind(I32), rec_urm.opcodes(&LZCNT), use_lzcnt);

    // Count trailing zero bits.
    e.enc32_isap(ctz.bind(I32), rec_urm.opcodes(&TZCNT), use_bmi1);
    e.enc64_isap(ctz.bind(I64), rec_urm.opcodes(&TZCNT).rex().w(), use_bmi1);
    e.enc64_isap(ctz.bind(I32), rec_urm.opcodes(&TZCNT).rex(), use_bmi1);
    e.enc64_isap(ctz.bind(I32), rec_urm.opcodes(&TZCNT), use_bmi1);

    // Bit scan forwards and reverse
    e.enc_i32_i64(x86_bsf, rec_bsf_and_bsr.opcodes(&BIT_SCAN_FORWARD));
    e.enc_i32_i64(x86_bsr, rec_bsf_and_bsr.opcodes(&BIT_SCAN_REVERSE));

    // Comparisons
    e.enc_i32_i64(icmp, rec_icscc.opcodes(&CMP_REG));
    e.enc_i32_i64(icmp_imm, rec_icscc_ib.opcodes(&CMP_IMM8).rrr(7));
    e.enc_i32_i64(icmp_imm, rec_icscc_id.opcodes(&CMP_IMM).rrr(7));
    e.enc_i32_i64(ifcmp, rec_rcmp.opcodes(&CMP_REG));
    e.enc_i32_i64(ifcmp_imm, rec_rcmp_ib.opcodes(&CMP_IMM8).rrr(7));
    e.enc_i32_i64(ifcmp_imm, rec_rcmp_id.opcodes(&CMP_IMM).rrr(7));
    // TODO: We could special-case ifcmp_imm(x, 0) to TEST(x, x).

    e.enc32(ifcmp_sp.bind(I32), rec_rcmp_sp.opcodes(&CMP_REG));
    e.enc64(ifcmp_sp.bind(I64), rec_rcmp_sp.opcodes(&CMP_REG).rex().w());

    // Convert flags to bool.
    // This encodes `b1` as an 8-bit low register with the value 0 or 1.
    e.enc_both(trueif, rec_seti_abcd.opcodes(&SET_BYTE_IF_OVERFLOW));
    e.enc_both(trueff, rec_setf_abcd.opcodes(&SET_BYTE_IF_OVERFLOW));

    // Conditional move (a.k.a integer select).
    e.enc_i32_i64(selectif, rec_cmov.opcodes(&CMOV_OVERFLOW));
}

#[inline(never)]
#[allow(clippy::cognitive_complexity)]
fn define_simd(
    e: &mut PerCpuModeEncodings,
    shared_defs: &SharedDefinitions,
    settings: &SettingGroup,
    x86: &InstructionGroup,
    r: &RecipeGroup,
) {
    let shared = &shared_defs.instructions;
    let formats = &shared_defs.formats;

    // Shorthands for instructions.
    let avg_round = shared.by_name("avg_round");
    let bitcast = shared.by_name("bitcast");
    let bor = shared.by_name("bor");
    let bxor = shared.by_name("bxor");
    let copy = shared.by_name("copy");
    let copy_nop = shared.by_name("copy_nop");
    let copy_to_ssa = shared.by_name("copy_to_ssa");
    let fadd = shared.by_name("fadd");
    let fcmp = shared.by_name("fcmp");
    let fcvt_from_sint = shared.by_name("fcvt_from_sint");
    let fdiv = shared.by_name("fdiv");
    let fill = shared.by_name("fill");
    let fill_nop = shared.by_name("fill_nop");
    let fmax = shared.by_name("fmax");
    let fmin = shared.by_name("fmin");
    let fmul = shared.by_name("fmul");
    let fsub = shared.by_name("fsub");
    let iadd = shared.by_name("iadd");
    let icmp = shared.by_name("icmp");
    let imul = shared.by_name("imul");
    let ishl_imm = shared.by_name("ishl_imm");
    let load = shared.by_name("load");
    let load_complex = shared.by_name("load_complex");
    let raw_bitcast = shared.by_name("raw_bitcast");
    let regfill = shared.by_name("regfill");
    let regmove = shared.by_name("regmove");
    let regspill = shared.by_name("regspill");
    let sadd_sat = shared.by_name("sadd_sat");
    let scalar_to_vector = shared.by_name("scalar_to_vector");
    let sload8x8 = shared.by_name("sload8x8");
    let sload8x8_complex = shared.by_name("sload8x8_complex");
    let sload16x4 = shared.by_name("sload16x4");
    let sload16x4_complex = shared.by_name("sload16x4_complex");
    let sload32x2 = shared.by_name("sload32x2");
    let sload32x2_complex = shared.by_name("sload32x2_complex");
    let spill = shared.by_name("spill");
    let sqrt = shared.by_name("sqrt");
    let sshr_imm = shared.by_name("sshr_imm");
    let ssub_sat = shared.by_name("ssub_sat");
    let store = shared.by_name("store");
    let store_complex = shared.by_name("store_complex");
    let uadd_sat = shared.by_name("uadd_sat");
    let uload8x8 = shared.by_name("uload8x8");
    let uload8x8_complex = shared.by_name("uload8x8_complex");
    let uload16x4 = shared.by_name("uload16x4");
    let uload16x4_complex = shared.by_name("uload16x4_complex");
    let uload32x2 = shared.by_name("uload32x2");
    let uload32x2_complex = shared.by_name("uload32x2_complex");
    let ushr_imm = shared.by_name("ushr_imm");
    let usub_sat = shared.by_name("usub_sat");
    let vconst = shared.by_name("vconst");
    let vselect = shared.by_name("vselect");
    let x86_cvtt2si = x86.by_name("x86_cvtt2si");
    let x86_insertps = x86.by_name("x86_insertps");
    let x86_movlhps = x86.by_name("x86_movlhps");
    let x86_movsd = x86.by_name("x86_movsd");
    let x86_packss = x86.by_name("x86_packss");
    let x86_pblendw = x86.by_name("x86_pblendw");
    let x86_pextr = x86.by_name("x86_pextr");
    let x86_pinsr = x86.by_name("x86_pinsr");
    let x86_pmaxs = x86.by_name("x86_pmaxs");
    let x86_pmaxu = x86.by_name("x86_pmaxu");
    let x86_pmins = x86.by_name("x86_pmins");
    let x86_pminu = x86.by_name("x86_pminu");
    let x86_pmullq = x86.by_name("x86_pmullq");
    let x86_pmuludq = x86.by_name("x86_pmuludq");
    let x86_pshufb = x86.by_name("x86_pshufb");
    let x86_pshufd = x86.by_name("x86_pshufd");
    let x86_psll = x86.by_name("x86_psll");
    let x86_psra = x86.by_name("x86_psra");
    let x86_psrl = x86.by_name("x86_psrl");
    let x86_ptest = x86.by_name("x86_ptest");
    let x86_punpckh = x86.by_name("x86_punpckh");
    let x86_punpckl = x86.by_name("x86_punpckl");
    let x86_vcvtudq2ps = x86.by_name("x86_vcvtudq2ps");

    // Shorthands for recipes.
    let rec_blend = r.template("blend");
    let rec_evex_reg_vvvv_rm_128 = r.template("evex_reg_vvvv_rm_128");
    let rec_evex_reg_rm_128 = r.template("evex_reg_rm_128");
    let rec_f_ib = r.template("f_ib");
    let rec_fa = r.template("fa");
    let rec_fa_ib = r.template("fa_ib");
    let rec_fax = r.template("fax");
    let rec_fcmp = r.template("fcmp");
    let rec_ffillSib32 = r.template("ffillSib32");
    let rec_ffillnull = r.recipe("ffillnull");
    let rec_fld = r.template("fld");
    let rec_fldDisp32 = r.template("fldDisp32");
    let rec_fldDisp8 = r.template("fldDisp8");
    let rec_fldWithIndex = r.template("fldWithIndex");
    let rec_fldWithIndexDisp32 = r.template("fldWithIndexDisp32");
    let rec_fldWithIndexDisp8 = r.template("fldWithIndexDisp8");
    let rec_fregfill32 = r.template("fregfill32");
    let rec_fregspill32 = r.template("fregspill32");
    let rec_frmov = r.template("frmov");
    let rec_frurm = r.template("frurm");
    let rec_fspillSib32 = r.template("fspillSib32");
    let rec_fst = r.template("fst");
    let rec_fstDisp32 = r.template("fstDisp32");
    let rec_fstDisp8 = r.template("fstDisp8");
    let rec_fstWithIndex = r.template("fstWithIndex");
    let rec_fstWithIndexDisp32 = r.template("fstWithIndexDisp32");
    let rec_fstWithIndexDisp8 = r.template("fstWithIndexDisp8");
    let rec_furm = r.template("furm");
    let rec_furm_reg_to_ssa = r.template("furm_reg_to_ssa");
    let rec_icscc_fpr = r.template("icscc_fpr");
    let rec_null_fpr = r.recipe("null_fpr");
    let rec_pfcmp = r.template("pfcmp");
    let rec_r_ib_unsigned_fpr = r.template("r_ib_unsigned_fpr");
    let rec_r_ib_unsigned_gpr = r.template("r_ib_unsigned_gpr");
    let rec_r_ib_unsigned_r = r.template("r_ib_unsigned_r");
    let rec_stacknull = r.recipe("stacknull");
    let rec_vconst = r.template("vconst");
    let rec_vconst_optimized = r.template("vconst_optimized");

    // Predicates shorthands.
    settings.predicate_by_name("all_ones_funcaddrs_and_not_is_pic");
    settings.predicate_by_name("not_all_ones_funcaddrs_and_not_is_pic");
    let use_ssse3_simd = settings.predicate_by_name("use_ssse3_simd");
    let use_sse41_simd = settings.predicate_by_name("use_sse41_simd");
    let use_sse42_simd = settings.predicate_by_name("use_sse42_simd");
    let use_avx512dq_simd = settings.predicate_by_name("use_avx512dq_simd");
    let use_avx512vl_simd = settings.predicate_by_name("use_avx512vl_simd");

    // SIMD vector size: eventually multiple vector sizes may be supported but for now only
    // SSE-sized vectors are available.
    let sse_vector_size: u64 = 128;

    // SIMD splat: before x86 can use vector data, it must be moved to XMM registers; see
    // legalize.rs for how this is done; once there, x86_pshuf* (below) is used for broadcasting the
    // value across the register.

    let allowed_simd_type = |t: &LaneType| t.lane_bits() >= 8 && t.lane_bits() < 128;

    // PSHUFB, 8-bit shuffle using two XMM registers.
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        let instruction = x86_pshufb.bind(vector(ty, sse_vector_size));
        let template = rec_fa.opcodes(&PSHUFB);
        e.enc_both_inferred_maybe_isap(instruction.clone(), template.clone(), Some(use_ssse3_simd));
    }

    // PSHUFD, 32-bit shuffle using one XMM register and a u8 immediate.
    for ty in ValueType::all_lane_types().filter(|t| t.lane_bits() == 32) {
        let instruction = x86_pshufd.bind(vector(ty, sse_vector_size));
        let template = rec_r_ib_unsigned_fpr.opcodes(&PSHUFD);
        e.enc_both_inferred(instruction, template);
    }

    // SIMD vselect; controlling value of vselect is a boolean vector, so each lane should be
    // either all ones or all zeroes - it makes it possible to always use 8-bit PBLENDVB;
    // for 32/64-bit lanes we can also use BLENDVPS and BLENDVPD
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        let opcode = match ty.lane_bits() {
            32 => &BLENDVPS,
            64 => &BLENDVPD,
            _ => &PBLENDVB,
        };
        let instruction = vselect.bind(vector(ty, sse_vector_size));
        let template = rec_blend.opcodes(opcode);
        e.enc_both_inferred_maybe_isap(instruction, template, Some(use_sse41_simd));
    }

    // PBLENDW, select lanes using a u8 immediate.
    for ty in ValueType::all_lane_types().filter(|t| t.lane_bits() == 16) {
        let instruction = x86_pblendw.bind(vector(ty, sse_vector_size));
        let template = rec_fa_ib.opcodes(&PBLENDW);
        e.enc_both_inferred_maybe_isap(instruction, template, Some(use_sse41_simd));
    }

    // SIMD scalar_to_vector; this uses MOV to copy the scalar value to an XMM register; according
    // to the Intel manual: "When the destination operand is an XMM register, the source operand is
    // written to the low doubleword of the register and the register is zero-extended to 128 bits."
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        let instruction = scalar_to_vector.bind(vector(ty, sse_vector_size));
        if ty.is_float() {
            // No need to move floats--they already live in XMM registers.
            e.enc_32_64_rec(instruction, rec_null_fpr, 0);
        } else {
            let template = rec_frurm.opcodes(&MOVD_LOAD_XMM);
            if ty.lane_bits() < 64 {
                e.enc_both_inferred(instruction, template);
            } else {
                // No 32-bit encodings for 64-bit widths.
                assert_eq!(ty.lane_bits(), 64);
                e.enc64(instruction, template.rex().w());
            }
        }
    }

    // SIMD insertlane
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        let (opcode, isap): (&[_], _) = match ty.lane_bits() {
            8 => (&PINSRB, Some(use_sse41_simd)),
            16 => (&PINSRW, None),
            32 | 64 => (&PINSR, Some(use_sse41_simd)),
            _ => panic!("invalid size for SIMD insertlane"),
        };

        let instruction = x86_pinsr.bind(vector(ty, sse_vector_size));
        let template = rec_r_ib_unsigned_r.opcodes(opcode);
        if ty.lane_bits() < 64 {
            e.enc_both_inferred_maybe_isap(instruction, template, isap);
        } else {
            // It turns out the 64-bit widths have REX/W encodings and only are available on
            // x86_64.
            e.enc64_maybe_isap(instruction, template.rex().w(), isap);
        }
    }

    // For legalizing insertlane with floats, INSERTPS from SSE4.1.
    {
        let instruction = x86_insertps.bind(vector(F32, sse_vector_size));
        let template = rec_fa_ib.opcodes(&INSERTPS);
        e.enc_both_inferred_maybe_isap(instruction, template, Some(use_sse41_simd));
    }

    // For legalizing insertlane with floats,  MOVSD from SSE2.
    {
        let instruction = x86_movsd.bind(vector(F64, sse_vector_size));
        let template = rec_fa.opcodes(&MOVSD_LOAD);
        e.enc_both_inferred(instruction, template); // from SSE2
    }

    // For legalizing insertlane with floats, MOVLHPS from SSE.
    {
        let instruction = x86_movlhps.bind(vector(F64, sse_vector_size));
        let template = rec_fa.opcodes(&MOVLHPS);
        e.enc_both_inferred(instruction, template); // from SSE
    }

    // SIMD extractlane
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        let opcode = match ty.lane_bits() {
            8 => &PEXTRB,
            16 => &PEXTRW,
            32 | 64 => &PEXTR,
            _ => panic!("invalid size for SIMD extractlane"),
        };

        let instruction = x86_pextr.bind(vector(ty, sse_vector_size));
        let template = rec_r_ib_unsigned_gpr.opcodes(opcode);
        if ty.lane_bits() < 64 {
            e.enc_both_inferred_maybe_isap(instruction, template, Some(use_sse41_simd));
        } else {
            // It turns out the 64-bit widths have REX/W encodings and only are available on
            // x86_64.
            e.enc64_maybe_isap(instruction, template.rex().w(), Some(use_sse41_simd));
        }
    }

    // SIMD packing/unpacking
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        let (high, low) = match ty.lane_bits() {
            8 => (&PUNPCKHBW, &PUNPCKLBW),
            16 => (&PUNPCKHWD, &PUNPCKLWD),
            32 => (&PUNPCKHDQ, &PUNPCKLDQ),
            64 => (&PUNPCKHQDQ, &PUNPCKLQDQ),
            _ => panic!("invalid size for SIMD packing/unpacking"),
        };

        e.enc_both_inferred(
            x86_punpckh.bind(vector(ty, sse_vector_size)),
            rec_fa.opcodes(high),
        );
        e.enc_both_inferred(
            x86_punpckl.bind(vector(ty, sse_vector_size)),
            rec_fa.opcodes(low),
        );
    }
    for (ty, opcodes) in &[(I16, &PACKSSWB), (I32, &PACKSSDW)] {
        let x86_packss = x86_packss.bind(vector(*ty, sse_vector_size));
        e.enc_both_inferred(x86_packss, rec_fa.opcodes(*opcodes));
    }

    // SIMD bitcast all 128-bit vectors to each other (for legalizing splat.x16x8).
    for from_type in ValueType::all_lane_types().filter(allowed_simd_type) {
        for to_type in
            ValueType::all_lane_types().filter(|t| allowed_simd_type(t) && *t != from_type)
        {
            let instruction = raw_bitcast
                .bind(vector(to_type, sse_vector_size))
                .bind(vector(from_type, sse_vector_size));
            e.enc_32_64_rec(instruction, rec_null_fpr, 0);
        }
    }

    // SIMD raw bitcast floats to vector (and back); assumes that floats are already stored in an
    // XMM register.
    for float_type in &[F32, F64] {
        for lane_type in ValueType::all_lane_types().filter(allowed_simd_type) {
            e.enc_32_64_rec(
                raw_bitcast
                    .bind(vector(lane_type, sse_vector_size))
                    .bind(*float_type),
                rec_null_fpr,
                0,
            );
            e.enc_32_64_rec(
                raw_bitcast
                    .bind(*float_type)
                    .bind(vector(lane_type, sse_vector_size)),
                rec_null_fpr,
                0,
            );
        }
    }

    // SIMD conversions
    {
        let fcvt_from_sint_32 = fcvt_from_sint
            .bind(vector(F32, sse_vector_size))
            .bind(vector(I32, sse_vector_size));
        e.enc_both(fcvt_from_sint_32, rec_furm.opcodes(&CVTDQ2PS));

        e.enc_32_64_maybe_isap(
            x86_vcvtudq2ps,
            rec_evex_reg_rm_128.opcodes(&VCVTUDQ2PS),
            Some(use_avx512vl_simd), // TODO need an OR predicate to join with AVX512F
        );

        e.enc_both_inferred(
            x86_cvtt2si
                .bind(vector(I32, sse_vector_size))
                .bind(vector(F32, sse_vector_size)),
            rec_furm.opcodes(&CVTTPS2DQ),
        );
    }

    // SIMD vconst for special cases (all zeroes, all ones)
    // this must be encoded prior to the MOVUPS implementation (below) so the compiler sees this
    // encoding first
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        let instruction = vconst.bind(vector(ty, sse_vector_size));

        let is_zero_128bit =
            InstructionPredicate::new_is_all_zeroes(&*formats.unary_const, "constant_handle");
        let template = rec_vconst_optimized.opcodes(&PXOR).infer_rex();
        e.enc_32_64_func(instruction.clone(), template, |builder| {
            builder.inst_predicate(is_zero_128bit)
        });

        let is_ones_128bit =
            InstructionPredicate::new_is_all_ones(&*formats.unary_const, "constant_handle");
        let template = rec_vconst_optimized.opcodes(&PCMPEQB).infer_rex();
        e.enc_32_64_func(instruction, template, |builder| {
            builder.inst_predicate(is_ones_128bit)
        });
    }

    // SIMD vconst using MOVUPS
    // TODO it would be ideal if eventually this became the more efficient MOVAPS but we would have
    // to guarantee that the constants are aligned when emitted and there is currently no mechanism
    // for that; alternately, constants could be loaded into XMM registers using a sequence like:
    // MOVQ + MOVHPD + MOVQ + MOVLPD (this allows the constants to be immediates instead of stored
    // in memory) but some performance measurements are needed.
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        let instruction = vconst.bind(vector(ty, sse_vector_size));
        let template = rec_vconst.opcodes(&MOVUPS_LOAD);
        e.enc_both_inferred(instruction, template); // from SSE
    }

    // SIMD register movement: store, load, spill, fill, regmove, etc. All of these use encodings of
    // MOVUPS and MOVAPS from SSE (TODO ideally all of these would either use MOVAPS when we have
    // alignment or type-specific encodings, see https://github.com/bytecodealliance/wasmtime/issues/1124).
    // Also, it would be ideal to infer REX prefixes for all of these instructions but for the
    // time being only instructions with common recipes have `infer_rex()` support.
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        // Store
        let bound_store = store.bind(vector(ty, sse_vector_size)).bind(Any);
        e.enc_both_inferred(bound_store.clone(), rec_fst.opcodes(&MOVUPS_STORE));
        e.enc_both_inferred(bound_store.clone(), rec_fstDisp8.opcodes(&MOVUPS_STORE));
        e.enc_both_inferred(bound_store, rec_fstDisp32.opcodes(&MOVUPS_STORE));

        // Store complex
        let bound_store_complex = store_complex.bind(vector(ty, sse_vector_size));
        e.enc_both(
            bound_store_complex.clone(),
            rec_fstWithIndex.opcodes(&MOVUPS_STORE),
        );
        e.enc_both(
            bound_store_complex.clone(),
            rec_fstWithIndexDisp8.opcodes(&MOVUPS_STORE),
        );
        e.enc_both(
            bound_store_complex,
            rec_fstWithIndexDisp32.opcodes(&MOVUPS_STORE),
        );

        // Load
        let bound_load = load.bind(vector(ty, sse_vector_size)).bind(Any);
        e.enc_both_inferred(bound_load.clone(), rec_fld.opcodes(&MOVUPS_LOAD));
        e.enc_both_inferred(bound_load.clone(), rec_fldDisp8.opcodes(&MOVUPS_LOAD));
        e.enc_both_inferred(bound_load, rec_fldDisp32.opcodes(&MOVUPS_LOAD));

        // Load complex
        let bound_load_complex = load_complex.bind(vector(ty, sse_vector_size));
        e.enc_both(
            bound_load_complex.clone(),
            rec_fldWithIndex.opcodes(&MOVUPS_LOAD),
        );
        e.enc_both(
            bound_load_complex.clone(),
            rec_fldWithIndexDisp8.opcodes(&MOVUPS_LOAD),
        );
        e.enc_both(
            bound_load_complex,
            rec_fldWithIndexDisp32.opcodes(&MOVUPS_LOAD),
        );

        // Spill
        let bound_spill = spill.bind(vector(ty, sse_vector_size));
        e.enc_both(bound_spill, rec_fspillSib32.opcodes(&MOVUPS_STORE));
        let bound_regspill = regspill.bind(vector(ty, sse_vector_size));
        e.enc_both(bound_regspill, rec_fregspill32.opcodes(&MOVUPS_STORE));

        // Fill
        let bound_fill = fill.bind(vector(ty, sse_vector_size));
        e.enc_both(bound_fill, rec_ffillSib32.opcodes(&MOVUPS_LOAD));
        let bound_regfill = regfill.bind(vector(ty, sse_vector_size));
        e.enc_both(bound_regfill, rec_fregfill32.opcodes(&MOVUPS_LOAD));
        let bound_fill_nop = fill_nop.bind(vector(ty, sse_vector_size));
        e.enc_32_64_rec(bound_fill_nop, rec_ffillnull, 0);

        // Regmove
        let bound_regmove = regmove.bind(vector(ty, sse_vector_size));
        e.enc_both(bound_regmove, rec_frmov.opcodes(&MOVAPS_LOAD));

        // Copy
        let bound_copy = copy.bind(vector(ty, sse_vector_size));
        e.enc_both(bound_copy, rec_furm.opcodes(&MOVAPS_LOAD));
        let bound_copy_to_ssa = copy_to_ssa.bind(vector(ty, sse_vector_size));
        e.enc_both(bound_copy_to_ssa, rec_furm_reg_to_ssa.opcodes(&MOVAPS_LOAD));
        let bound_copy_nop = copy_nop.bind(vector(ty, sse_vector_size));
        e.enc_32_64_rec(bound_copy_nop, rec_stacknull, 0);
    }

    // SIMD load extend
    for (inst, opcodes) in &[
        (uload8x8, &PMOVZXBW),
        (uload16x4, &PMOVZXWD),
        (uload32x2, &PMOVZXDQ),
        (sload8x8, &PMOVSXBW),
        (sload16x4, &PMOVSXWD),
        (sload32x2, &PMOVSXDQ),
    ] {
        let isap = Some(use_sse41_simd);
        for recipe in &[rec_fld, rec_fldDisp8, rec_fldDisp32] {
            let inst = *inst;
            let template = recipe.opcodes(*opcodes);
            e.enc_both_inferred_maybe_isap(inst.clone().bind(I32), template.clone(), isap);
            e.enc64_maybe_isap(inst.bind(I64), template.infer_rex(), isap);
        }
    }

    // SIMD load extend (complex addressing)
    let is_load_complex_length_two =
        InstructionPredicate::new_length_equals(&*formats.load_complex, 2);
    for (inst, opcodes) in &[
        (uload8x8_complex, &PMOVZXBW),
        (uload16x4_complex, &PMOVZXWD),
        (uload32x2_complex, &PMOVZXDQ),
        (sload8x8_complex, &PMOVSXBW),
        (sload16x4_complex, &PMOVSXWD),
        (sload32x2_complex, &PMOVSXDQ),
    ] {
        for recipe in &[
            rec_fldWithIndex,
            rec_fldWithIndexDisp8,
            rec_fldWithIndexDisp32,
        ] {
            let template = recipe.opcodes(*opcodes);
            let predicate = |encoding: EncodingBuilder| {
                encoding
                    .isa_predicate(use_sse41_simd)
                    .inst_predicate(is_load_complex_length_two.clone())
            };
            e.enc32_func(inst.clone(), template.clone(), predicate);
            // No infer_rex calculator for these recipes; place REX version first as in enc_x86_64.
            e.enc64_func(inst.clone(), template.rex(), predicate);
            e.enc64_func(inst.clone(), template, predicate);
        }
    }

    // SIMD integer addition
    for (ty, opcodes) in &[(I8, &PADDB), (I16, &PADDW), (I32, &PADDD), (I64, &PADDQ)] {
        let iadd = iadd.bind(vector(*ty, sse_vector_size));
        e.enc_both_inferred(iadd, rec_fa.opcodes(*opcodes));
    }

    // SIMD integer saturating addition
    e.enc_both_inferred(
        sadd_sat.bind(vector(I8, sse_vector_size)),
        rec_fa.opcodes(&PADDSB),
    );
    e.enc_both_inferred(
        sadd_sat.bind(vector(I16, sse_vector_size)),
        rec_fa.opcodes(&PADDSW),
    );
    e.enc_both_inferred(
        uadd_sat.bind(vector(I8, sse_vector_size)),
        rec_fa.opcodes(&PADDUSB),
    );
    e.enc_both_inferred(
        uadd_sat.bind(vector(I16, sse_vector_size)),
        rec_fa.opcodes(&PADDUSW),
    );

    // SIMD integer subtraction
    let isub = shared.by_name("isub");
    for (ty, opcodes) in &[(I8, &PSUBB), (I16, &PSUBW), (I32, &PSUBD), (I64, &PSUBQ)] {
        let isub = isub.bind(vector(*ty, sse_vector_size));
        e.enc_both_inferred(isub, rec_fa.opcodes(*opcodes));
    }

    // SIMD integer saturating subtraction
    e.enc_both_inferred(
        ssub_sat.bind(vector(I8, sse_vector_size)),
        rec_fa.opcodes(&PSUBSB),
    );
    e.enc_both_inferred(
        ssub_sat.bind(vector(I16, sse_vector_size)),
        rec_fa.opcodes(&PSUBSW),
    );
    e.enc_both_inferred(
        usub_sat.bind(vector(I8, sse_vector_size)),
        rec_fa.opcodes(&PSUBUSB),
    );
    e.enc_both_inferred(
        usub_sat.bind(vector(I16, sse_vector_size)),
        rec_fa.opcodes(&PSUBUSW),
    );

    // SIMD integer multiplication: the x86 ISA does not have instructions for multiplying I8x16
    // and I64x2 and these are (at the time of writing) not necessary for WASM SIMD.
    for (ty, opcodes, isap) in &[
        (I16, &PMULLW[..], None),
        (I32, &PMULLD[..], Some(use_sse41_simd)),
    ] {
        let imul = imul.bind(vector(*ty, sse_vector_size));
        e.enc_both_inferred_maybe_isap(imul, rec_fa.opcodes(opcodes), *isap);
    }

    // SIMD multiplication with lane expansion.
    e.enc_both_inferred(x86_pmuludq, rec_fa.opcodes(&PMULUDQ));

    // SIMD integer multiplication for I64x2 using a AVX512.
    {
        e.enc_32_64_maybe_isap(
            x86_pmullq,
            rec_evex_reg_vvvv_rm_128.opcodes(&VPMULLQ).w(),
            Some(use_avx512dq_simd), // TODO need an OR predicate to join with AVX512VL
        );
    }

    // SIMD integer average with rounding.
    for (ty, opcodes) in &[(I8, &PAVGB[..]), (I16, &PAVGW[..])] {
        let avgr = avg_round.bind(vector(*ty, sse_vector_size));
        e.enc_both_inferred(avgr, rec_fa.opcodes(opcodes));
    }

    // SIMD logical operations
    let band = shared.by_name("band");
    let band_not = shared.by_name("band_not");
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        // and
        let band = band.bind(vector(ty, sse_vector_size));
        e.enc_both_inferred(band, rec_fa.opcodes(&PAND));

        // and not (note flipped recipe operands to match band_not order)
        let band_not = band_not.bind(vector(ty, sse_vector_size));
        e.enc_both_inferred(band_not, rec_fax.opcodes(&PANDN));

        // or
        let bor = bor.bind(vector(ty, sse_vector_size));
        e.enc_both_inferred(bor, rec_fa.opcodes(&POR));

        // xor
        let bxor = bxor.bind(vector(ty, sse_vector_size));
        e.enc_both_inferred(bxor, rec_fa.opcodes(&PXOR));

        // ptest
        let x86_ptest = x86_ptest.bind(vector(ty, sse_vector_size));
        e.enc_both_inferred_maybe_isap(x86_ptest, rec_fcmp.opcodes(&PTEST), Some(use_sse41_simd));
    }

    // SIMD bitcast from I32/I64 to the low bits of a vector (e.g. I64x2); this register movement
    // allows SIMD shifts to be legalized more easily. TODO ideally this would be typed as an
    // I128x1 but restrictions on the type builder prevent this; the general idea here is that
    // the upper bits are all zeroed and do not form parts of any separate lane. See
    // https://github.com/bytecodealliance/wasmtime/issues/1140.
    e.enc_both_inferred(
        bitcast.bind(vector(I64, sse_vector_size)).bind(I32),
        rec_frurm.opcodes(&MOVD_LOAD_XMM),
    );
    e.enc64(
        bitcast.bind(vector(I64, sse_vector_size)).bind(I64),
        rec_frurm.opcodes(&MOVD_LOAD_XMM).rex().w(),
    );

    // SIMD shift left
    for (ty, opcodes) in &[(I16, &PSLLW), (I32, &PSLLD), (I64, &PSLLQ)] {
        let x86_psll = x86_psll.bind(vector(*ty, sse_vector_size));
        e.enc_both_inferred(x86_psll, rec_fa.opcodes(*opcodes));
    }

    // SIMD shift right (logical)
    for (ty, opcodes) in &[(I16, &PSRLW), (I32, &PSRLD), (I64, &PSRLQ)] {
        let x86_psrl = x86_psrl.bind(vector(*ty, sse_vector_size));
        e.enc_both_inferred(x86_psrl, rec_fa.opcodes(*opcodes));
    }

    // SIMD shift right (arithmetic)
    for (ty, opcodes) in &[(I16, &PSRAW), (I32, &PSRAD)] {
        let x86_psra = x86_psra.bind(vector(*ty, sse_vector_size));
        e.enc_both_inferred(x86_psra, rec_fa.opcodes(*opcodes));
    }

    // SIMD immediate shift
    for (ty, opcodes) in &[(I16, &PS_W_IMM), (I32, &PS_D_IMM), (I64, &PS_Q_IMM)] {
        let ishl_imm = ishl_imm.bind(vector(*ty, sse_vector_size));
        e.enc_both_inferred(ishl_imm, rec_f_ib.opcodes(*opcodes).rrr(6));

        let ushr_imm = ushr_imm.bind(vector(*ty, sse_vector_size));
        e.enc_both_inferred(ushr_imm, rec_f_ib.opcodes(*opcodes).rrr(2));

        // One exception: PSRAQ does not exist in for 64x2 in SSE2, it requires a higher CPU feature set.
        if *ty != I64 {
            let sshr_imm = sshr_imm.bind(vector(*ty, sse_vector_size));
            e.enc_both_inferred(sshr_imm, rec_f_ib.opcodes(*opcodes).rrr(4));
        }
    }

    // SIMD integer comparisons
    {
        use IntCC::*;
        for (ty, cc, opcodes, isa_predicate) in &[
            (I8, Equal, &PCMPEQB[..], None),
            (I16, Equal, &PCMPEQW[..], None),
            (I32, Equal, &PCMPEQD[..], None),
            (I64, Equal, &PCMPEQQ[..], Some(use_sse41_simd)),
            (I8, SignedGreaterThan, &PCMPGTB[..], None),
            (I16, SignedGreaterThan, &PCMPGTW[..], None),
            (I32, SignedGreaterThan, &PCMPGTD[..], None),
            (I64, SignedGreaterThan, &PCMPGTQ, Some(use_sse42_simd)),
        ] {
            let instruction = icmp
                .bind(Immediate::IntCC(*cc))
                .bind(vector(*ty, sse_vector_size));
            let template = rec_icscc_fpr.opcodes(opcodes);
            e.enc_both_inferred_maybe_isap(instruction, template, *isa_predicate);
        }
    }

    // SIMD min/max
    for (ty, inst, opcodes, isa_predicate) in &[
        (I8, x86_pmaxs, &PMAXSB[..], Some(use_sse41_simd)),
        (I16, x86_pmaxs, &PMAXSW[..], None),
        (I32, x86_pmaxs, &PMAXSD[..], Some(use_sse41_simd)),
        (I8, x86_pmaxu, &PMAXUB[..], None),
        (I16, x86_pmaxu, &PMAXUW[..], Some(use_sse41_simd)),
        (I32, x86_pmaxu, &PMAXUD[..], Some(use_sse41_simd)),
        (I8, x86_pmins, &PMINSB[..], Some(use_sse41_simd)),
        (I16, x86_pmins, &PMINSW[..], None),
        (I32, x86_pmins, &PMINSD[..], Some(use_sse41_simd)),
        (I8, x86_pminu, &PMINUB[..], None),
        (I16, x86_pminu, &PMINUW[..], Some(use_sse41_simd)),
        (I32, x86_pminu, &PMINUD[..], Some(use_sse41_simd)),
    ] {
        let inst = inst.bind(vector(*ty, sse_vector_size));
        e.enc_both_inferred_maybe_isap(inst, rec_fa.opcodes(opcodes), *isa_predicate);
    }

    // SIMD float comparisons
    e.enc_both_inferred(
        fcmp.bind(vector(F32, sse_vector_size)),
        rec_pfcmp.opcodes(&CMPPS),
    );
    e.enc_both_inferred(
        fcmp.bind(vector(F64, sse_vector_size)),
        rec_pfcmp.opcodes(&CMPPD),
    );

    // SIMD float arithmetic
    for (ty, inst, opcodes) in &[
        (F32, fadd, &ADDPS[..]),
        (F64, fadd, &ADDPD[..]),
        (F32, fsub, &SUBPS[..]),
        (F64, fsub, &SUBPD[..]),
        (F32, fmul, &MULPS[..]),
        (F64, fmul, &MULPD[..]),
        (F32, fdiv, &DIVPS[..]),
        (F64, fdiv, &DIVPD[..]),
        (F32, fmin, &MINPS[..]),
        (F64, fmin, &MINPD[..]),
        (F32, fmax, &MAXPS[..]),
        (F64, fmax, &MAXPD[..]),
    ] {
        let inst = inst.bind(vector(*ty, sse_vector_size));
        e.enc_both_inferred(inst, rec_fa.opcodes(opcodes));
    }
    for (ty, inst, opcodes) in &[(F32, sqrt, &SQRTPS[..]), (F64, sqrt, &SQRTPD[..])] {
        let inst = inst.bind(vector(*ty, sse_vector_size));
        e.enc_both_inferred(inst, rec_furm.opcodes(opcodes));
    }
}

#[inline(never)]
fn define_entity_ref(
    e: &mut PerCpuModeEncodings,
    shared_defs: &SharedDefinitions,
    settings: &SettingGroup,
    r: &RecipeGroup,
) {
    let shared = &shared_defs.instructions;
    let formats = &shared_defs.formats;

    // Shorthands for instructions.
    let const_addr = shared.by_name("const_addr");
    let func_addr = shared.by_name("func_addr");
    let stack_addr = shared.by_name("stack_addr");
    let symbol_value = shared.by_name("symbol_value");

    // Shorthands for recipes.
    let rec_allones_fnaddr4 = r.template("allones_fnaddr4");
    let rec_allones_fnaddr8 = r.template("allones_fnaddr8");
    let rec_fnaddr4 = r.template("fnaddr4");
    let rec_fnaddr8 = r.template("fnaddr8");
    let rec_const_addr = r.template("const_addr");
    let rec_got_fnaddr8 = r.template("got_fnaddr8");
    let rec_got_gvaddr8 = r.template("got_gvaddr8");
    let rec_gvaddr4 = r.template("gvaddr4");
    let rec_gvaddr8 = r.template("gvaddr8");
    let rec_pcrel_fnaddr8 = r.template("pcrel_fnaddr8");
    let rec_pcrel_gvaddr8 = r.template("pcrel_gvaddr8");
    let rec_spaddr_id = r.template("spaddr_id");

    // Predicates shorthands.
    let all_ones_funcaddrs_and_not_is_pic =
        settings.predicate_by_name("all_ones_funcaddrs_and_not_is_pic");
    let is_pic = settings.predicate_by_name("is_pic");
    let not_all_ones_funcaddrs_and_not_is_pic =
        settings.predicate_by_name("not_all_ones_funcaddrs_and_not_is_pic");
    let not_is_pic = settings.predicate_by_name("not_is_pic");

    // Function addresses.

    // Non-PIC, all-ones funcaddresses.
    e.enc32_isap(
        func_addr.bind(I32),
        rec_fnaddr4.opcodes(&MOV_IMM),
        not_all_ones_funcaddrs_and_not_is_pic,
    );
    e.enc64_isap(
        func_addr.bind(I64),
        rec_fnaddr8.opcodes(&MOV_IMM).rex().w(),
        not_all_ones_funcaddrs_and_not_is_pic,
    );

    // Non-PIC, all-zeros funcaddresses.
    e.enc32_isap(
        func_addr.bind(I32),
        rec_allones_fnaddr4.opcodes(&MOV_IMM),
        all_ones_funcaddrs_and_not_is_pic,
    );
    e.enc64_isap(
        func_addr.bind(I64),
        rec_allones_fnaddr8.opcodes(&MOV_IMM).rex().w(),
        all_ones_funcaddrs_and_not_is_pic,
    );

    // 64-bit, colocated, both PIC and non-PIC. Use the lea instruction's pc-relative field.
    let is_colocated_func =
        InstructionPredicate::new_is_colocated_func(&*formats.func_addr, "func_ref");
    e.enc64_instp(
        func_addr.bind(I64),
        rec_pcrel_fnaddr8.opcodes(&LEA).rex().w(),
        is_colocated_func,
    );

    // 64-bit, non-colocated, PIC.
    e.enc64_isap(
        func_addr.bind(I64),
        rec_got_fnaddr8.opcodes(&MOV_LOAD).rex().w(),
        is_pic,
    );

    // Global addresses.

    // Non-PIC.
    e.enc32_isap(
        symbol_value.bind(I32),
        rec_gvaddr4.opcodes(&MOV_IMM),
        not_is_pic,
    );
    e.enc64_isap(
        symbol_value.bind(I64),
        rec_gvaddr8.opcodes(&MOV_IMM).rex().w(),
        not_is_pic,
    );

    // PIC, colocated.
    e.enc64_func(
        symbol_value.bind(I64),
        rec_pcrel_gvaddr8.opcodes(&LEA).rex().w(),
        |encoding| {
            encoding
                .isa_predicate(is_pic)
                .inst_predicate(InstructionPredicate::new_is_colocated_data(formats))
        },
    );

    // PIC, non-colocated.
    e.enc64_isap(
        symbol_value.bind(I64),
        rec_got_gvaddr8.opcodes(&MOV_LOAD).rex().w(),
        is_pic,
    );

    // Stack addresses.
    //
    // TODO: Add encoding rules for stack_load and stack_store, so that they
    // don't get legalized to stack_addr + load/store.
    e.enc64(stack_addr.bind(I64), rec_spaddr_id.opcodes(&LEA).rex().w());
    e.enc32(stack_addr.bind(I32), rec_spaddr_id.opcodes(&LEA));

    // Constant addresses (PIC).
    e.enc64(const_addr.bind(I64), rec_const_addr.opcodes(&LEA).rex().w());
    e.enc32(const_addr.bind(I32), rec_const_addr.opcodes(&LEA));
}

/// Control flow opcodes.
#[inline(never)]
fn define_control_flow(
    e: &mut PerCpuModeEncodings,
    shared_defs: &SharedDefinitions,
    settings: &SettingGroup,
    r: &RecipeGroup,
) {
    let shared = &shared_defs.instructions;
    let formats = &shared_defs.formats;

    // Shorthands for instructions.
    let brff = shared.by_name("brff");
    let brif = shared.by_name("brif");
    let brnz = shared.by_name("brnz");
    let brz = shared.by_name("brz");
    let call = shared.by_name("call");
    let call_indirect = shared.by_name("call_indirect");
    let debugtrap = shared.by_name("debugtrap");
    let indirect_jump_table_br = shared.by_name("indirect_jump_table_br");
    let jump = shared.by_name("jump");
    let jump_table_base = shared.by_name("jump_table_base");
    let jump_table_entry = shared.by_name("jump_table_entry");
    let return_ = shared.by_name("return");
    let trap = shared.by_name("trap");
    let trapff = shared.by_name("trapff");
    let trapif = shared.by_name("trapif");
    let resumable_trap = shared.by_name("resumable_trap");

    // Shorthands for recipes.
    let rec_brfb = r.template("brfb");
    let rec_brfd = r.template("brfd");
    let rec_brib = r.template("brib");
    let rec_brid = r.template("brid");
    let rec_call_id = r.template("call_id");
    let rec_call_plt_id = r.template("call_plt_id");
    let rec_call_r = r.template("call_r");
    let rec_debugtrap = r.recipe("debugtrap");
    let rec_indirect_jmp = r.template("indirect_jmp");
    let rec_jmpb = r.template("jmpb");
    let rec_jmpd = r.template("jmpd");
    let rec_jt_base = r.template("jt_base");
    let rec_jt_entry = r.template("jt_entry");
    let rec_ret = r.template("ret");
    let rec_t8jccb_abcd = r.template("t8jccb_abcd");
    let rec_t8jccd_abcd = r.template("t8jccd_abcd");
    let rec_t8jccd_long = r.template("t8jccd_long");
    let rec_tjccb = r.template("tjccb");
    let rec_tjccd = r.template("tjccd");
    let rec_trap = r.template("trap");
    let rec_trapif = r.recipe("trapif");
    let rec_trapff = r.recipe("trapff");

    // Predicates shorthands.
    let is_pic = settings.predicate_by_name("is_pic");

    // Call/return

    // 32-bit, both PIC and non-PIC.
    e.enc32(call, rec_call_id.opcodes(&CALL_RELATIVE));

    // 64-bit, colocated, both PIC and non-PIC. Use the call instruction's pc-relative field.
    let is_colocated_func = InstructionPredicate::new_is_colocated_func(&*formats.call, "func_ref");
    e.enc64_instp(call, rec_call_id.opcodes(&CALL_RELATIVE), is_colocated_func);

    // 64-bit, non-colocated, PIC. There is no 64-bit non-colocated non-PIC version, since non-PIC
    // is currently using the large model, which requires calls be lowered to
    // func_addr+call_indirect.
    e.enc64_isap(call, rec_call_plt_id.opcodes(&CALL_RELATIVE), is_pic);

    e.enc32(
        call_indirect.bind(I32),
        rec_call_r.opcodes(&JUMP_ABSOLUTE).rrr(2),
    );
    e.enc64(
        call_indirect.bind(I64),
        rec_call_r.opcodes(&JUMP_ABSOLUTE).rrr(2).rex(),
    );
    e.enc64(
        call_indirect.bind(I64),
        rec_call_r.opcodes(&JUMP_ABSOLUTE).rrr(2),
    );

    e.enc32(return_, rec_ret.opcodes(&RET_NEAR));
    e.enc64(return_, rec_ret.opcodes(&RET_NEAR));

    // Branches.
    e.enc32(jump, rec_jmpb.opcodes(&JUMP_SHORT));
    e.enc64(jump, rec_jmpb.opcodes(&JUMP_SHORT));
    e.enc32(jump, rec_jmpd.opcodes(&JUMP_NEAR_RELATIVE));
    e.enc64(jump, rec_jmpd.opcodes(&JUMP_NEAR_RELATIVE));

    e.enc_both(brif, rec_brib.opcodes(&JUMP_SHORT_IF_OVERFLOW));
    e.enc_both(brif, rec_brid.opcodes(&JUMP_NEAR_IF_OVERFLOW));

    // Not all float condition codes are legal, see `supported_floatccs`.
    e.enc_both(brff, rec_brfb.opcodes(&JUMP_SHORT_IF_OVERFLOW));
    e.enc_both(brff, rec_brfd.opcodes(&JUMP_NEAR_IF_OVERFLOW));

    // Note that the tjccd opcode will be prefixed with 0x0f.
    e.enc_i32_i64_explicit_rex(brz, rec_tjccb.opcodes(&JUMP_SHORT_IF_EQUAL));
    e.enc_i32_i64_explicit_rex(brz, rec_tjccd.opcodes(&TEST_BYTE_REG));
    e.enc_i32_i64_explicit_rex(brnz, rec_tjccb.opcodes(&JUMP_SHORT_IF_NOT_EQUAL));
    e.enc_i32_i64_explicit_rex(brnz, rec_tjccd.opcodes(&TEST_REG));

    // Branch on a b1 value in a register only looks at the low 8 bits. See also
    // bint encodings below.
    //
    // Start with the worst-case encoding for X86_32 only. The register allocator
    // can't handle a branch with an ABCD-constrained operand.
    e.enc32(brz.bind(B1), rec_t8jccd_long.opcodes(&TEST_BYTE_REG));
    e.enc32(brnz.bind(B1), rec_t8jccd_long.opcodes(&TEST_REG));

    e.enc_both(brz.bind(B1), rec_t8jccb_abcd.opcodes(&JUMP_SHORT_IF_EQUAL));
    e.enc_both(brz.bind(B1), rec_t8jccd_abcd.opcodes(&TEST_BYTE_REG));
    e.enc_both(
        brnz.bind(B1),
        rec_t8jccb_abcd.opcodes(&JUMP_SHORT_IF_NOT_EQUAL),
    );
    e.enc_both(brnz.bind(B1), rec_t8jccd_abcd.opcodes(&TEST_REG));

    // Jump tables.
    e.enc64(
        jump_table_entry.bind(I64),
        rec_jt_entry.opcodes(&MOVSXD).rex().w(),
    );
    e.enc32(jump_table_entry.bind(I32), rec_jt_entry.opcodes(&MOV_LOAD));

    e.enc64(
        jump_table_base.bind(I64),
        rec_jt_base.opcodes(&LEA).rex().w(),
    );
    e.enc32(jump_table_base.bind(I32), rec_jt_base.opcodes(&LEA));

    e.enc_x86_64(
        indirect_jump_table_br.bind(I64),
        rec_indirect_jmp.opcodes(&JUMP_ABSOLUTE).rrr(4),
    );
    e.enc32(
        indirect_jump_table_br.bind(I32),
        rec_indirect_jmp.opcodes(&JUMP_ABSOLUTE).rrr(4),
    );

    // Trap as ud2
    e.enc32(trap, rec_trap.opcodes(&UNDEFINED2));
    e.enc64(trap, rec_trap.opcodes(&UNDEFINED2));
    e.enc32(resumable_trap, rec_trap.opcodes(&UNDEFINED2));
    e.enc64(resumable_trap, rec_trap.opcodes(&UNDEFINED2));

    // Debug trap as int3
    e.enc32_rec(debugtrap, rec_debugtrap, 0);
    e.enc64_rec(debugtrap, rec_debugtrap, 0);

    e.enc32_rec(trapif, rec_trapif, 0);
    e.enc64_rec(trapif, rec_trapif, 0);
    e.enc32_rec(trapff, rec_trapff, 0);
    e.enc64_rec(trapff, rec_trapff, 0);
}

/// Reference type instructions.
#[inline(never)]
fn define_reftypes(e: &mut PerCpuModeEncodings, shared_defs: &SharedDefinitions, r: &RecipeGroup) {
    let shared = &shared_defs.instructions;

    let is_null = shared.by_name("is_null");
    let is_invalid = shared.by_name("is_invalid");
    let null = shared.by_name("null");
    let safepoint = shared.by_name("safepoint");

    let rec_is_zero = r.template("is_zero");
    let rec_is_invalid = r.template("is_invalid");
    let rec_pu_id_ref = r.template("pu_id_ref");
    let rec_safepoint = r.recipe("safepoint");

    // Null references implemented as iconst 0.
    e.enc32(null.bind(R32), rec_pu_id_ref.opcodes(&MOV_IMM));

    e.enc64(null.bind(R64), rec_pu_id_ref.rex().opcodes(&MOV_IMM));
    e.enc64(null.bind(R64), rec_pu_id_ref.opcodes(&MOV_IMM));

    // is_null, implemented by testing whether the value is 0.
    e.enc_r32_r64_rex_only(is_null, rec_is_zero.opcodes(&TEST_REG));

    // is_invalid, implemented by testing whether the value is -1.
    e.enc_r32_r64_rex_only(is_invalid, rec_is_invalid.opcodes(&CMP_IMM8).rrr(7));

    // safepoint instruction calls sink, no actual encoding.
    e.enc32_rec(safepoint, rec_safepoint, 0);
    e.enc64_rec(safepoint, rec_safepoint, 0);
}

#[allow(clippy::cognitive_complexity)]
pub(crate) fn define(
    shared_defs: &SharedDefinitions,
    settings: &SettingGroup,
    x86: &InstructionGroup,
    r: &RecipeGroup,
) -> PerCpuModeEncodings {
    // Definitions.
    let mut e = PerCpuModeEncodings::new();

    define_moves(&mut e, shared_defs, r);
    define_memory(&mut e, shared_defs, x86, r);
    define_fpu_moves(&mut e, shared_defs, r);
    define_fpu_memory(&mut e, shared_defs, r);
    define_fpu_ops(&mut e, shared_defs, settings, x86, r);
    define_alu(&mut e, shared_defs, settings, x86, r);
    define_simd(&mut e, shared_defs, settings, x86, r);
    define_entity_ref(&mut e, shared_defs, settings, r);
    define_control_flow(&mut e, shared_defs, settings, r);
    define_reftypes(&mut e, shared_defs, r);

    let x86_elf_tls_get_addr = x86.by_name("x86_elf_tls_get_addr");
    let x86_macho_tls_get_addr = x86.by_name("x86_macho_tls_get_addr");

    let rec_elf_tls_get_addr = r.recipe("elf_tls_get_addr");
    let rec_macho_tls_get_addr = r.recipe("macho_tls_get_addr");

    e.enc64_rec(x86_elf_tls_get_addr, rec_elf_tls_get_addr, 0);
    e.enc64_rec(x86_macho_tls_get_addr, rec_macho_tls_get_addr, 0);

    e
}

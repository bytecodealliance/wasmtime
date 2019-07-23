#![allow(non_snake_case)]

use std::collections::HashMap;

use crate::cdsl::encodings::{Encoding, EncodingBuilder};
use crate::cdsl::instructions::{
    BoundInstruction, InstSpec, Instruction, InstructionGroup, InstructionPredicate,
    InstructionPredicateNode, InstructionPredicateRegistry,
};
use crate::cdsl::recipes::{EncodingRecipe, EncodingRecipeNumber, Recipes};
use crate::cdsl::settings::{SettingGroup, SettingPredicateNumber};
use crate::cdsl::types::ValueType;
use crate::shared::types::Bool::{B1, B16, B32, B64, B8};
use crate::shared::types::Float::{F32, F64};
use crate::shared::types::Int::{I16, I32, I64, I8};
use crate::shared::types::Reference::{R32, R64};
use crate::shared::Definitions as SharedDefinitions;

use super::recipes::{RecipeGroup, Template};

pub struct PerCpuModeEncodings {
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
        let builder = EncodingBuilder::new(inst.into(), recipe_number, bits);
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

    /// Add encodings for `inst.i32` to X86_32.
    /// Add encodings for `inst.i32` to X86_64 with and without REX.
    /// Add encodings for `inst.i64` to X86_64 with a REX.W prefix.
    fn enc_i32_i64(&mut self, inst: impl Into<InstSpec>, template: Template) {
        let inst: InstSpec = inst.into();
        self.enc32(inst.bind(I32), template.nonrex());

        // REX-less encoding must come after REX encoding so we don't use it by default. Otherwise
        // reg-alloc would never use r8 and up.
        self.enc64(inst.bind(I32), template.rex());
        self.enc64(inst.bind(I32), template.nonrex());
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
    /// Add encodings for `inst.r32` to X86_64 with and without REX.
    /// Add encodings for `inst.r64` to X86_64 with a REX.W prefix.
    fn enc_r32_r64(&mut self, inst: impl Into<InstSpec>, template: Template) {
        let inst: InstSpec = inst.into();
        self.enc32(inst.bind_ref(R32), template.nonrex());

        // REX-less encoding must come after REX encoding so we don't use it by default. Otherwise
        // reg-alloc would never use r8 and up.
        self.enc64(inst.bind_ref(R32), template.rex());
        self.enc64(inst.bind_ref(R32), template.nonrex());
        self.enc64(inst.bind_ref(R64), template.rex().w());
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
        inst: BoundInstruction,
        template: Template,
        isap: SettingPredicateNumber,
    ) {
        self.enc32_isap(inst.clone(), template.clone(), isap);
        self.enc_x86_64_isap(inst, template, isap);
    }
    fn enc_both_instp(
        &mut self,
        inst: BoundInstruction,
        template: Template,
        instp: InstructionPredicateNode,
    ) {
        self.enc32_instp(inst.clone(), template.clone(), instp.clone());
        self.enc_x86_64_instp(inst, template, instp);
    }

    /// Add encodings for `inst.i32` to X86_32.
    /// Add encodings for `inst.i32` to X86_64 with and without REX.
    /// Add encodings for `inst.i64` to X86_64 with a REX prefix, using the `w_bit`
    /// argument to determine whether or not to set the REX.W bit.
    fn enc_i32_i64_ld_st(&mut self, inst: &Instruction, w_bit: bool, template: Template) {
        self.enc32(inst.clone().bind(I32).bind_any(), template.clone());

        // REX-less encoding must come after REX encoding so we don't use it by
        // default. Otherwise reg-alloc would never use r8 and up.
        self.enc64(inst.clone().bind(I32).bind_any(), template.clone().rex());
        self.enc64(inst.clone().bind(I32).bind_any(), template.clone());

        if w_bit {
            self.enc64(inst.clone().bind(I64).bind_any(), template.rex().w());
        } else {
            self.enc64(inst.clone().bind(I64).bind_any(), template.clone().rex());
            self.enc64(inst.clone().bind(I64).bind_any(), template);
        }
    }

    /// Add the same encoding to both X86_32 and X86_64; assumes configuration (e.g. REX, operand binding) has already happened
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

pub fn define(
    shared_defs: &SharedDefinitions,
    settings: &SettingGroup,
    x86: &InstructionGroup,
    r: &RecipeGroup,
) -> PerCpuModeEncodings {
    let shared = &shared_defs.instructions;
    let formats = &shared_defs.format_registry;

    // Shorthands for instructions.
    let adjust_sp_down = shared.by_name("adjust_sp_down");
    let adjust_sp_down_imm = shared.by_name("adjust_sp_down_imm");
    let adjust_sp_up_imm = shared.by_name("adjust_sp_up_imm");
    let band = shared.by_name("band");
    let band_imm = shared.by_name("band_imm");
    let band_not = shared.by_name("band_not");
    let bconst = shared.by_name("bconst");
    let bint = shared.by_name("bint");
    let bitcast = shared.by_name("bitcast");
    let bnot = shared.by_name("bnot");
    let bor = shared.by_name("bor");
    let bor_imm = shared.by_name("bor_imm");
    let brff = shared.by_name("brff");
    let brif = shared.by_name("brif");
    let brnz = shared.by_name("brnz");
    let brz = shared.by_name("brz");
    let bxor = shared.by_name("bxor");
    let bxor_imm = shared.by_name("bxor_imm");
    let call = shared.by_name("call");
    let call_indirect = shared.by_name("call_indirect");
    let ceil = shared.by_name("ceil");
    let clz = shared.by_name("clz");
    let copy = shared.by_name("copy");
    let copy_nop = shared.by_name("copy_nop");
    let copy_special = shared.by_name("copy_special");
    let copy_to_ssa = shared.by_name("copy_to_ssa");
    let ctz = shared.by_name("ctz");
    let debugtrap = shared.by_name("debugtrap");
    let extractlane = shared.by_name("extractlane");
    let f32const = shared.by_name("f32const");
    let f64const = shared.by_name("f64const");
    let fadd = shared.by_name("fadd");
    let fcmp = shared.by_name("fcmp");
    let fcvt_from_sint = shared.by_name("fcvt_from_sint");
    let fdemote = shared.by_name("fdemote");
    let fdiv = shared.by_name("fdiv");
    let ffcmp = shared.by_name("ffcmp");
    let fill = shared.by_name("fill");
    let fill_nop = shared.by_name("fill_nop");
    let floor = shared.by_name("floor");
    let fmul = shared.by_name("fmul");
    let fpromote = shared.by_name("fpromote");
    let fsub = shared.by_name("fsub");
    let func_addr = shared.by_name("func_addr");
    let iadd = shared.by_name("iadd");
    let iadd_imm = shared.by_name("iadd_imm");
    let icmp = shared.by_name("icmp");
    let icmp_imm = shared.by_name("icmp_imm");
    let iconst = shared.by_name("iconst");
    let ifcmp = shared.by_name("ifcmp");
    let ifcmp_imm = shared.by_name("ifcmp_imm");
    let ifcmp_sp = shared.by_name("ifcmp_sp");
    let imul = shared.by_name("imul");
    let indirect_jump_table_br = shared.by_name("indirect_jump_table_br");
    let insertlane = shared.by_name("insertlane");
    let ireduce = shared.by_name("ireduce");
    let ishl = shared.by_name("ishl");
    let ishl_imm = shared.by_name("ishl_imm");
    let is_null = shared.by_name("is_null");
    let istore16 = shared.by_name("istore16");
    let istore16_complex = shared.by_name("istore16_complex");
    let istore32 = shared.by_name("istore32");
    let istore32_complex = shared.by_name("istore32_complex");
    let istore8 = shared.by_name("istore8");
    let istore8_complex = shared.by_name("istore8_complex");
    let isub = shared.by_name("isub");
    let jump = shared.by_name("jump");
    let jump_table_base = shared.by_name("jump_table_base");
    let jump_table_entry = shared.by_name("jump_table_entry");
    let load = shared.by_name("load");
    let load_complex = shared.by_name("load_complex");
    let nearest = shared.by_name("nearest");
    let null = shared.by_name("null");
    let popcnt = shared.by_name("popcnt");
    let raw_bitcast = shared.by_name("raw_bitcast");
    let regfill = shared.by_name("regfill");
    let regmove = shared.by_name("regmove");
    let regspill = shared.by_name("regspill");
    let return_ = shared.by_name("return");
    let rotl = shared.by_name("rotl");
    let rotl_imm = shared.by_name("rotl_imm");
    let rotr = shared.by_name("rotr");
    let rotr_imm = shared.by_name("rotr_imm");
    let safepoint = shared.by_name("safepoint");
    let scalar_to_vector = shared.by_name("scalar_to_vector");
    let selectif = shared.by_name("selectif");
    let sextend = shared.by_name("sextend");
    let sload16 = shared.by_name("sload16");
    let sload16_complex = shared.by_name("sload16_complex");
    let sload32 = shared.by_name("sload32");
    let sload32_complex = shared.by_name("sload32_complex");
    let sload8 = shared.by_name("sload8");
    let sload8_complex = shared.by_name("sload8_complex");
    let spill = shared.by_name("spill");
    let sqrt = shared.by_name("sqrt");
    let sshr = shared.by_name("sshr");
    let sshr_imm = shared.by_name("sshr_imm");
    let stack_addr = shared.by_name("stack_addr");
    let store = shared.by_name("store");
    let store_complex = shared.by_name("store_complex");
    let symbol_value = shared.by_name("symbol_value");
    let trap = shared.by_name("trap");
    let trapff = shared.by_name("trapff");
    let trapif = shared.by_name("trapif");
    let resumable_trap = shared.by_name("resumable_trap");
    let trueff = shared.by_name("trueff");
    let trueif = shared.by_name("trueif");
    let trunc = shared.by_name("trunc");
    let uextend = shared.by_name("uextend");
    let uload16 = shared.by_name("uload16");
    let uload16_complex = shared.by_name("uload16_complex");
    let uload32 = shared.by_name("uload32");
    let uload32_complex = shared.by_name("uload32_complex");
    let uload8 = shared.by_name("uload8");
    let uload8_complex = shared.by_name("uload8_complex");
    let ushr = shared.by_name("ushr");
    let ushr_imm = shared.by_name("ushr_imm");
    let vconst = shared.by_name("vconst");
    let x86_bsf = x86.by_name("x86_bsf");
    let x86_bsr = x86.by_name("x86_bsr");
    let x86_cvtt2si = x86.by_name("x86_cvtt2si");
    let x86_fmax = x86.by_name("x86_fmax");
    let x86_fmin = x86.by_name("x86_fmin");
    let x86_pop = x86.by_name("x86_pop");
    let x86_pshufd = x86.by_name("x86_pshufd");
    let x86_pshufb = x86.by_name("x86_pshufb");
    let x86_push = x86.by_name("x86_push");
    let x86_sdivmodx = x86.by_name("x86_sdivmodx");
    let x86_smulx = x86.by_name("x86_smulx");
    let x86_udivmodx = x86.by_name("x86_udivmodx");
    let x86_umulx = x86.by_name("x86_umulx");

    // Shorthands for recipes.
    let rec_adjustsp = r.template("adjustsp");
    let rec_adjustsp_ib = r.template("adjustsp_ib");
    let rec_adjustsp_id = r.template("adjustsp_id");
    let rec_allones_fnaddr4 = r.template("allones_fnaddr4");
    let rec_allones_fnaddr8 = r.template("allones_fnaddr8");
    let rec_brfb = r.template("brfb");
    let rec_brfd = r.template("brfd");
    let rec_brib = r.template("brib");
    let rec_brid = r.template("brid");
    let rec_bsf_and_bsr = r.template("bsf_and_bsr");
    let rec_call_id = r.template("call_id");
    let rec_call_plt_id = r.template("call_plt_id");
    let rec_call_r = r.template("call_r");
    let rec_cmov = r.template("cmov");
    let rec_copysp = r.template("copysp");
    let rec_div = r.template("div");
    let rec_debugtrap = r.recipe("debugtrap");
    let rec_f32imm_z = r.template("f32imm_z");
    let rec_f64imm_z = r.template("f64imm_z");
    let rec_fa = r.template("fa");
    let rec_fax = r.template("fax");
    let rec_fcmp = r.template("fcmp");
    let rec_fcscc = r.template("fcscc");
    let rec_ffillnull = r.recipe("ffillnull");
    let rec_ffillSib32 = r.template("ffillSib32");
    let rec_fillnull = r.recipe("fillnull");
    let rec_fillSib32 = r.template("fillSib32");
    let rec_fld = r.template("fld");
    let rec_fldDisp32 = r.template("fldDisp32");
    let rec_fldDisp8 = r.template("fldDisp8");
    let rec_fldWithIndex = r.template("fldWithIndex");
    let rec_fldWithIndexDisp32 = r.template("fldWithIndexDisp32");
    let rec_fldWithIndexDisp8 = r.template("fldWithIndexDisp8");
    let rec_fnaddr4 = r.template("fnaddr4");
    let rec_fnaddr8 = r.template("fnaddr8");
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
    let rec_furmi_rnd = r.template("furmi_rnd");
    let rec_got_fnaddr8 = r.template("got_fnaddr8");
    let rec_got_gvaddr8 = r.template("got_gvaddr8");
    let rec_gvaddr4 = r.template("gvaddr4");
    let rec_gvaddr8 = r.template("gvaddr8");
    let rec_icscc = r.template("icscc");
    let rec_icscc_ib = r.template("icscc_ib");
    let rec_icscc_id = r.template("icscc_id");
    let rec_indirect_jmp = r.template("indirect_jmp");
    let rec_is_zero = r.template("is_zero");
    let rec_jmpb = r.template("jmpb");
    let rec_jmpd = r.template("jmpd");
    let rec_jt_base = r.template("jt_base");
    let rec_jt_entry = r.template("jt_entry");
    let rec_ld = r.template("ld");
    let rec_ldDisp32 = r.template("ldDisp32");
    let rec_ldDisp8 = r.template("ldDisp8");
    let rec_ldWithIndex = r.template("ldWithIndex");
    let rec_ldWithIndexDisp32 = r.template("ldWithIndexDisp32");
    let rec_ldWithIndexDisp8 = r.template("ldWithIndexDisp8");
    let rec_mulx = r.template("mulx");
    let rec_null = r.recipe("null");
    let rec_null_fpr = r.recipe("null_fpr");
    let rec_pcrel_fnaddr8 = r.template("pcrel_fnaddr8");
    let rec_pcrel_gvaddr8 = r.template("pcrel_gvaddr8");
    let rec_popq = r.template("popq");
    let rec_pu_id = r.template("pu_id");
    let rec_pu_id_bool = r.template("pu_id_bool");
    let rec_pu_id_ref = r.template("pu_id_ref");
    let rec_pu_iq = r.template("pu_iq");
    let rec_pushq = r.template("pushq");
    let rec_ret = r.template("ret");
    let rec_r_ib = r.template("r_ib");
    let rec_r_ib_unsigned_gpr = r.template("r_ib_unsigned_gpr");
    let rec_r_ib_unsigned_fpr = r.template("r_ib_unsigned_fpr");
    let rec_r_ib_unsigned_r = r.template("r_ib_unsigned_r");
    let rec_r_id = r.template("r_id");
    let rec_rcmp = r.template("rcmp");
    let rec_rcmp_ib = r.template("rcmp_ib");
    let rec_rcmp_id = r.template("rcmp_id");
    let rec_rcmp_sp = r.template("rcmp_sp");
    let rec_regfill32 = r.template("regfill32");
    let rec_regspill32 = r.template("regspill32");
    let rec_rc = r.template("rc");
    let rec_rfumr = r.template("rfumr");
    let rec_rfurm = r.template("rfurm");
    let rec_rmov = r.template("rmov");
    let rec_rr = r.template("rr");
    let rec_rrx = r.template("rrx");
    let rec_safepoint = r.recipe("safepoint");
    let rec_setf_abcd = r.template("setf_abcd");
    let rec_seti_abcd = r.template("seti_abcd");
    let rec_spaddr4_id = r.template("spaddr4_id");
    let rec_spaddr8_id = r.template("spaddr8_id");
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
    let rec_t8jccb_abcd = r.template("t8jccb_abcd");
    let rec_t8jccd_abcd = r.template("t8jccd_abcd");
    let rec_t8jccd_long = r.template("t8jccd_long");
    let rec_tjccb = r.template("tjccb");
    let rec_tjccd = r.template("tjccd");
    let rec_trap = r.template("trap");
    let rec_trapif = r.recipe("trapif");
    let rec_trapff = r.recipe("trapff");
    let rec_u_id = r.template("u_id");
    let rec_umr = r.template("umr");
    let rec_umr_reg_to_ssa = r.template("umr_reg_to_ssa");
    let rec_ur = r.template("ur");
    let rec_urm = r.template("urm");
    let rec_urm_noflags = r.template("urm_noflags");
    let rec_urm_noflags_abcd = r.template("urm_noflags_abcd");
    let rec_vconst = r.template("vconst");

    // Predicates shorthands.
    let all_ones_funcaddrs_and_not_is_pic =
        settings.predicate_by_name("all_ones_funcaddrs_and_not_is_pic");
    let is_pic = settings.predicate_by_name("is_pic");
    let not_all_ones_funcaddrs_and_not_is_pic =
        settings.predicate_by_name("not_all_ones_funcaddrs_and_not_is_pic");
    let not_is_pic = settings.predicate_by_name("not_is_pic");
    let use_popcnt = settings.predicate_by_name("use_popcnt");
    let use_lzcnt = settings.predicate_by_name("use_lzcnt");
    let use_bmi1 = settings.predicate_by_name("use_bmi1");
    let use_sse41 = settings.predicate_by_name("use_sse41");
    let use_ssse3_simd = settings.predicate_by_name("use_ssse3_simd");
    let use_sse41_simd = settings.predicate_by_name("use_sse41_simd");

    // Definitions.
    let mut e = PerCpuModeEncodings::new();

    e.enc_i32_i64(iadd, rec_rr.opcodes(vec![0x01]));
    e.enc_i32_i64(isub, rec_rr.opcodes(vec![0x29]));
    e.enc_i32_i64(band, rec_rr.opcodes(vec![0x21]));
    e.enc_i32_i64(bor, rec_rr.opcodes(vec![0x09]));
    e.enc_i32_i64(bxor, rec_rr.opcodes(vec![0x31]));

    // x86 has a bitwise not instruction NOT.
    e.enc_i32_i64(bnot, rec_ur.opcodes(vec![0xf7]).rrr(2));

    // Also add a `b1` encodings for the logic instructions.
    // TODO: Should this be done with 8-bit instructions? It would improve partial register
    // dependencies.
    e.enc_both(band.bind(B1), rec_rr.opcodes(vec![0x21]));
    e.enc_both(bor.bind(B1), rec_rr.opcodes(vec![0x09]));
    e.enc_both(bxor.bind(B1), rec_rr.opcodes(vec![0x31]));

    e.enc_i32_i64(imul, rec_rrx.opcodes(vec![0x0f, 0xaf]));
    e.enc_i32_i64(x86_sdivmodx, rec_div.opcodes(vec![0xf7]).rrr(7));
    e.enc_i32_i64(x86_udivmodx, rec_div.opcodes(vec![0xf7]).rrr(6));

    e.enc_i32_i64(x86_smulx, rec_mulx.opcodes(vec![0xf7]).rrr(5));
    e.enc_i32_i64(x86_umulx, rec_mulx.opcodes(vec![0xf7]).rrr(4));

    e.enc_i32_i64(copy, rec_umr.opcodes(vec![0x89]));
    e.enc_r32_r64(copy, rec_umr.opcodes(vec![0x89]));
    e.enc_both(copy.bind(B1), rec_umr.opcodes(vec![0x89]));
    e.enc_both(copy.bind(I8), rec_umr.opcodes(vec![0x89]));
    e.enc_both(copy.bind(I16), rec_umr.opcodes(vec![0x89]));

    // TODO For x86-64, only define REX forms for now, since we can't describe the
    // special regunit immediate operands with the current constraint language.
    for &ty in &[I8, I16, I32] {
        e.enc32(regmove.bind(ty), rec_rmov.opcodes(vec![0x89]));
        e.enc64(regmove.bind(ty), rec_rmov.opcodes(vec![0x89]).rex());
    }
    e.enc64(regmove.bind(I64), rec_rmov.opcodes(vec![0x89]).rex().w());
    e.enc_both(regmove.bind(B1), rec_rmov.opcodes(vec![0x89]));
    e.enc_both(regmove.bind(I8), rec_rmov.opcodes(vec![0x89]));
    e.enc32(regmove.bind_ref(R32), rec_rmov.opcodes(vec![0x89]));
    e.enc64(regmove.bind_ref(R32), rec_rmov.opcodes(vec![0x89]).rex());
    e.enc64(
        regmove.bind_ref(R64),
        rec_rmov.opcodes(vec![0x89]).rex().w(),
    );

    e.enc_i32_i64(iadd_imm, rec_r_ib.opcodes(vec![0x83]).rrr(0));
    e.enc_i32_i64(iadd_imm, rec_r_id.opcodes(vec![0x81]).rrr(0));

    e.enc_i32_i64(band_imm, rec_r_ib.opcodes(vec![0x83]).rrr(4));
    e.enc_i32_i64(band_imm, rec_r_id.opcodes(vec![0x81]).rrr(4));

    e.enc_i32_i64(bor_imm, rec_r_ib.opcodes(vec![0x83]).rrr(1));
    e.enc_i32_i64(bor_imm, rec_r_id.opcodes(vec![0x81]).rrr(1));

    e.enc_i32_i64(bxor_imm, rec_r_ib.opcodes(vec![0x83]).rrr(6));
    e.enc_i32_i64(bxor_imm, rec_r_id.opcodes(vec![0x81]).rrr(6));

    // TODO: band_imm.i64 with an unsigned 32-bit immediate can be encoded as band_imm.i32. Can
    // even use the single-byte immediate for 0xffff_ffXX masks.

    // Immediate constants.
    e.enc32(iconst.bind(I32), rec_pu_id.opcodes(vec![0xb8]));

    e.enc64(iconst.bind(I32), rec_pu_id.rex().opcodes(vec![0xb8]));
    e.enc64(iconst.bind(I32), rec_pu_id.opcodes(vec![0xb8]));

    // The 32-bit immediate movl also zero-extends to 64 bits.
    let f_unary_imm = formats.get(formats.by_name("UnaryImm"));
    let is_unsigned_int32 = InstructionPredicate::new_is_unsigned_int(f_unary_imm, "imm", 32, 0);

    e.enc64_func(
        iconst.bind(I64),
        rec_pu_id.opcodes(vec![0xb8]).rex(),
        |encoding| encoding.inst_predicate(is_unsigned_int32.clone()),
    );
    e.enc64_func(
        iconst.bind(I64),
        rec_pu_id.opcodes(vec![0xb8]),
        |encoding| encoding.inst_predicate(is_unsigned_int32),
    );

    // Sign-extended 32-bit immediate.
    e.enc64(
        iconst.bind(I64),
        rec_u_id.rex().opcodes(vec![0xc7]).rrr(0).w(),
    );

    // Finally, the 0xb8 opcode takes an 8-byte immediate with a REX.W prefix.
    e.enc64(iconst.bind(I64), rec_pu_iq.opcodes(vec![0xb8]).rex().w());

    // Bool constants (uses MOV)
    for &ty in &[B1, B8, B16, B32] {
        e.enc_both(bconst.bind(ty), rec_pu_id_bool.opcodes(vec![0xb8]));
    }
    e.enc64(bconst.bind(B64), rec_pu_id_bool.opcodes(vec![0xb8]).rex());

    // Shifts and rotates.
    // Note that the dynamic shift amount is only masked by 5 or 6 bits; the 8-bit
    // and 16-bit shifts would need explicit masking.

    for &(inst, rrr) in &[(rotl, 0), (rotr, 1), (ishl, 4), (ushr, 5), (sshr, 7)] {
        // Cannot use enc_i32_i64 for this pattern because instructions require
        // to bind any.
        e.enc32(
            inst.bind(I32).bind_any(),
            rec_rc.opcodes(vec![0xd3]).rrr(rrr),
        );
        e.enc64(
            inst.bind(I64).bind_any(),
            rec_rc.opcodes(vec![0xd3]).rrr(rrr).rex().w(),
        );
        e.enc64(
            inst.bind(I32).bind_any(),
            rec_rc.opcodes(vec![0xd3]).rrr(rrr).rex(),
        );
        e.enc64(
            inst.bind(I32).bind_any(),
            rec_rc.opcodes(vec![0xd3]).rrr(rrr),
        );
    }

    for &(inst, rrr) in &[
        (rotl_imm, 0),
        (rotr_imm, 1),
        (ishl_imm, 4),
        (ushr_imm, 5),
        (sshr_imm, 7),
    ] {
        e.enc_i32_i64(inst, rec_r_ib.opcodes(vec![0xc1]).rrr(rrr));
    }

    // Population count.
    e.enc32_isap(
        popcnt.bind(I32),
        rec_urm.opcodes(vec![0xf3, 0x0f, 0xb8]),
        use_popcnt,
    );
    e.enc64_isap(
        popcnt.bind(I64),
        rec_urm.opcodes(vec![0xf3, 0x0f, 0xb8]).rex().w(),
        use_popcnt,
    );
    e.enc64_isap(
        popcnt.bind(I32),
        rec_urm.opcodes(vec![0xf3, 0x0f, 0xb8]).rex(),
        use_popcnt,
    );
    e.enc64_isap(
        popcnt.bind(I32),
        rec_urm.opcodes(vec![0xf3, 0x0f, 0xb8]),
        use_popcnt,
    );

    // Count leading zero bits.
    e.enc32_isap(
        clz.bind(I32),
        rec_urm.opcodes(vec![0xf3, 0x0f, 0xbd]),
        use_lzcnt,
    );
    e.enc64_isap(
        clz.bind(I64),
        rec_urm.opcodes(vec![0xf3, 0x0f, 0xbd]).rex().w(),
        use_lzcnt,
    );
    e.enc64_isap(
        clz.bind(I32),
        rec_urm.opcodes(vec![0xf3, 0x0f, 0xbd]).rex(),
        use_lzcnt,
    );
    e.enc64_isap(
        clz.bind(I32),
        rec_urm.opcodes(vec![0xf3, 0x0f, 0xbd]),
        use_lzcnt,
    );

    // Count trailing zero bits.
    e.enc32_isap(
        ctz.bind(I32),
        rec_urm.opcodes(vec![0xf3, 0x0f, 0xbc]),
        use_bmi1,
    );
    e.enc64_isap(
        ctz.bind(I64),
        rec_urm.opcodes(vec![0xf3, 0x0f, 0xbc]).rex().w(),
        use_bmi1,
    );
    e.enc64_isap(
        ctz.bind(I32),
        rec_urm.opcodes(vec![0xf3, 0x0f, 0xbc]).rex(),
        use_bmi1,
    );
    e.enc64_isap(
        ctz.bind(I32),
        rec_urm.opcodes(vec![0xf3, 0x0f, 0xbc]),
        use_bmi1,
    );

    // Loads and stores.
    let f_load_complex = formats.get(formats.by_name("LoadComplex"));
    let is_load_complex_length_two = InstructionPredicate::new_length_equals(f_load_complex, 2);

    for recipe in &[rec_ldWithIndex, rec_ldWithIndexDisp8, rec_ldWithIndexDisp32] {
        e.enc_i32_i64_instp(
            load_complex,
            recipe.opcodes(vec![0x8b]),
            is_load_complex_length_two.clone(),
        );
        e.enc_x86_64_instp(
            uload32_complex,
            recipe.opcodes(vec![0x8b]),
            is_load_complex_length_two.clone(),
        );

        e.enc64_instp(
            sload32_complex,
            recipe.opcodes(vec![0x63]).rex().w(),
            is_load_complex_length_two.clone(),
        );

        e.enc_i32_i64_instp(
            uload16_complex,
            recipe.opcodes(vec![0x0f, 0xb7]),
            is_load_complex_length_two.clone(),
        );
        e.enc_i32_i64_instp(
            sload16_complex,
            recipe.opcodes(vec![0x0f, 0xbf]),
            is_load_complex_length_two.clone(),
        );

        e.enc_i32_i64_instp(
            uload8_complex,
            recipe.opcodes(vec![0x0f, 0xb6]),
            is_load_complex_length_two.clone(),
        );

        e.enc_i32_i64_instp(
            sload8_complex,
            recipe.opcodes(vec![0x0f, 0xbe]),
            is_load_complex_length_two.clone(),
        );
    }

    let f_store_complex = formats.get(formats.by_name("StoreComplex"));
    let is_store_complex_length_three = InstructionPredicate::new_length_equals(f_store_complex, 3);

    for recipe in &[rec_stWithIndex, rec_stWithIndexDisp8, rec_stWithIndexDisp32] {
        e.enc_i32_i64_instp(
            store_complex,
            recipe.opcodes(vec![0x89]),
            is_store_complex_length_three.clone(),
        );
        e.enc_x86_64_instp(
            istore32_complex,
            recipe.opcodes(vec![0x89]),
            is_store_complex_length_three.clone(),
        );
        e.enc_both_instp(
            istore16_complex.bind(I32),
            recipe.opcodes(vec![0x66, 0x89]),
            is_store_complex_length_three.clone(),
        );
        e.enc_x86_64_instp(
            istore16_complex.bind(I64),
            recipe.opcodes(vec![0x66, 0x89]),
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
            recipe.opcodes(vec![0x88]),
            is_store_complex_length_three.clone(),
        );
        e.enc_x86_64_instp(
            istore8_complex.bind(I64),
            recipe.opcodes(vec![0x88]),
            is_store_complex_length_three.clone(),
        );
    }

    for recipe in &[rec_st, rec_stDisp8, rec_stDisp32] {
        e.enc_i32_i64_ld_st(store, true, recipe.opcodes(vec![0x89]));
        e.enc_x86_64(istore32.bind(I64).bind_any(), recipe.opcodes(vec![0x89]));
        e.enc_i32_i64_ld_st(istore16, false, recipe.opcodes(vec![0x66, 0x89]));
    }

    // Byte stores are more complicated because the registers they can address
    // depends of the presence of a REX prefix. The st*_abcd recipes fall back to
    // the corresponding st* recipes when a REX prefix is applied.

    for recipe in &[rec_st_abcd, rec_stDisp8_abcd, rec_stDisp32_abcd] {
        e.enc_both(istore8.bind(I32).bind_any(), recipe.opcodes(vec![0x88]));
        e.enc_x86_64(istore8.bind(I64).bind_any(), recipe.opcodes(vec![0x88]));
    }

    e.enc_i32_i64(spill, rec_spillSib32.opcodes(vec![0x89]));
    e.enc_i32_i64(regspill, rec_regspill32.opcodes(vec![0x89]));
    e.enc_r32_r64(spill, rec_spillSib32.opcodes(vec![0x89]));
    e.enc_r32_r64(regspill, rec_regspill32.opcodes(vec![0x89]));

    // Use a 32-bit write for spilling `b1`, `i8` and `i16` to avoid
    // constraining the permitted registers.
    // See MIN_SPILL_SLOT_SIZE which makes this safe.

    e.enc_both(spill.bind(B1), rec_spillSib32.opcodes(vec![0x89]));
    e.enc_both(regspill.bind(B1), rec_regspill32.opcodes(vec![0x89]));
    for &ty in &[I8, I16] {
        e.enc_both(spill.bind(ty), rec_spillSib32.opcodes(vec![0x89]));
        e.enc_both(regspill.bind(ty), rec_regspill32.opcodes(vec![0x89]));
    }

    for recipe in &[rec_ld, rec_ldDisp8, rec_ldDisp32] {
        e.enc_i32_i64_ld_st(load, true, recipe.opcodes(vec![0x8b]));
        e.enc_x86_64(uload32.bind(I64), recipe.opcodes(vec![0x8b]));
        e.enc64(sload32.bind(I64), recipe.opcodes(vec![0x63]).rex().w());
        e.enc_i32_i64_ld_st(uload16, true, recipe.opcodes(vec![0x0f, 0xb7]));
        e.enc_i32_i64_ld_st(sload16, true, recipe.opcodes(vec![0x0f, 0xbf]));
        e.enc_i32_i64_ld_st(uload8, true, recipe.opcodes(vec![0x0f, 0xb6]));
        e.enc_i32_i64_ld_st(sload8, true, recipe.opcodes(vec![0x0f, 0xbe]));
    }

    e.enc_i32_i64(fill, rec_fillSib32.opcodes(vec![0x8b]));
    e.enc_i32_i64(regfill, rec_regfill32.opcodes(vec![0x8b]));
    e.enc_r32_r64(fill, rec_fillSib32.opcodes(vec![0x8b]));
    e.enc_r32_r64(regfill, rec_regfill32.opcodes(vec![0x8b]));

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

    e.enc_both(fill.bind(B1), rec_fillSib32.opcodes(vec![0x8b]));
    e.enc_both(regfill.bind(B1), rec_regfill32.opcodes(vec![0x8b]));
    for &ty in &[I8, I16] {
        e.enc_both(fill.bind(ty), rec_fillSib32.opcodes(vec![0x8b]));
        e.enc_both(regfill.bind(ty), rec_regfill32.opcodes(vec![0x8b]));
    }

    // Push and Pop.
    e.enc32(x86_push.bind(I32), rec_pushq.opcodes(vec![0x50]));
    e.enc_x86_64(x86_push.bind(I64), rec_pushq.opcodes(vec![0x50]));

    e.enc32(x86_pop.bind(I32), rec_popq.opcodes(vec![0x58]));
    e.enc_x86_64(x86_pop.bind(I64), rec_popq.opcodes(vec![0x58]));

    // Copy Special
    // For x86-64, only define REX forms for now, since we can't describe the
    // special regunit immediate operands with the current constraint language.
    e.enc64(copy_special, rec_copysp.opcodes(vec![0x89]).rex().w());
    e.enc32(copy_special, rec_copysp.opcodes(vec![0x89]));

    // Copy to SSA
    e.enc_i32_i64(copy_to_ssa, rec_umr_reg_to_ssa.opcodes(vec![0x89]));
    e.enc_r32_r64(copy_to_ssa, rec_umr_reg_to_ssa.opcodes(vec![0x89]));
    e.enc_both(copy_to_ssa.bind(B1), rec_umr_reg_to_ssa.opcodes(vec![0x89]));
    e.enc_both(copy_to_ssa.bind(I8), rec_umr_reg_to_ssa.opcodes(vec![0x89]));
    e.enc_both(
        copy_to_ssa.bind(I16),
        rec_umr_reg_to_ssa.opcodes(vec![0x89]),
    );
    e.enc_both(
        copy_to_ssa.bind(F64),
        rec_furm_reg_to_ssa.opcodes(vec![0xf2, 0x0f, 0x10]),
    );
    e.enc_both(
        copy_to_ssa.bind(F32),
        rec_furm_reg_to_ssa.opcodes(vec![0xf3, 0x0f, 0x10]),
    );

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
    e.enc32(adjust_sp_down.bind(I32), rec_adjustsp.opcodes(vec![0x29]));
    e.enc64(
        adjust_sp_down.bind(I64),
        rec_adjustsp.opcodes(vec![0x29]).rex().w(),
    );

    // Adjust SP up by an immediate (or down, with a negative immediate).
    e.enc32(adjust_sp_up_imm, rec_adjustsp_ib.opcodes(vec![0x83]));
    e.enc32(adjust_sp_up_imm, rec_adjustsp_id.opcodes(vec![0x81]));
    e.enc64(
        adjust_sp_up_imm,
        rec_adjustsp_ib.opcodes(vec![0x83]).rex().w(),
    );
    e.enc64(
        adjust_sp_up_imm,
        rec_adjustsp_id.opcodes(vec![0x81]).rex().w(),
    );

    // Adjust SP down by an immediate (or up, with a negative immediate).
    e.enc32(
        adjust_sp_down_imm,
        rec_adjustsp_ib.opcodes(vec![0x83]).rrr(5),
    );
    e.enc32(
        adjust_sp_down_imm,
        rec_adjustsp_id.opcodes(vec![0x81]).rrr(5),
    );
    e.enc64(
        adjust_sp_down_imm,
        rec_adjustsp_ib.opcodes(vec![0x83]).rrr(5).rex().w(),
    );
    e.enc64(
        adjust_sp_down_imm,
        rec_adjustsp_id.opcodes(vec![0x81]).rrr(5).rex().w(),
    );

    // Float loads and stores.
    e.enc_both(
        load.bind(F32).bind_any(),
        rec_fld.opcodes(vec![0xf3, 0x0f, 0x10]),
    );
    e.enc_both(
        load.bind(F32).bind_any(),
        rec_fldDisp8.opcodes(vec![0xf3, 0x0f, 0x10]),
    );
    e.enc_both(
        load.bind(F32).bind_any(),
        rec_fldDisp32.opcodes(vec![0xf3, 0x0f, 0x10]),
    );

    e.enc_both(
        load_complex.bind(F32),
        rec_fldWithIndex.opcodes(vec![0xf3, 0x0f, 0x10]),
    );
    e.enc_both(
        load_complex.bind(F32),
        rec_fldWithIndexDisp8.opcodes(vec![0xf3, 0x0f, 0x10]),
    );
    e.enc_both(
        load_complex.bind(F32),
        rec_fldWithIndexDisp32.opcodes(vec![0xf3, 0x0f, 0x10]),
    );

    e.enc_both(
        load.bind(F64).bind_any(),
        rec_fld.opcodes(vec![0xf2, 0x0f, 0x10]),
    );
    e.enc_both(
        load.bind(F64).bind_any(),
        rec_fldDisp8.opcodes(vec![0xf2, 0x0f, 0x10]),
    );
    e.enc_both(
        load.bind(F64).bind_any(),
        rec_fldDisp32.opcodes(vec![0xf2, 0x0f, 0x10]),
    );

    e.enc_both(
        load_complex.bind(F64),
        rec_fldWithIndex.opcodes(vec![0xf2, 0x0f, 0x10]),
    );
    e.enc_both(
        load_complex.bind(F64),
        rec_fldWithIndexDisp8.opcodes(vec![0xf2, 0x0f, 0x10]),
    );
    e.enc_both(
        load_complex.bind(F64),
        rec_fldWithIndexDisp32.opcodes(vec![0xf2, 0x0f, 0x10]),
    );

    e.enc_both(
        store.bind(F32).bind_any(),
        rec_fst.opcodes(vec![0xf3, 0x0f, 0x11]),
    );
    e.enc_both(
        store.bind(F32).bind_any(),
        rec_fstDisp8.opcodes(vec![0xf3, 0x0f, 0x11]),
    );
    e.enc_both(
        store.bind(F32).bind_any(),
        rec_fstDisp32.opcodes(vec![0xf3, 0x0f, 0x11]),
    );

    e.enc_both(
        store_complex.bind(F32),
        rec_fstWithIndex.opcodes(vec![0xf3, 0x0f, 0x11]),
    );
    e.enc_both(
        store_complex.bind(F32),
        rec_fstWithIndexDisp8.opcodes(vec![0xf3, 0x0f, 0x11]),
    );
    e.enc_both(
        store_complex.bind(F32),
        rec_fstWithIndexDisp32.opcodes(vec![0xf3, 0x0f, 0x11]),
    );

    e.enc_both(
        store.bind(F64).bind_any(),
        rec_fst.opcodes(vec![0xf2, 0x0f, 0x11]),
    );
    e.enc_both(
        store.bind(F64).bind_any(),
        rec_fstDisp8.opcodes(vec![0xf2, 0x0f, 0x11]),
    );
    e.enc_both(
        store.bind(F64).bind_any(),
        rec_fstDisp32.opcodes(vec![0xf2, 0x0f, 0x11]),
    );

    e.enc_both(
        store_complex.bind(F64),
        rec_fstWithIndex.opcodes(vec![0xf2, 0x0f, 0x11]),
    );
    e.enc_both(
        store_complex.bind(F64),
        rec_fstWithIndexDisp8.opcodes(vec![0xf2, 0x0f, 0x11]),
    );
    e.enc_both(
        store_complex.bind(F64),
        rec_fstWithIndexDisp32.opcodes(vec![0xf2, 0x0f, 0x11]),
    );

    e.enc_both(
        fill.bind(F32),
        rec_ffillSib32.opcodes(vec![0xf3, 0x0f, 0x10]),
    );
    e.enc_both(
        regfill.bind(F32),
        rec_fregfill32.opcodes(vec![0xf3, 0x0f, 0x10]),
    );
    e.enc_both(
        fill.bind(F64),
        rec_ffillSib32.opcodes(vec![0xf2, 0x0f, 0x10]),
    );
    e.enc_both(
        regfill.bind(F64),
        rec_fregfill32.opcodes(vec![0xf2, 0x0f, 0x10]),
    );

    e.enc_both(
        spill.bind(F32),
        rec_fspillSib32.opcodes(vec![0xf3, 0x0f, 0x11]),
    );
    e.enc_both(
        regspill.bind(F32),
        rec_fregspill32.opcodes(vec![0xf3, 0x0f, 0x11]),
    );
    e.enc_both(
        spill.bind(F64),
        rec_fspillSib32.opcodes(vec![0xf2, 0x0f, 0x11]),
    );
    e.enc_both(
        regspill.bind(F64),
        rec_fregspill32.opcodes(vec![0xf2, 0x0f, 0x11]),
    );

    // Function addresses.

    // Non-PIC, all-ones funcaddresses.
    e.enc32_isap(
        func_addr.bind(I32),
        rec_fnaddr4.opcodes(vec![0xb8]),
        not_all_ones_funcaddrs_and_not_is_pic,
    );
    e.enc64_isap(
        func_addr.bind(I64),
        rec_fnaddr8.opcodes(vec![0xb8]).rex().w(),
        not_all_ones_funcaddrs_and_not_is_pic,
    );

    // Non-PIC, all-zeros funcaddresses.
    e.enc32_isap(
        func_addr.bind(I32),
        rec_allones_fnaddr4.opcodes(vec![0xb8]),
        all_ones_funcaddrs_and_not_is_pic,
    );
    e.enc64_isap(
        func_addr.bind(I64),
        rec_allones_fnaddr8.opcodes(vec![0xb8]).rex().w(),
        all_ones_funcaddrs_and_not_is_pic,
    );

    // 64-bit, colocated, both PIC and non-PIC. Use the lea instruction's pc-relative field.
    let f_func_addr = formats.get(formats.by_name("FuncAddr"));
    let is_colocated_func = InstructionPredicate::new_is_colocated_func(f_func_addr, "func_ref");
    e.enc64_instp(
        func_addr.bind(I64),
        rec_pcrel_fnaddr8.opcodes(vec![0x8d]).rex().w(),
        is_colocated_func,
    );

    // 64-bit, non-colocated, PIC.
    e.enc64_isap(
        func_addr.bind(I64),
        rec_got_fnaddr8.opcodes(vec![0x8b]).rex().w(),
        is_pic,
    );

    // Global addresses.

    // Non-PIC.
    e.enc32_isap(
        symbol_value.bind(I32),
        rec_gvaddr4.opcodes(vec![0xb8]),
        not_is_pic,
    );
    e.enc64_isap(
        symbol_value.bind(I64),
        rec_gvaddr8.opcodes(vec![0xb8]).rex().w(),
        not_is_pic,
    );

    // PIC, colocated.
    e.enc64_func(
        symbol_value.bind(I64),
        rec_pcrel_gvaddr8.opcodes(vec![0x8d]).rex().w(),
        |encoding| {
            encoding
                .isa_predicate(is_pic)
                .inst_predicate(InstructionPredicate::new_is_colocated_data(formats))
        },
    );

    // PIC, non-colocated.
    e.enc64_isap(
        symbol_value.bind(I64),
        rec_got_gvaddr8.opcodes(vec![0x8b]).rex().w(),
        is_pic,
    );

    // Stack addresses.
    //
    // TODO: Add encoding rules for stack_load and stack_store, so that they
    // don't get legalized to stack_addr + load/store.
    e.enc32(stack_addr.bind(I32), rec_spaddr4_id.opcodes(vec![0x8d]));
    e.enc64(
        stack_addr.bind(I64),
        rec_spaddr8_id.opcodes(vec![0x8d]).rex().w(),
    );

    // Call/return

    // 32-bit, both PIC and non-PIC.
    e.enc32(call, rec_call_id.opcodes(vec![0xe8]));

    // 64-bit, colocated, both PIC and non-PIC. Use the call instruction's pc-relative field.
    let f_call = formats.get(formats.by_name("Call"));
    let is_colocated_func = InstructionPredicate::new_is_colocated_func(f_call, "func_ref");
    e.enc64_instp(call, rec_call_id.opcodes(vec![0xe8]), is_colocated_func);

    // 64-bit, non-colocated, PIC. There is no 64-bit non-colocated non-PIC version, since non-PIC
    // is currently using the large model, which requires calls be lowered to
    // func_addr+call_indirect.
    e.enc64_isap(call, rec_call_plt_id.opcodes(vec![0xe8]), is_pic);

    e.enc32(
        call_indirect.bind(I32),
        rec_call_r.opcodes(vec![0xff]).rrr(2),
    );
    e.enc64(
        call_indirect.bind(I64),
        rec_call_r.opcodes(vec![0xff]).rrr(2).rex(),
    );
    e.enc64(
        call_indirect.bind(I64),
        rec_call_r.opcodes(vec![0xff]).rrr(2),
    );

    e.enc32(return_, rec_ret.opcodes(vec![0xc3]));
    e.enc64(return_, rec_ret.opcodes(vec![0xc3]));

    // Branches.
    e.enc32(jump, rec_jmpb.opcodes(vec![0xeb]));
    e.enc64(jump, rec_jmpb.opcodes(vec![0xeb]));
    e.enc32(jump, rec_jmpd.opcodes(vec![0xe9]));
    e.enc64(jump, rec_jmpd.opcodes(vec![0xe9]));

    e.enc_both(brif, rec_brib.opcodes(vec![0x70]));
    e.enc_both(brif, rec_brid.opcodes(vec![0x0f, 0x80]));

    // Not all float condition codes are legal, see `supported_floatccs`.
    e.enc_both(brff, rec_brfb.opcodes(vec![0x70]));
    e.enc_both(brff, rec_brfd.opcodes(vec![0x0f, 0x80]));

    // Note that the tjccd opcode will be prefixed with 0x0f.
    e.enc_i32_i64(brz, rec_tjccb.opcodes(vec![0x74]));
    e.enc_i32_i64(brz, rec_tjccd.opcodes(vec![0x84]));
    e.enc_i32_i64(brnz, rec_tjccb.opcodes(vec![0x75]));
    e.enc_i32_i64(brnz, rec_tjccd.opcodes(vec![0x85]));

    // Branch on a b1 value in a register only looks at the low 8 bits. See also
    // bint encodings below.
    //
    // Start with the worst-case encoding for X86_32 only. The register allocator
    // can't handle a branch with an ABCD-constrained operand.
    e.enc32(brz.bind(B1), rec_t8jccd_long.opcodes(vec![0x84]));
    e.enc32(brnz.bind(B1), rec_t8jccd_long.opcodes(vec![0x85]));

    e.enc_both(brz.bind(B1), rec_t8jccb_abcd.opcodes(vec![0x74]));
    e.enc_both(brz.bind(B1), rec_t8jccd_abcd.opcodes(vec![0x84]));
    e.enc_both(brnz.bind(B1), rec_t8jccb_abcd.opcodes(vec![0x75]));
    e.enc_both(brnz.bind(B1), rec_t8jccd_abcd.opcodes(vec![0x85]));

    // Jump tables.
    e.enc64(
        jump_table_entry.bind(I64),
        rec_jt_entry.opcodes(vec![0x63]).rex().w(),
    );
    e.enc32(jump_table_entry.bind(I32), rec_jt_entry.opcodes(vec![0x8b]));

    e.enc64(
        jump_table_base.bind(I64),
        rec_jt_base.opcodes(vec![0x8d]).rex().w(),
    );
    e.enc32(jump_table_base.bind(I32), rec_jt_base.opcodes(vec![0x8d]));

    e.enc_x86_64(
        indirect_jump_table_br.bind(I64),
        rec_indirect_jmp.opcodes(vec![0xff]).rrr(4),
    );
    e.enc32(
        indirect_jump_table_br.bind(I32),
        rec_indirect_jmp.opcodes(vec![0xff]).rrr(4),
    );

    // Trap as ud2
    e.enc32(trap, rec_trap.opcodes(vec![0x0f, 0x0b]));
    e.enc64(trap, rec_trap.opcodes(vec![0x0f, 0x0b]));
    e.enc32(resumable_trap, rec_trap.opcodes(vec![0x0f, 0x0b]));
    e.enc64(resumable_trap, rec_trap.opcodes(vec![0x0f, 0x0b]));

    // Debug trap as int3
    e.enc32_rec(debugtrap, rec_debugtrap, 0);
    e.enc64_rec(debugtrap, rec_debugtrap, 0);

    e.enc32_rec(trapif, rec_trapif, 0);
    e.enc64_rec(trapif, rec_trapif, 0);
    e.enc32_rec(trapff, rec_trapff, 0);
    e.enc64_rec(trapff, rec_trapff, 0);

    // Comparisons
    e.enc_i32_i64(icmp, rec_icscc.opcodes(vec![0x39]));
    e.enc_i32_i64(icmp_imm, rec_icscc_ib.opcodes(vec![0x83]).rrr(7));
    e.enc_i32_i64(icmp_imm, rec_icscc_id.opcodes(vec![0x81]).rrr(7));
    e.enc_i32_i64(ifcmp, rec_rcmp.opcodes(vec![0x39]));
    e.enc_i32_i64(ifcmp_imm, rec_rcmp_ib.opcodes(vec![0x83]).rrr(7));
    e.enc_i32_i64(ifcmp_imm, rec_rcmp_id.opcodes(vec![0x81]).rrr(7));
    // TODO: We could special-case ifcmp_imm(x, 0) to TEST(x, x).

    e.enc32(ifcmp_sp.bind(I32), rec_rcmp_sp.opcodes(vec![0x39]));
    e.enc64(
        ifcmp_sp.bind(I64),
        rec_rcmp_sp.opcodes(vec![0x39]).rex().w(),
    );

    // Convert flags to bool.
    // This encodes `b1` as an 8-bit low register with the value 0 or 1.
    e.enc_both(trueif, rec_seti_abcd.opcodes(vec![0x0f, 0x90]));
    e.enc_both(trueff, rec_setf_abcd.opcodes(vec![0x0f, 0x90]));

    // Conditional move (a.k.a integer select).
    e.enc_i32_i64(selectif, rec_cmov.opcodes(vec![0x0f, 0x40]));

    // Bit scan forwards and reverse
    e.enc_i32_i64(x86_bsf, rec_bsf_and_bsr.opcodes(vec![0x0f, 0xbc]));
    e.enc_i32_i64(x86_bsr, rec_bsf_and_bsr.opcodes(vec![0x0f, 0xbd]));

    // Convert bool to int.
    //
    // This assumes that b1 is represented as an 8-bit low register with the value 0
    // or 1.
    //
    // Encode movzbq as movzbl, because it's equivalent and shorter.
    e.enc32(
        bint.bind(I32).bind(B1),
        rec_urm_noflags_abcd.opcodes(vec![0x0f, 0xb6]),
    );

    e.enc64(
        bint.bind(I64).bind(B1),
        rec_urm_noflags.opcodes(vec![0x0f, 0xb6]).rex(),
    );
    e.enc64(
        bint.bind(I64).bind(B1),
        rec_urm_noflags_abcd.opcodes(vec![0x0f, 0xb6]),
    );
    e.enc64(
        bint.bind(I32).bind(B1),
        rec_urm_noflags.opcodes(vec![0x0f, 0xb6]).rex(),
    );
    e.enc64(
        bint.bind(I32).bind(B1),
        rec_urm_noflags_abcd.opcodes(vec![0x0f, 0xb6]),
    );

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
        rec_urm_noflags_abcd.opcodes(vec![0x0f, 0xbe]),
    );
    e.enc64(
        sextend.bind(I32).bind(I8),
        rec_urm_noflags.opcodes(vec![0x0f, 0xbe]).rex(),
    );
    e.enc64(
        sextend.bind(I32).bind(I8),
        rec_urm_noflags_abcd.opcodes(vec![0x0f, 0xbe]),
    );

    // movswl
    e.enc32(
        sextend.bind(I32).bind(I16),
        rec_urm_noflags.opcodes(vec![0x0f, 0xbf]),
    );
    e.enc64(
        sextend.bind(I32).bind(I16),
        rec_urm_noflags.opcodes(vec![0x0f, 0xbf]).rex(),
    );
    e.enc64(
        sextend.bind(I32).bind(I16),
        rec_urm_noflags.opcodes(vec![0x0f, 0xbf]),
    );

    // movsbq
    e.enc64(
        sextend.bind(I64).bind(I8),
        rec_urm_noflags.opcodes(vec![0x0f, 0xbe]).rex().w(),
    );

    // movswq
    e.enc64(
        sextend.bind(I64).bind(I16),
        rec_urm_noflags.opcodes(vec![0x0f, 0xbf]).rex().w(),
    );

    // movslq
    e.enc64(
        sextend.bind(I64).bind(I32),
        rec_urm_noflags.opcodes(vec![0x63]).rex().w(),
    );

    // movzbl
    e.enc32(
        uextend.bind(I32).bind(I8),
        rec_urm_noflags_abcd.opcodes(vec![0x0f, 0xb6]),
    );
    e.enc64(
        uextend.bind(I32).bind(I8),
        rec_urm_noflags.opcodes(vec![0x0f, 0xb6]).rex(),
    );
    e.enc64(
        uextend.bind(I32).bind(I8),
        rec_urm_noflags_abcd.opcodes(vec![0x0f, 0xb6]),
    );

    // movzwl
    e.enc32(
        uextend.bind(I32).bind(I16),
        rec_urm_noflags.opcodes(vec![0x0f, 0xb7]),
    );
    e.enc64(
        uextend.bind(I32).bind(I16),
        rec_urm_noflags.opcodes(vec![0x0f, 0xb7]).rex(),
    );
    e.enc64(
        uextend.bind(I32).bind(I16),
        rec_urm_noflags.opcodes(vec![0x0f, 0xb7]),
    );

    // movzbq, encoded as movzbl because it's equivalent and shorter.
    e.enc64(
        uextend.bind(I64).bind(I8),
        rec_urm_noflags.opcodes(vec![0x0f, 0xb6]).rex(),
    );
    e.enc64(
        uextend.bind(I64).bind(I8),
        rec_urm_noflags_abcd.opcodes(vec![0x0f, 0xb6]),
    );

    // movzwq, encoded as movzwl because it's equivalent and shorter
    e.enc64(
        uextend.bind(I64).bind(I16),
        rec_urm_noflags.opcodes(vec![0x0f, 0xb7]).rex(),
    );
    e.enc64(
        uextend.bind(I64).bind(I16),
        rec_urm_noflags.opcodes(vec![0x0f, 0xb7]),
    );

    // A 32-bit register copy clears the high 32 bits.
    e.enc64(
        uextend.bind(I64).bind(I32),
        rec_umr.opcodes(vec![0x89]).rex(),
    );
    e.enc64(uextend.bind(I64).bind(I32), rec_umr.opcodes(vec![0x89]));

    // Floating point

    // Floating-point constants equal to 0.0 can be encoded using either `xorps` or `xorpd`, for
    // 32-bit and 64-bit floats respectively.
    let f_unary_ieee32 = formats.get(formats.by_name("UnaryIeee32"));
    let is_zero_32_bit_float = InstructionPredicate::new_is_zero_32bit_float(f_unary_ieee32, "imm");
    e.enc32_instp(
        f32const,
        rec_f32imm_z.opcodes(vec![0x0f, 0x57]),
        is_zero_32_bit_float.clone(),
    );

    let f_unary_ieee64 = formats.get(formats.by_name("UnaryIeee64"));
    let is_zero_64_bit_float = InstructionPredicate::new_is_zero_64bit_float(f_unary_ieee64, "imm");
    e.enc32_instp(
        f64const,
        rec_f64imm_z.opcodes(vec![0x66, 0x0f, 0x57]),
        is_zero_64_bit_float.clone(),
    );

    e.enc_x86_64_instp(
        f32const,
        rec_f32imm_z.opcodes(vec![0x0f, 0x57]),
        is_zero_32_bit_float,
    );
    e.enc_x86_64_instp(
        f64const,
        rec_f64imm_z.opcodes(vec![0x66, 0x0f, 0x57]),
        is_zero_64_bit_float,
    );

    // movd
    e.enc_both(
        bitcast.bind(F32).bind(I32),
        rec_frurm.opcodes(vec![0x66, 0x0f, 0x6e]),
    );
    e.enc_both(
        bitcast.bind(I32).bind(F32),
        rec_rfumr.opcodes(vec![0x66, 0x0f, 0x7e]),
    );

    // movq
    e.enc64(
        bitcast.bind(F64).bind(I64),
        rec_frurm.opcodes(vec![0x66, 0x0f, 0x6e]).rex().w(),
    );
    e.enc64(
        bitcast.bind(I64).bind(F64),
        rec_rfumr.opcodes(vec![0x66, 0x0f, 0x7e]).rex().w(),
    );

    // movaps
    e.enc_both(copy.bind(F32), rec_furm.opcodes(vec![0x0f, 0x28]));
    e.enc_both(copy.bind(F64), rec_furm.opcodes(vec![0x0f, 0x28]));

    // TODO For x86-64, only define REX forms for now, since we can't describe the special regunit
    // immediate operands with the current constraint language.
    e.enc32(regmove.bind(F32), rec_frmov.opcodes(vec![0x0f, 0x28]));
    e.enc64(regmove.bind(F32), rec_frmov.opcodes(vec![0x0f, 0x28]).rex());

    // TODO For x86-64, only define REX forms for now, since we can't describe the special regunit
    // immediate operands with the current constraint language.
    e.enc32(regmove.bind(F64), rec_frmov.opcodes(vec![0x0f, 0x28]));
    e.enc64(regmove.bind(F64), rec_frmov.opcodes(vec![0x0f, 0x28]).rex());

    // cvtsi2ss
    e.enc_i32_i64(
        fcvt_from_sint.bind(F32),
        rec_frurm.opcodes(vec![0xf3, 0x0f, 0x2a]),
    );

    // cvtsi2sd
    e.enc_i32_i64(
        fcvt_from_sint.bind(F64),
        rec_frurm.opcodes(vec![0xf2, 0x0f, 0x2a]),
    );

    // cvtss2sd
    e.enc_both(
        fpromote.bind(F64).bind(F32),
        rec_furm.opcodes(vec![0xf3, 0x0f, 0x5a]),
    );

    // cvtsd2ss
    e.enc_both(
        fdemote.bind(F32).bind(F64),
        rec_furm.opcodes(vec![0xf2, 0x0f, 0x5a]),
    );

    // cvttss2si
    e.enc_both(
        x86_cvtt2si.bind(I32).bind(F32),
        rec_rfurm.opcodes(vec![0xf3, 0x0f, 0x2c]),
    );
    e.enc64(
        x86_cvtt2si.bind(I64).bind(F32),
        rec_rfurm.opcodes(vec![0xf3, 0x0f, 0x2c]).rex().w(),
    );

    // cvttsd2si
    e.enc_both(
        x86_cvtt2si.bind(I32).bind(F64),
        rec_rfurm.opcodes(vec![0xf2, 0x0f, 0x2c]),
    );
    e.enc64(
        x86_cvtt2si.bind(I64).bind(F64),
        rec_rfurm.opcodes(vec![0xf2, 0x0f, 0x2c]).rex().w(),
    );

    // Exact square roots.
    e.enc_both(sqrt.bind(F32), rec_furm.opcodes(vec![0xf3, 0x0f, 0x51]));
    e.enc_both(sqrt.bind(F64), rec_furm.opcodes(vec![0xf2, 0x0f, 0x51]));

    // Rounding. The recipe looks at the opcode to pick an immediate.
    for inst in &[nearest, floor, ceil, trunc] {
        e.enc_both_isap(
            inst.bind(F32),
            rec_furmi_rnd.opcodes(vec![0x66, 0x0f, 0x3a, 0x0a]),
            use_sse41,
        );
        e.enc_both_isap(
            inst.bind(F64),
            rec_furmi_rnd.opcodes(vec![0x66, 0x0f, 0x3a, 0x0b]),
            use_sse41,
        );
    }

    // Binary arithmetic ops.
    for &(inst, opc) in &[
        (fadd, 0x58),
        (fsub, 0x5c),
        (fmul, 0x59),
        (fdiv, 0x5e),
        (x86_fmin, 0x5d),
        (x86_fmax, 0x5f),
    ] {
        e.enc_both(inst.bind(F32), rec_fa.opcodes(vec![0xf3, 0x0f, opc]));
        e.enc_both(inst.bind(F64), rec_fa.opcodes(vec![0xf2, 0x0f, opc]));
    }

    // Binary bitwise ops.
    for &(inst, opc) in &[(band, 0x54), (bor, 0x56), (bxor, 0x57)] {
        e.enc_both(inst.bind(F32), rec_fa.opcodes(vec![0x0f, opc]));
        e.enc_both(inst.bind(F64), rec_fa.opcodes(vec![0x0f, opc]));
    }

    // The `andnps(x,y)` instruction computes `~x&y`, while band_not(x,y)` is `x&~y.
    e.enc_both(band_not.bind(F32), rec_fax.opcodes(vec![0x0f, 0x55]));
    e.enc_both(band_not.bind(F64), rec_fax.opcodes(vec![0x0f, 0x55]));

    // Comparisons.
    //
    // This only covers the condition codes in `supported_floatccs`, the rest are
    // handled by legalization patterns.
    e.enc_both(fcmp.bind(F32), rec_fcscc.opcodes(vec![0x0f, 0x2e]));
    e.enc_both(fcmp.bind(F64), rec_fcscc.opcodes(vec![0x66, 0x0f, 0x2e]));
    e.enc_both(ffcmp.bind(F32), rec_fcmp.opcodes(vec![0x0f, 0x2e]));
    e.enc_both(ffcmp.bind(F64), rec_fcmp.opcodes(vec![0x66, 0x0f, 0x2e]));

    // SIMD vector size: eventually multiple vector sizes may be supported but for now only SSE-sized vectors are available
    let sse_vector_size: u64 = 128;

    // SIMD splat: before x86 can use vector data, it must be moved to XMM registers; see
    // legalize.rs for how this is done; once there, x86_pshuf* (below) is used for broadcasting the
    // value across the register

    // PSHUFB, 8-bit shuffle using two XMM registers
    for ty in ValueType::all_lane_types().filter(|t| t.lane_bits() == 8) {
        let instruction = x86_pshufb.bind_vector_from_lane(ty, sse_vector_size);
        let template = rec_fa.nonrex().opcodes(vec![0x66, 0x0f, 0x38, 00]);
        e.enc32_isap(instruction.clone(), template.clone(), use_ssse3_simd);
        e.enc64_isap(instruction, template, use_ssse3_simd);
    }

    // PSHUFD, 32-bit shuffle using one XMM register and a u8 immediate
    for ty in ValueType::all_lane_types().filter(|t| t.lane_bits() == 32) {
        let instruction = x86_pshufd.bind_vector_from_lane(ty, sse_vector_size);
        let template = rec_r_ib_unsigned_fpr
            .nonrex()
            .opcodes(vec![0x66, 0x0f, 0x70]);
        e.enc32(instruction.clone(), template.clone());
        e.enc64(instruction, template);
    }

    // SIMD scalar_to_vector; this uses MOV to copy the scalar value to an XMM register; according
    // to the Intel manual: "When the destination operand is an XMM register, the source operand is
    // written to the low doubleword of the register and the regiser is zero-extended to 128 bits."
    for ty in ValueType::all_lane_types().filter(|t| t.lane_bits() >= 8) {
        let instruction = scalar_to_vector
            .bind_vector_from_lane(ty, sse_vector_size)
            .bind(ty);
        let template = rec_frurm.opcodes(vec![0x66, 0x0f, 0x6e]); // MOVD/MOVQ
        if ty.lane_bits() < 64 {
            // no 32-bit encodings for 64-bit widths
            e.enc32(instruction.clone(), template.clone());
        }
        e.enc_x86_64(instruction, template);
    }

    // SIMD insertlane
    let mut insertlane_mapping: HashMap<u64, (Vec<u8>, Option<SettingPredicateNumber>)> =
        HashMap::new();
    insertlane_mapping.insert(8, (vec![0x66, 0x0f, 0x3a, 0x20], Some(use_sse41_simd))); // PINSRB
    insertlane_mapping.insert(16, (vec![0x66, 0x0f, 0xc4], None)); // PINSRW from SSE2
    insertlane_mapping.insert(32, (vec![0x66, 0x0f, 0x3a, 0x22], Some(use_sse41_simd))); // PINSRD
    insertlane_mapping.insert(64, (vec![0x66, 0x0f, 0x3a, 0x22], Some(use_sse41_simd))); // PINSRQ, only x86_64

    for ty in ValueType::all_lane_types() {
        if let Some((opcode, isap)) = insertlane_mapping.get(&ty.lane_bits()) {
            let instruction = insertlane.bind_vector_from_lane(ty, sse_vector_size);
            let template = rec_r_ib_unsigned_r.opcodes(opcode.clone());
            if ty.lane_bits() < 64 {
                e.enc_32_64_maybe_isap(instruction, template.nonrex(), isap.clone());
            } else {
                // turns out the 64-bit widths have REX/W encodings and only are available on x86_64
                e.enc64_maybe_isap(instruction, template.rex().w(), isap.clone());
            }
        }
    }

    // SIMD extractlane
    let mut extractlane_mapping: HashMap<u64, (Vec<u8>, Option<SettingPredicateNumber>)> =
        HashMap::new();
    extractlane_mapping.insert(8, (vec![0x66, 0x0f, 0x3a, 0x14], Some(use_sse41_simd))); // PEXTRB
    extractlane_mapping.insert(16, (vec![0x66, 0x0f, 0xc5], None)); // PEXTRW from zSSE2, SSE4.1 has a PEXTRW that can move to reg/m16 but the opcode is four bytes
    extractlane_mapping.insert(32, (vec![0x66, 0x0f, 0x3a, 0x16], Some(use_sse41_simd))); // PEXTRD
    extractlane_mapping.insert(64, (vec![0x66, 0x0f, 0x3a, 0x16], Some(use_sse41_simd))); // PEXTRQ, only x86_64

    for ty in ValueType::all_lane_types() {
        if let Some((opcode, isap)) = extractlane_mapping.get(&ty.lane_bits()) {
            let instruction = extractlane.bind_vector_from_lane(ty, sse_vector_size);
            let template = rec_r_ib_unsigned_gpr.opcodes(opcode.clone());
            if ty.lane_bits() < 64 {
                e.enc_32_64_maybe_isap(instruction, template.nonrex(), isap.clone());
            } else {
                // turns out the 64-bit widths have REX/W encodings and only are available on x86_64
                e.enc64_maybe_isap(instruction, template.rex().w(), isap.clone());
            }
        }
    }

    // SIMD bitcast f64 to all 8-bit-lane vectors (for legalizing splat.x8x16); assumes that f64 is stored in an XMM register
    for ty in ValueType::all_lane_types().filter(|t| t.lane_bits() == 8) {
        let instruction = bitcast.bind_vector_from_lane(ty, sse_vector_size).bind(F64);
        e.enc32_rec(instruction.clone(), rec_null_fpr, 0);
        e.enc64_rec(instruction, rec_null_fpr, 0);
    }

    // SIMD bitcast all 128-bit vectors to each other (for legalizing splat.x16x8)
    for from_type in ValueType::all_lane_types().filter(|t| t.lane_bits() >= 8) {
        for to_type in ValueType::all_lane_types().filter(|t| t.lane_bits() >= 8 && *t != from_type)
        {
            let instruction = raw_bitcast
                .bind_vector_from_lane(to_type, sse_vector_size)
                .bind_vector_from_lane(from_type, sse_vector_size);
            e.enc32_rec(instruction.clone(), rec_null_fpr, 0);
            e.enc64_rec(instruction, rec_null_fpr, 0);
        }
    }

    // SIMD vconst using MOVUPS
    // TODO it would be ideal if eventually this became the more efficient MOVAPS but we would have
    // to guarantee that the constants are aligned when emitted and there is currently no mechanism
    // for that; alternately, constants could be loaded into XMM registers using a sequence like:
    // MOVQ + MOVHPD + MOVQ + MOVLPD (this allows the constants to be immediates instead of stored
    // in memory) but some performance measurements are needed.
    for ty in ValueType::all_lane_types().filter(|t| t.lane_bits() >= 8) {
        let instruction = vconst.bind_vector_from_lane(ty, sse_vector_size);
        let template = rec_vconst.nonrex().opcodes(vec![0x0f, 0x10]);
        e.enc_32_64_maybe_isap(instruction, template, None); // from SSE
    }

    // Reference type instructions

    // Null references implemented as iconst 0.
    e.enc32(null.bind_ref(R32), rec_pu_id_ref.opcodes(vec![0xb8]));

    e.enc64(null.bind_ref(R64), rec_pu_id_ref.rex().opcodes(vec![0xb8]));
    e.enc64(null.bind_ref(R64), rec_pu_id_ref.opcodes(vec![0xb8]));

    // is_null, implemented by testing whether the value is 0.
    e.enc_r32_r64(is_null, rec_is_zero.opcodes(vec![0x85]));

    // safepoint instruction calls sink, no actual encoding.
    e.enc32_rec(safepoint, rec_safepoint, 0);
    e.enc64_rec(safepoint, rec_safepoint, 0);

    e
}

use crate::cdsl::ast::{Apply, Expr, Literal, VarPool};
use crate::cdsl::encodings::{Encoding, EncodingBuilder};
use crate::cdsl::instructions::{
    Bindable, BoundInstruction, InstSpec, InstructionPredicateNode, InstructionPredicateRegistry,
};
use crate::cdsl::recipes::{EncodingRecipeNumber, Recipes};
use crate::cdsl::settings::SettingGroup;

use crate::shared::types::Bool::B1;
use crate::shared::types::Float::{F32, F64};
use crate::shared::types::Int::{I16, I32, I64, I8};
use crate::shared::types::Reference::{R32, R64};
use crate::shared::Definitions as SharedDefinitions;

use super::recipes::RecipeGroup;

pub(crate) struct PerCpuModeEncodings<'defs> {
    pub inst_pred_reg: InstructionPredicateRegistry,
    pub enc32: Vec<Encoding>,
    pub enc64: Vec<Encoding>,
    recipes: &'defs Recipes,
}

impl<'defs> PerCpuModeEncodings<'defs> {
    fn new(recipes: &'defs Recipes) -> Self {
        Self {
            inst_pred_reg: InstructionPredicateRegistry::new(),
            enc32: Vec::new(),
            enc64: Vec::new(),
            recipes,
        }
    }
    fn enc(
        &self,
        inst: impl Into<InstSpec>,
        recipe: EncodingRecipeNumber,
        bits: u16,
    ) -> EncodingBuilder {
        EncodingBuilder::new(inst.into(), recipe, bits)
    }
    fn add32(&mut self, encoding: EncodingBuilder) {
        self.enc32
            .push(encoding.build(self.recipes, &mut self.inst_pred_reg));
    }
    fn add64(&mut self, encoding: EncodingBuilder) {
        self.enc64
            .push(encoding.build(self.recipes, &mut self.inst_pred_reg));
    }
}

// The low 7 bits of a RISC-V instruction is the base opcode. All 32-bit instructions have 11 as
// the two low bits, with bits 6:2 determining the base opcode.
//
// Encbits for the 32-bit recipes are opcode[6:2] | (funct3 << 5) | ...
// The functions below encode the encbits.

fn load_bits(funct3: u16) -> u16 {
    assert!(funct3 <= 0b111);
    funct3 << 5
}

fn store_bits(funct3: u16) -> u16 {
    assert!(funct3 <= 0b111);
    0b01000 | (funct3 << 5)
}

fn branch_bits(funct3: u16) -> u16 {
    assert!(funct3 <= 0b111);
    0b11000 | (funct3 << 5)
}

fn jalr_bits() -> u16 {
    // This was previously accepting an argument funct3 of 3 bits and used the following formula:
    //0b11001 | (funct3 << 5)
    0b11001
}

fn jal_bits() -> u16 {
    0b11011
}

fn opimm_bits(funct3: u16, funct7: u16) -> u16 {
    assert!(funct3 <= 0b111);
    0b00100 | (funct3 << 5) | (funct7 << 8)
}

fn opimm32_bits(funct3: u16, funct7: u16) -> u16 {
    assert!(funct3 <= 0b111);
    0b00110 | (funct3 << 5) | (funct7 << 8)
}

fn op_bits(funct3: u16, funct7: u16) -> u16 {
    assert!(funct3 <= 0b111);
    assert!(funct7 <= 0b111_1111);
    0b01100 | (funct3 << 5) | (funct7 << 8)
}

fn op32_bits(funct3: u16, funct7: u16) -> u16 {
    assert!(funct3 <= 0b111);
    assert!(funct7 <= 0b111_1111);
    0b01110 | (funct3 << 5) | (funct7 << 8)
}

fn lui_bits() -> u16 {
    0b01101
}

pub(crate) fn define<'defs>(
    shared_defs: &'defs SharedDefinitions,
    isa_settings: &SettingGroup,
    recipes: &'defs RecipeGroup,
) -> PerCpuModeEncodings<'defs> {
    // Instructions shorthands.
    let shared = &shared_defs.instructions;

    let band = shared.by_name("band");
    let band_imm = shared.by_name("band_imm");
    let bor = shared.by_name("bor");
    let bor_imm = shared.by_name("bor_imm");
    let br_icmp = shared.by_name("br_icmp");
    let brz = shared.by_name("brz");
    let brnz = shared.by_name("brnz");
    let bxor = shared.by_name("bxor");
    let bxor_imm = shared.by_name("bxor_imm");
    let call = shared.by_name("call");
    let call_indirect = shared.by_name("call_indirect");
    let copy = shared.by_name("copy");
    let copy_nop = shared.by_name("copy_nop");
    let copy_to_ssa = shared.by_name("copy_to_ssa");
    let fill = shared.by_name("fill");
    let fill_nop = shared.by_name("fill_nop");
    let iadd = shared.by_name("iadd");
    let iadd_imm = shared.by_name("iadd_imm");
    let iconst = shared.by_name("iconst");
    let icmp = shared.by_name("icmp");
    let icmp_imm = shared.by_name("icmp_imm");
    let imul = shared.by_name("imul");
    let ishl = shared.by_name("ishl");
    let ishl_imm = shared.by_name("ishl_imm");
    let isub = shared.by_name("isub");
    let jump = shared.by_name("jump");
    let regmove = shared.by_name("regmove");
    let spill = shared.by_name("spill");
    let sshr = shared.by_name("sshr");
    let sshr_imm = shared.by_name("sshr_imm");
    let ushr = shared.by_name("ushr");
    let ushr_imm = shared.by_name("ushr_imm");
    let return_ = shared.by_name("return");

    // Recipes shorthands, prefixed with r_.
    let r_copytossa = recipes.by_name("copytossa");
    let r_fillnull = recipes.by_name("fillnull");
    let r_icall = recipes.by_name("Icall");
    let r_icopy = recipes.by_name("Icopy");
    let r_ii = recipes.by_name("Ii");
    let r_iicmp = recipes.by_name("Iicmp");
    let r_iret = recipes.by_name("Iret");
    let r_irmov = recipes.by_name("Irmov");
    let r_iz = recipes.by_name("Iz");
    let r_gp_sp = recipes.by_name("GPsp");
    let r_gp_fi = recipes.by_name("GPfi");
    let r_r = recipes.by_name("R");
    let r_ricmp = recipes.by_name("Ricmp");
    let r_rshamt = recipes.by_name("Rshamt");
    let r_sb = recipes.by_name("SB");
    let r_sb_zero = recipes.by_name("SBzero");
    let r_stacknull = recipes.by_name("stacknull");
    let r_u = recipes.by_name("U");
    let r_uj = recipes.by_name("UJ");
    let r_uj_call = recipes.by_name("UJcall");

    // Predicates shorthands.
    let use_m = isa_settings.predicate_by_name("use_m");

    // Definitions.
    let mut e = PerCpuModeEncodings::new(&recipes.recipes);

    // Basic arithmetic binary instructions are encoded in an R-type instruction.
    for &(inst, inst_imm, f3, f7) in &[
        (iadd, Some(iadd_imm), 0b000, 0b000_0000),
        (isub, None, 0b000, 0b010_0000),
        (bxor, Some(bxor_imm), 0b100, 0b000_0000),
        (bor, Some(bor_imm), 0b110, 0b000_0000),
        (band, Some(band_imm), 0b111, 0b000_0000),
    ] {
        e.add32(e.enc(inst.bind(I32), r_r, op_bits(f3, f7)));
        e.add64(e.enc(inst.bind(I64), r_r, op_bits(f3, f7)));

        // Immediate versions for add/xor/or/and.
        if let Some(inst_imm) = inst_imm {
            e.add32(e.enc(inst_imm.bind(I32), r_ii, opimm_bits(f3, 0)));
            e.add64(e.enc(inst_imm.bind(I64), r_ii, opimm_bits(f3, 0)));
        }
    }

    // 32-bit ops in RV64.
    e.add64(e.enc(iadd.bind(I32), r_r, op32_bits(0b000, 0b000_0000)));
    e.add64(e.enc(isub.bind(I32), r_r, op32_bits(0b000, 0b010_0000)));
    // There are no andiw/oriw/xoriw variations.
    e.add64(e.enc(iadd_imm.bind(I32), r_ii, opimm32_bits(0b000, 0)));

    // Use iadd_imm with %x0 to materialize constants.
    e.add32(e.enc(iconst.bind(I32), r_iz, opimm_bits(0b0, 0)));
    e.add64(e.enc(iconst.bind(I32), r_iz, opimm_bits(0b0, 0)));
    e.add64(e.enc(iconst.bind(I64), r_iz, opimm_bits(0b0, 0)));

    // Dynamic shifts have the same masking semantics as the clif base instructions.
    for &(inst, inst_imm, f3, f7) in &[
        (ishl, ishl_imm, 0b1, 0b0),
        (ushr, ushr_imm, 0b101, 0b0),
        (sshr, sshr_imm, 0b101, 0b10_0000),
    ] {
        e.add32(e.enc(inst.bind(I32).bind(I32), r_r, op_bits(f3, f7)));
        e.add64(e.enc(inst.bind(I64).bind(I64), r_r, op_bits(f3, f7)));
        e.add64(e.enc(inst.bind(I32).bind(I32), r_r, op32_bits(f3, f7)));
        // Allow i32 shift amounts in 64-bit shifts.
        e.add64(e.enc(inst.bind(I64).bind(I32), r_r, op_bits(f3, f7)));
        e.add64(e.enc(inst.bind(I32).bind(I64), r_r, op32_bits(f3, f7)));

        // Immediate shifts.
        e.add32(e.enc(inst_imm.bind(I32), r_rshamt, opimm_bits(f3, f7)));
        e.add64(e.enc(inst_imm.bind(I64), r_rshamt, opimm_bits(f3, f7)));
        e.add64(e.enc(inst_imm.bind(I32), r_rshamt, opimm32_bits(f3, f7)));
    }

    // Signed and unsigned integer 'less than'. There are no 'w' variants for comparing 32-bit
    // numbers in RV64.
    {
        let mut var_pool = VarPool::new();

        // Helper that creates an instruction predicate for an instruction in the icmp family.
        let mut icmp_instp = |bound_inst: &BoundInstruction,
                              intcc_field: &'static str|
         -> InstructionPredicateNode {
            let x = var_pool.create("x");
            let y = var_pool.create("y");
            let cc = Literal::enumerator_for(&shared_defs.imm.intcc, intcc_field);
            Apply::new(
                bound_inst.clone().into(),
                vec![Expr::Literal(cc), Expr::Var(x), Expr::Var(y)],
            )
            .inst_predicate(&var_pool)
            .unwrap()
        };

        let icmp_i32 = icmp.bind(I32);
        let icmp_i64 = icmp.bind(I64);
        e.add32(
            e.enc(icmp_i32.clone(), r_ricmp, op_bits(0b010, 0b000_0000))
                .inst_predicate(icmp_instp(&icmp_i32, "slt")),
        );
        e.add64(
            e.enc(icmp_i64.clone(), r_ricmp, op_bits(0b010, 0b000_0000))
                .inst_predicate(icmp_instp(&icmp_i64, "slt")),
        );

        e.add32(
            e.enc(icmp_i32.clone(), r_ricmp, op_bits(0b011, 0b000_0000))
                .inst_predicate(icmp_instp(&icmp_i32, "ult")),
        );
        e.add64(
            e.enc(icmp_i64.clone(), r_ricmp, op_bits(0b011, 0b000_0000))
                .inst_predicate(icmp_instp(&icmp_i64, "ult")),
        );

        // Immediate variants.
        let icmp_i32 = icmp_imm.bind(I32);
        let icmp_i64 = icmp_imm.bind(I64);
        e.add32(
            e.enc(icmp_i32.clone(), r_iicmp, opimm_bits(0b010, 0))
                .inst_predicate(icmp_instp(&icmp_i32, "slt")),
        );
        e.add64(
            e.enc(icmp_i64.clone(), r_iicmp, opimm_bits(0b010, 0))
                .inst_predicate(icmp_instp(&icmp_i64, "slt")),
        );

        e.add32(
            e.enc(icmp_i32.clone(), r_iicmp, opimm_bits(0b011, 0))
                .inst_predicate(icmp_instp(&icmp_i32, "ult")),
        );
        e.add64(
            e.enc(icmp_i64.clone(), r_iicmp, opimm_bits(0b011, 0))
                .inst_predicate(icmp_instp(&icmp_i64, "ult")),
        );
    }

    // Integer constants with the low 12 bits clear are materialized by lui.
    e.add32(e.enc(iconst.bind(I32), r_u, lui_bits()));
    e.add64(e.enc(iconst.bind(I32), r_u, lui_bits()));
    e.add64(e.enc(iconst.bind(I64), r_u, lui_bits()));

    // "M" Standard Extension for Integer Multiplication and Division.
    // Gated by the `use_m` flag.
    e.add32(
        e.enc(imul.bind(I32), r_r, op_bits(0b000, 0b0000_0001))
            .isa_predicate(use_m),
    );
    e.add64(
        e.enc(imul.bind(I64), r_r, op_bits(0b000, 0b0000_0001))
            .isa_predicate(use_m),
    );
    e.add64(
        e.enc(imul.bind(I32), r_r, op32_bits(0b000, 0b0000_0001))
            .isa_predicate(use_m),
    );

    // Control flow.

    // Unconditional branches.
    e.add32(e.enc(jump, r_uj, jal_bits()));
    e.add64(e.enc(jump, r_uj, jal_bits()));
    e.add32(e.enc(call, r_uj_call, jal_bits()));
    e.add64(e.enc(call, r_uj_call, jal_bits()));

    // Conditional branches.
    {
        let mut var_pool = VarPool::new();

        // Helper that creates an instruction predicate for an instruction in the icmp family.
        let mut br_icmp_instp = |bound_inst: &BoundInstruction,
                                 intcc_field: &'static str|
         -> InstructionPredicateNode {
            let x = var_pool.create("x");
            let y = var_pool.create("y");
            let dest = var_pool.create("dest");
            let args = var_pool.create("args");
            let cc = Literal::enumerator_for(&shared_defs.imm.intcc, intcc_field);
            Apply::new(
                bound_inst.clone().into(),
                vec![
                    Expr::Literal(cc),
                    Expr::Var(x),
                    Expr::Var(y),
                    Expr::Var(dest),
                    Expr::Var(args),
                ],
            )
            .inst_predicate(&var_pool)
            .unwrap()
        };

        let br_icmp_i32 = br_icmp.bind(I32);
        let br_icmp_i64 = br_icmp.bind(I64);
        for &(cond, f3) in &[
            ("eq", 0b000),
            ("ne", 0b001),
            ("slt", 0b100),
            ("sge", 0b101),
            ("ult", 0b110),
            ("uge", 0b111),
        ] {
            e.add32(
                e.enc(br_icmp_i32.clone(), r_sb, branch_bits(f3))
                    .inst_predicate(br_icmp_instp(&br_icmp_i32, cond)),
            );
            e.add64(
                e.enc(br_icmp_i64.clone(), r_sb, branch_bits(f3))
                    .inst_predicate(br_icmp_instp(&br_icmp_i64, cond)),
            );
        }
    }

    for &(inst, f3) in &[(brz, 0b000), (brnz, 0b001)] {
        e.add32(e.enc(inst.bind(I32), r_sb_zero, branch_bits(f3)));
        e.add64(e.enc(inst.bind(I64), r_sb_zero, branch_bits(f3)));
        e.add32(e.enc(inst.bind(B1), r_sb_zero, branch_bits(f3)));
        e.add64(e.enc(inst.bind(B1), r_sb_zero, branch_bits(f3)));
    }

    // Returns are a special case of jalr_bits using %x1 to hold the return address.
    // The return address is provided by a special-purpose `link` return value that
    // is added by legalize_signature().
    e.add32(e.enc(return_, r_iret, jalr_bits()));
    e.add64(e.enc(return_, r_iret, jalr_bits()));
    e.add32(e.enc(call_indirect.bind(I32), r_icall, jalr_bits()));
    e.add64(e.enc(call_indirect.bind(I64), r_icall, jalr_bits()));

    // Spill and fill.
    e.add32(e.enc(spill.bind(I32), r_gp_sp, store_bits(0b010)));
    e.add64(e.enc(spill.bind(I32), r_gp_sp, store_bits(0b010)));
    e.add64(e.enc(spill.bind(I64), r_gp_sp, store_bits(0b011)));
    e.add32(e.enc(fill.bind(I32), r_gp_fi, load_bits(0b010)));
    e.add64(e.enc(fill.bind(I32), r_gp_fi, load_bits(0b010)));
    e.add64(e.enc(fill.bind(I64), r_gp_fi, load_bits(0b011)));

    // No-op fills, created by late-stage redundant-fill removal.
    for &ty in &[I64, I32] {
        e.add64(e.enc(fill_nop.bind(ty), r_fillnull, 0));
        e.add32(e.enc(fill_nop.bind(ty), r_fillnull, 0));
    }
    e.add64(e.enc(fill_nop.bind(B1), r_fillnull, 0));
    e.add32(e.enc(fill_nop.bind(B1), r_fillnull, 0));

    // Register copies.
    e.add32(e.enc(copy.bind(I32), r_icopy, opimm_bits(0b000, 0)));
    e.add64(e.enc(copy.bind(I64), r_icopy, opimm_bits(0b000, 0)));
    e.add64(e.enc(copy.bind(I32), r_icopy, opimm32_bits(0b000, 0)));

    e.add32(e.enc(regmove.bind(I32), r_irmov, opimm_bits(0b000, 0)));
    e.add64(e.enc(regmove.bind(I64), r_irmov, opimm_bits(0b000, 0)));
    e.add64(e.enc(regmove.bind(I32), r_irmov, opimm32_bits(0b000, 0)));

    e.add32(e.enc(copy.bind(B1), r_icopy, opimm_bits(0b000, 0)));
    e.add64(e.enc(copy.bind(B1), r_icopy, opimm_bits(0b000, 0)));
    e.add32(e.enc(regmove.bind(B1), r_irmov, opimm_bits(0b000, 0)));
    e.add64(e.enc(regmove.bind(B1), r_irmov, opimm_bits(0b000, 0)));

    // Stack-slot-to-the-same-stack-slot copy, which is guaranteed to turn
    // into a no-op.
    // The same encoding is generated for both the 64- and 32-bit architectures.
    for &ty in &[I64, I32, I16, I8] {
        e.add32(e.enc(copy_nop.bind(ty), r_stacknull, 0));
        e.add64(e.enc(copy_nop.bind(ty), r_stacknull, 0));
    }
    for &ty in &[F64, F32] {
        e.add32(e.enc(copy_nop.bind(ty), r_stacknull, 0));
        e.add64(e.enc(copy_nop.bind(ty), r_stacknull, 0));
    }

    // Copy-to-SSA
    e.add32(e.enc(copy_to_ssa.bind(I32), r_copytossa, opimm_bits(0b000, 0)));
    e.add64(e.enc(copy_to_ssa.bind(I64), r_copytossa, opimm_bits(0b000, 0)));
    e.add64(e.enc(copy_to_ssa.bind(I32), r_copytossa, opimm32_bits(0b000, 0)));
    e.add32(e.enc(copy_to_ssa.bind(B1), r_copytossa, opimm_bits(0b000, 0)));
    e.add64(e.enc(copy_to_ssa.bind(B1), r_copytossa, opimm_bits(0b000, 0)));
    e.add32(e.enc(copy_to_ssa.bind(R32), r_copytossa, opimm_bits(0b000, 0)));
    e.add64(e.enc(copy_to_ssa.bind(R64), r_copytossa, opimm_bits(0b000, 0)));

    e
}

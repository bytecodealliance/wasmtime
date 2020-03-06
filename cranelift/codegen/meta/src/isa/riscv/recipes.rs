use std::collections::HashMap;

use crate::cdsl::instructions::InstructionPredicate;
use crate::cdsl::recipes::{EncodingRecipeBuilder, EncodingRecipeNumber, Recipes, Stack};
use crate::cdsl::regs::IsaRegs;
use crate::shared::Definitions as SharedDefinitions;

/// An helper to create recipes and use them when defining the RISCV encodings.
pub(crate) struct RecipeGroup {
    /// The actualy list of recipes explicitly created in this file.
    pub recipes: Recipes,

    /// Provides fast lookup from a name to an encoding recipe.
    name_to_recipe: HashMap<String, EncodingRecipeNumber>,
}

impl RecipeGroup {
    fn new() -> Self {
        Self {
            recipes: Recipes::new(),
            name_to_recipe: HashMap::new(),
        }
    }

    fn push(&mut self, builder: EncodingRecipeBuilder) {
        assert!(
            self.name_to_recipe.get(&builder.name).is_none(),
            format!("riscv recipe '{}' created twice", builder.name)
        );
        let name = builder.name.clone();
        let number = self.recipes.push(builder.build());
        self.name_to_recipe.insert(name, number);
    }

    pub fn by_name(&self, name: &str) -> EncodingRecipeNumber {
        *self
            .name_to_recipe
            .get(name)
            .unwrap_or_else(|| panic!("unknown riscv recipe name {}", name))
    }

    pub fn collect(self) -> Recipes {
        self.recipes
    }
}

pub(crate) fn define(shared_defs: &SharedDefinitions, regs: &IsaRegs) -> RecipeGroup {
    let formats = &shared_defs.formats;

    // Register classes shorthands.
    let gpr = regs.class_by_name("GPR");

    // Definitions.
    let mut recipes = RecipeGroup::new();

    // R-type 32-bit instructions: These are mostly binary arithmetic instructions.
    // The encbits are `opcode[6:2] | (funct3 << 5) | (funct7 << 8)
    recipes.push(
        EncodingRecipeBuilder::new("R", &formats.binary, 4)
            .operands_in(vec![gpr, gpr])
            .operands_out(vec![gpr])
            .emit("put_r(bits, in_reg0, in_reg1, out_reg0, sink);"),
    );

    // R-type with an immediate shift amount instead of rs2.
    recipes.push(
        EncodingRecipeBuilder::new("Rshamt", &formats.binary_imm, 4)
            .operands_in(vec![gpr])
            .operands_out(vec![gpr])
            .emit("put_rshamt(bits, in_reg0, imm.into(), out_reg0, sink);"),
    );

    // R-type encoding of an integer comparison.
    recipes.push(
        EncodingRecipeBuilder::new("Ricmp", &formats.int_compare, 4)
            .operands_in(vec![gpr, gpr])
            .operands_out(vec![gpr])
            .emit("put_r(bits, in_reg0, in_reg1, out_reg0, sink);"),
    );

    recipes.push(
        EncodingRecipeBuilder::new("Ii", &formats.binary_imm, 4)
            .operands_in(vec![gpr])
            .operands_out(vec![gpr])
            .inst_predicate(InstructionPredicate::new_is_signed_int(
                &*formats.binary_imm,
                "imm",
                12,
                0,
            ))
            .emit("put_i(bits, in_reg0, imm.into(), out_reg0, sink);"),
    );

    // I-type instruction with a hardcoded %x0 rs1.
    recipes.push(
        EncodingRecipeBuilder::new("Iz", &formats.unary_imm, 4)
            .operands_out(vec![gpr])
            .inst_predicate(InstructionPredicate::new_is_signed_int(
                &formats.unary_imm,
                "imm",
                12,
                0,
            ))
            .emit("put_i(bits, 0, imm.into(), out_reg0, sink);"),
    );

    // I-type encoding of an integer comparison.
    recipes.push(
        EncodingRecipeBuilder::new("Iicmp", &formats.int_compare_imm, 4)
            .operands_in(vec![gpr])
            .operands_out(vec![gpr])
            .inst_predicate(InstructionPredicate::new_is_signed_int(
                &formats.int_compare_imm,
                "imm",
                12,
                0,
            ))
            .emit("put_i(bits, in_reg0, imm.into(), out_reg0, sink);"),
    );

    // I-type encoding for `jalr` as a return instruction. We won't use the immediate offset.  The
    // variable return values are not encoded.
    recipes.push(
        EncodingRecipeBuilder::new("Iret", &formats.multiary, 4).emit(
            r#"
                    // Return instructions are always a jalr to %x1.
                    // The return address is provided as a special-purpose link argument.
                    put_i(
                        bits,
                        1, // rs1 = %x1
                        0, // no offset.
                        0, // rd = %x0: no address written.
                        sink,
                    );
                "#,
        ),
    );

    // I-type encoding for `jalr` as a call_indirect.
    recipes.push(
        EncodingRecipeBuilder::new("Icall", &formats.call_indirect, 4)
            .operands_in(vec![gpr])
            .emit(
                r#"
                    // call_indirect instructions are jalr with rd=%x1.
                    put_i(
                        bits,
                        in_reg0,
                        0, // no offset.
                        1, // rd = %x1: link register.
                        sink,
                    );
                "#,
            ),
    );

    // Copy of a GPR is implemented as addi x, 0.
    recipes.push(
        EncodingRecipeBuilder::new("Icopy", &formats.unary, 4)
            .operands_in(vec![gpr])
            .operands_out(vec![gpr])
            .emit("put_i(bits, in_reg0, 0, out_reg0, sink);"),
    );

    // Same for a GPR regmove.
    recipes.push(
        EncodingRecipeBuilder::new("Irmov", &formats.reg_move, 4)
            .operands_in(vec![gpr])
            .emit("put_i(bits, src, 0, dst, sink);"),
    );

    // Same for copy-to-SSA -- GPR regmove.
    recipes.push(
        EncodingRecipeBuilder::new("copytossa", &formats.copy_to_ssa, 4)
            // No operands_in to mention, because a source register is specified directly.
            .operands_out(vec![gpr])
            .emit("put_i(bits, src, 0, out_reg0, sink);"),
    );

    // U-type instructions have a 20-bit immediate that targets bits 12-31.
    recipes.push(
        EncodingRecipeBuilder::new("U", &formats.unary_imm, 4)
            .operands_out(vec![gpr])
            .inst_predicate(InstructionPredicate::new_is_signed_int(
                &formats.unary_imm,
                "imm",
                32,
                12,
            ))
            .emit("put_u(bits, imm.into(), out_reg0, sink);"),
    );

    // UJ-type unconditional branch instructions.
    recipes.push(
        EncodingRecipeBuilder::new("UJ", &formats.jump, 4)
            .branch_range((0, 21))
            .emit(
                r#"
                    let dest = i64::from(func.offsets[destination]);
                    let disp = dest - i64::from(sink.offset());
                    put_uj(bits, disp, 0, sink);
                "#,
            ),
    );

    recipes.push(EncodingRecipeBuilder::new("UJcall", &formats.call, 4).emit(
        r#"
                    sink.reloc_external(Reloc::RiscvCall,
                                        &func.dfg.ext_funcs[func_ref].name,
                                        0);
                    // rd=%x1 is the standard link register.
                    put_uj(bits, 0, 1, sink);
                "#,
    ));

    // SB-type branch instructions.
    recipes.push(
        EncodingRecipeBuilder::new("SB", &formats.branch_icmp, 4)
            .operands_in(vec![gpr, gpr])
            .branch_range((0, 13))
            .emit(
                r#"
                    let dest = i64::from(func.offsets[destination]);
                    let disp = dest - i64::from(sink.offset());
                    put_sb(bits, disp, in_reg0, in_reg1, sink);
                "#,
            ),
    );

    // SB-type branch instruction with rs2 fixed to zero.
    recipes.push(
        EncodingRecipeBuilder::new("SBzero", &formats.branch, 4)
            .operands_in(vec![gpr])
            .branch_range((0, 13))
            .emit(
                r#"
                    let dest = i64::from(func.offsets[destination]);
                    let disp = dest - i64::from(sink.offset());
                    put_sb(bits, disp, in_reg0, 0, sink);
                "#,
            ),
    );

    // Spill of a GPR.
    recipes.push(
        EncodingRecipeBuilder::new("GPsp", &formats.unary, 4)
            .operands_in(vec![gpr])
            .operands_out(vec![Stack::new(gpr)])
            .emit("unimplemented!();"),
    );

    // Fill of a GPR.
    recipes.push(
        EncodingRecipeBuilder::new("GPfi", &formats.unary, 4)
            .operands_in(vec![Stack::new(gpr)])
            .operands_out(vec![gpr])
            .emit("unimplemented!();"),
    );

    // Stack-slot to same stack-slot copy, which is guaranteed to turn into a no-op.
    recipes.push(
        EncodingRecipeBuilder::new("stacknull", &formats.unary, 0)
            .operands_in(vec![Stack::new(gpr)])
            .operands_out(vec![Stack::new(gpr)])
            .emit(""),
    );

    // No-op fills, created by late-stage redundant-fill removal.
    recipes.push(
        EncodingRecipeBuilder::new("fillnull", &formats.unary, 0)
            .operands_in(vec![Stack::new(gpr)])
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .emit(""),
    );

    recipes
}

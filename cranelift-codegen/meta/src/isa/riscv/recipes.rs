use std::collections::HashMap;

use crate::cdsl::formats::FormatRegistry;
use crate::cdsl::instructions::InstructionPredicate;
use crate::cdsl::recipes::{EncodingRecipeBuilder, EncodingRecipeNumber, Recipes, Stack};
use crate::cdsl::regs::IsaRegs;
use crate::shared::Definitions as SharedDefinitions;

/// An helper to create recipes and use them when defining the RISCV encodings.
pub struct RecipeGroup<'formats> {
    /// Memoized format registry, to pass it to the builders.
    formats: &'formats FormatRegistry,

    /// The actualy list of recipes explicitly created in this file.
    pub recipes: Recipes,

    /// Provides fast lookup from a name to an encoding recipe.
    name_to_recipe: HashMap<String, EncodingRecipeNumber>,
}

impl<'formats> RecipeGroup<'formats> {
    fn new(formats: &'formats FormatRegistry) -> Self {
        Self {
            formats,
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
        let number = self.recipes.push(builder.build(self.formats));
        self.name_to_recipe.insert(name, number);
    }

    pub fn by_name(&self, name: &str) -> EncodingRecipeNumber {
        let number = *self
            .name_to_recipe
            .get(name)
            .expect(&format!("unknown riscv recipe name {}", name));
        number
    }

    pub fn collect(self) -> Recipes {
        self.recipes
    }
}

pub(crate) fn define<'formats>(
    shared_defs: &'formats SharedDefinitions,
    regs: &IsaRegs,
) -> RecipeGroup<'formats> {
    let formats = &shared_defs.format_registry;

    // Format shorthands.
    let f_binary = formats.by_name("Binary");
    let f_binary_imm = formats.by_name("BinaryImm");
    let f_branch = formats.by_name("Branch");
    let f_branch_icmp = formats.by_name("BranchIcmp");
    let f_call = formats.by_name("Call");
    let f_call_indirect = formats.by_name("CallIndirect");
    let f_copy_to_ssa = formats.by_name("CopyToSsa");
    let f_int_compare = formats.by_name("IntCompare");
    let f_int_compare_imm = formats.by_name("IntCompareImm");
    let f_jump = formats.by_name("Jump");
    let f_multiary = formats.by_name("MultiAry");
    let f_regmove = formats.by_name("RegMove");
    let f_unary = formats.by_name("Unary");
    let f_unary_imm = formats.by_name("UnaryImm");

    // Register classes shorthands.
    let gpr = regs.class_by_name("GPR");

    // Definitions.
    let mut recipes = RecipeGroup::new(&shared_defs.format_registry);

    // R-type 32-bit instructions: These are mostly binary arithmetic instructions.
    // The encbits are `opcode[6:2] | (funct3 << 5) | (funct7 << 8)
    recipes.push(
        EncodingRecipeBuilder::new("R", f_binary, 4)
            .operands_in(vec![gpr, gpr])
            .operands_out(vec![gpr])
            .emit("put_r(bits, in_reg0, in_reg1, out_reg0, sink);"),
    );

    // R-type with an immediate shift amount instead of rs2.
    recipes.push(
        EncodingRecipeBuilder::new("Rshamt", f_binary_imm, 4)
            .operands_in(vec![gpr])
            .operands_out(vec![gpr])
            .emit("put_rshamt(bits, in_reg0, imm.into(), out_reg0, sink);"),
    );

    // R-type encoding of an integer comparison.
    recipes.push(
        EncodingRecipeBuilder::new("Ricmp", f_int_compare, 4)
            .operands_in(vec![gpr, gpr])
            .operands_out(vec![gpr])
            .emit("put_r(bits, in_reg0, in_reg1, out_reg0, sink);"),
    );

    let format = formats.get(f_binary_imm);
    recipes.push(
        EncodingRecipeBuilder::new("Ii", f_binary_imm, 4)
            .operands_in(vec![gpr])
            .operands_out(vec![gpr])
            .inst_predicate(InstructionPredicate::new_is_signed_int(
                format, "imm", 12, 0,
            ))
            .emit("put_i(bits, in_reg0, imm.into(), out_reg0, sink);"),
    );

    // I-type instruction with a hardcoded %x0 rs1.
    let format = formats.get(f_unary_imm);
    recipes.push(
        EncodingRecipeBuilder::new("Iz", f_unary_imm, 4)
            .operands_out(vec![gpr])
            .inst_predicate(InstructionPredicate::new_is_signed_int(
                format, "imm", 12, 0,
            ))
            .emit("put_i(bits, 0, imm.into(), out_reg0, sink);"),
    );

    // I-type encoding of an integer comparison.
    let format = formats.get(f_int_compare_imm);
    recipes.push(
        EncodingRecipeBuilder::new("Iicmp", f_int_compare_imm, 4)
            .operands_in(vec![gpr])
            .operands_out(vec![gpr])
            .inst_predicate(InstructionPredicate::new_is_signed_int(
                format, "imm", 12, 0,
            ))
            .emit("put_i(bits, in_reg0, imm.into(), out_reg0, sink);"),
    );

    // I-type encoding for `jalr` as a return instruction. We won't use the immediate offset.  The
    // variable return values are not encoded.
    recipes.push(EncodingRecipeBuilder::new("Iret", f_multiary, 4).emit(
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
    ));

    // I-type encoding for `jalr` as a call_indirect.
    recipes.push(
        EncodingRecipeBuilder::new("Icall", f_call_indirect, 4)
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
        EncodingRecipeBuilder::new("Icopy", f_unary, 4)
            .operands_in(vec![gpr])
            .operands_out(vec![gpr])
            .emit("put_i(bits, in_reg0, 0, out_reg0, sink);"),
    );

    // Same for a GPR regmove.
    recipes.push(
        EncodingRecipeBuilder::new("Irmov", f_regmove, 4)
            .operands_in(vec![gpr])
            .emit("put_i(bits, src, 0, dst, sink);"),
    );

    // Same for copy-to-SSA -- GPR regmove.
    recipes.push(
        EncodingRecipeBuilder::new("copytossa", f_copy_to_ssa, 4)
            // No operands_in to mention, because a source register is specified directly.
            .operands_out(vec![gpr])
            .emit("put_i(bits, src, 0, out_reg0, sink);"),
    );

    // U-type instructions have a 20-bit immediate that targets bits 12-31.
    let format = formats.get(f_unary_imm);
    recipes.push(
        EncodingRecipeBuilder::new("U", f_unary_imm, 4)
            .operands_out(vec![gpr])
            .inst_predicate(InstructionPredicate::new_is_signed_int(
                format, "imm", 32, 12,
            ))
            .emit("put_u(bits, imm.into(), out_reg0, sink);"),
    );

    // UJ-type unconditional branch instructions.
    recipes.push(
        EncodingRecipeBuilder::new("UJ", f_jump, 4)
            .branch_range((0, 21))
            .emit(
                r#"
                    let dest = i64::from(func.offsets[destination]);
                    let disp = dest - i64::from(sink.offset());
                    put_uj(bits, disp, 0, sink);
                "#,
            ),
    );

    recipes.push(EncodingRecipeBuilder::new("UJcall", f_call, 4).emit(
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
        EncodingRecipeBuilder::new("SB", f_branch_icmp, 4)
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
        EncodingRecipeBuilder::new("SBzero", f_branch, 4)
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
        EncodingRecipeBuilder::new("GPsp", f_unary, 4)
            .operands_in(vec![gpr])
            .operands_out(vec![Stack::new(gpr)])
            .emit("unimplemented!();"),
    );

    // Fill of a GPR.
    recipes.push(
        EncodingRecipeBuilder::new("GPfi", f_unary, 4)
            .operands_in(vec![Stack::new(gpr)])
            .operands_out(vec![gpr])
            .emit("unimplemented!();"),
    );

    // Stack-slot to same stack-slot copy, which is guaranteed to turn into a no-op.
    recipes.push(
        EncodingRecipeBuilder::new("stacknull", f_unary, 0)
            .operands_in(vec![Stack::new(gpr)])
            .operands_out(vec![Stack::new(gpr)])
            .emit(""),
    );

    // No-op fills, created by late-stage redundant-fill removal.
    recipes.push(
        EncodingRecipeBuilder::new("fillnull", f_unary, 0)
            .operands_in(vec![Stack::new(gpr)])
            .operands_out(vec![gpr])
            .clobbers_flags(false)
            .emit(""),
    );

    recipes
}

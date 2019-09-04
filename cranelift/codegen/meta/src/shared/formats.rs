use crate::cdsl::formats::{FormatRegistry, InstructionFormatBuilder as Builder};
use crate::shared::{entities::EntityRefs, immediates::Immediates};

pub(crate) fn define(imm: &Immediates, entities: &EntityRefs) -> FormatRegistry {
    let mut registry = FormatRegistry::new();

    registry.insert(Builder::new("Unary").value());
    registry.insert(Builder::new("UnaryImm").imm(&imm.imm64));
    registry.insert(Builder::new("UnaryImm128").imm(&imm.uimm128));
    registry.insert(Builder::new("UnaryIeee32").imm(&imm.ieee32));
    registry.insert(Builder::new("UnaryIeee64").imm(&imm.ieee64));
    registry.insert(Builder::new("UnaryBool").imm(&imm.boolean));
    registry.insert(Builder::new("UnaryGlobalValue").imm(&entities.global_value));

    registry.insert(Builder::new("Binary").value().value());
    registry.insert(Builder::new("BinaryImm").value().imm(&imm.imm64));

    // The select instructions are controlled by the second VALUE operand.
    // The first VALUE operand is the controlling flag which has a derived type.
    // The fma instruction has the same constraint on all inputs.
    registry.insert(
        Builder::new("Ternary")
            .value()
            .value()
            .value()
            .typevar_operand(1),
    );

    // Catch-all for instructions with many outputs and inputs and no immediate
    // operands.
    registry.insert(Builder::new("MultiAry").varargs());

    registry.insert(Builder::new("NullAry"));

    registry.insert(
        Builder::new("InsertLane")
            .value()
            .imm_with_name("lane", &imm.uimm8)
            .value(),
    );
    registry.insert(
        Builder::new("ExtractLane")
            .value()
            .imm_with_name("lane", &imm.uimm8),
    );

    registry.insert(Builder::new("IntCompare").imm(&imm.intcc).value().value());
    registry.insert(
        Builder::new("IntCompareImm")
            .imm(&imm.intcc)
            .value()
            .imm(&imm.imm64),
    );
    registry.insert(Builder::new("IntCond").imm(&imm.intcc).value());

    registry.insert(
        Builder::new("FloatCompare")
            .imm(&imm.floatcc)
            .value()
            .value(),
    );
    registry.insert(Builder::new("FloatCond").imm(&imm.floatcc).value());;

    registry.insert(
        Builder::new("IntSelect")
            .imm(&imm.intcc)
            .value()
            .value()
            .value(),
    );

    registry.insert(Builder::new("Jump").imm(&entities.ebb).varargs());
    registry.insert(Builder::new("Branch").value().imm(&entities.ebb).varargs());
    registry.insert(
        Builder::new("BranchInt")
            .imm(&imm.intcc)
            .value()
            .imm(&entities.ebb)
            .varargs(),
    );
    registry.insert(
        Builder::new("BranchFloat")
            .imm(&imm.floatcc)
            .value()
            .imm(&entities.ebb)
            .varargs(),
    );
    registry.insert(
        Builder::new("BranchIcmp")
            .imm(&imm.intcc)
            .value()
            .value()
            .imm(&entities.ebb)
            .varargs(),
    );
    registry.insert(
        Builder::new("BranchTable")
            .value()
            .imm(&entities.ebb)
            .imm(&entities.jump_table),
    );
    registry.insert(
        Builder::new("BranchTableEntry")
            .value()
            .value()
            .imm(&imm.uimm8)
            .imm(&entities.jump_table),
    );
    registry.insert(Builder::new("BranchTableBase").imm(&entities.jump_table));
    registry.insert(
        Builder::new("IndirectJump")
            .value()
            .imm(&entities.jump_table),
    );

    registry.insert(Builder::new("Call").imm(&entities.func_ref).varargs());
    registry.insert(
        Builder::new("CallIndirect")
            .imm(&entities.sig_ref)
            .value()
            .varargs(),
    );
    registry.insert(Builder::new("FuncAddr").imm(&entities.func_ref));

    registry.insert(
        Builder::new("Load")
            .imm(&imm.memflags)
            .value()
            .imm(&imm.offset32),
    );
    registry.insert(
        Builder::new("LoadComplex")
            .imm(&imm.memflags)
            .varargs()
            .imm(&imm.offset32),
    );
    registry.insert(
        Builder::new("Store")
            .imm(&imm.memflags)
            .value()
            .value()
            .imm(&imm.offset32),
    );
    registry.insert(
        Builder::new("StoreComplex")
            .imm(&imm.memflags)
            .value()
            .varargs()
            .imm(&imm.offset32),
    );
    registry.insert(
        Builder::new("StackLoad")
            .imm(&entities.stack_slot)
            .imm(&imm.offset32),
    );
    registry.insert(
        Builder::new("StackStore")
            .value()
            .imm(&entities.stack_slot)
            .imm(&imm.offset32),
    );

    // Accessing a WebAssembly heap.
    registry.insert(
        Builder::new("HeapAddr")
            .imm(&entities.heap)
            .value()
            .imm(&imm.uimm32),
    );

    // Accessing a WebAssembly table.
    registry.insert(
        Builder::new("TableAddr")
            .imm(&entities.table)
            .value()
            .imm(&imm.offset32),
    );

    registry.insert(
        Builder::new("RegMove")
            .value()
            .imm_with_name("src", &imm.regunit)
            .imm_with_name("dst", &imm.regunit),
    );
    registry.insert(
        Builder::new("CopySpecial")
            .imm_with_name("src", &imm.regunit)
            .imm_with_name("dst", &imm.regunit),
    );
    registry.insert(Builder::new("CopyToSsa").imm_with_name("src", &imm.regunit));
    registry.insert(
        Builder::new("RegSpill")
            .value()
            .imm_with_name("src", &imm.regunit)
            .imm_with_name("dst", &entities.stack_slot),
    );
    registry.insert(
        Builder::new("RegFill")
            .value()
            .imm_with_name("src", &entities.stack_slot)
            .imm_with_name("dst", &imm.regunit),
    );

    registry.insert(Builder::new("Trap").imm(&imm.trapcode));
    registry.insert(Builder::new("CondTrap").value().imm(&imm.trapcode));
    registry.insert(
        Builder::new("IntCondTrap")
            .imm(&imm.intcc)
            .value()
            .imm(&imm.trapcode),
    );
    registry.insert(
        Builder::new("FloatCondTrap")
            .imm(&imm.floatcc)
            .value()
            .imm(&imm.trapcode),
    );

    registry
}

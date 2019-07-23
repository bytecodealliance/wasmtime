use crate::cdsl::formats::{FormatRegistry, InstructionFormatBuilder as Builder};
use crate::shared::OperandKinds;

pub fn define(immediates: &OperandKinds, entities: &OperandKinds) -> FormatRegistry {
    // Shorthands for immediates.
    let uimm8 = immediates.by_name("uimm8");
    let uimm32 = immediates.by_name("uimm32");
    let uimm128 = immediates.by_name("uimm128");
    let imm64 = immediates.by_name("imm64");
    let ieee32 = immediates.by_name("ieee32");
    let ieee64 = immediates.by_name("ieee64");
    let boolean = immediates.by_name("boolean");
    let intcc = immediates.by_name("intcc");
    let floatcc = immediates.by_name("floatcc");
    let memflags = immediates.by_name("memflags");
    let offset32 = immediates.by_name("offset32");
    let trapcode = immediates.by_name("trapcode");
    let regunit = immediates.by_name("regunit");

    // Shorthands for entities.
    let global_value = entities.by_name("global_value");
    let ebb = entities.by_name("ebb");
    let jump_table = entities.by_name("jump_table");
    let func_ref = entities.by_name("func_ref");
    let sig_ref = entities.by_name("sig_ref");
    let stack_slot = entities.by_name("stack_slot");
    let heap = entities.by_name("heap");
    let table = entities.by_name("table");

    let mut registry = FormatRegistry::new();

    registry.insert(Builder::new("Unary").value());
    registry.insert(Builder::new("UnaryImm").imm(imm64));
    registry.insert(Builder::new("UnaryImm128").imm(uimm128));
    registry.insert(Builder::new("UnaryIeee32").imm(ieee32));
    registry.insert(Builder::new("UnaryIeee64").imm(ieee64));
    registry.insert(Builder::new("UnaryBool").imm(boolean));
    registry.insert(Builder::new("UnaryGlobalValue").imm(global_value));

    registry.insert(Builder::new("Binary").value().value());
    registry.insert(Builder::new("BinaryImm").value().imm(imm64));

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
            .imm(("lane", uimm8))
            .value(),
    );
    registry.insert(Builder::new("ExtractLane").value().imm(("lane", uimm8)));

    registry.insert(Builder::new("IntCompare").imm(intcc).value().value());
    registry.insert(Builder::new("IntCompareImm").imm(intcc).value().imm(imm64));
    registry.insert(Builder::new("IntCond").imm(intcc).value());

    registry.insert(Builder::new("FloatCompare").imm(floatcc).value().value());
    registry.insert(Builder::new("FloatCond").imm(floatcc).value());;

    registry.insert(Builder::new("IntSelect").imm(intcc).value().value().value());

    registry.insert(Builder::new("Jump").imm(ebb).varargs());
    registry.insert(Builder::new("Branch").value().imm(ebb).varargs());
    registry.insert(
        Builder::new("BranchInt")
            .imm(intcc)
            .value()
            .imm(ebb)
            .varargs(),
    );
    registry.insert(
        Builder::new("BranchFloat")
            .imm(floatcc)
            .value()
            .imm(ebb)
            .varargs(),
    );
    registry.insert(
        Builder::new("BranchIcmp")
            .imm(intcc)
            .value()
            .value()
            .imm(ebb)
            .varargs(),
    );
    registry.insert(Builder::new("BranchTable").value().imm(ebb).imm(jump_table));
    registry.insert(
        Builder::new("BranchTableEntry")
            .value()
            .value()
            .imm(uimm8)
            .imm(jump_table),
    );
    registry.insert(Builder::new("BranchTableBase").imm(jump_table));
    registry.insert(Builder::new("IndirectJump").value().imm(jump_table));

    registry.insert(Builder::new("Call").imm(func_ref).varargs());
    registry.insert(Builder::new("CallIndirect").imm(sig_ref).value().varargs());
    registry.insert(Builder::new("FuncAddr").imm(func_ref));

    registry.insert(Builder::new("Load").imm(memflags).value().imm(offset32));
    registry.insert(
        Builder::new("LoadComplex")
            .imm(memflags)
            .varargs()
            .imm(offset32),
    );
    registry.insert(
        Builder::new("Store")
            .imm(memflags)
            .value()
            .value()
            .imm(offset32),
    );
    registry.insert(
        Builder::new("StoreComplex")
            .imm(memflags)
            .value()
            .varargs()
            .imm(offset32),
    );
    registry.insert(Builder::new("StackLoad").imm(stack_slot).imm(offset32));
    registry.insert(
        Builder::new("StackStore")
            .value()
            .imm(stack_slot)
            .imm(offset32),
    );

    // Accessing a WebAssembly heap.
    registry.insert(Builder::new("HeapAddr").imm(heap).value().imm(uimm32));

    // Accessing a WebAssembly table.
    registry.insert(Builder::new("TableAddr").imm(table).value().imm(offset32));

    registry.insert(
        Builder::new("RegMove")
            .value()
            .imm(("src", regunit))
            .imm(("dst", regunit)),
    );
    registry.insert(
        Builder::new("CopySpecial")
            .imm(("src", regunit))
            .imm(("dst", regunit)),
    );
    registry.insert(Builder::new("CopyToSsa").imm(("src", regunit)));
    registry.insert(
        Builder::new("RegSpill")
            .value()
            .imm(("src", regunit))
            .imm(("dst", stack_slot)),
    );
    registry.insert(
        Builder::new("RegFill")
            .value()
            .imm(("src", stack_slot))
            .imm(("dst", regunit)),
    );

    registry.insert(Builder::new("Trap").imm(trapcode));
    registry.insert(Builder::new("CondTrap").value().imm(trapcode));
    registry.insert(Builder::new("IntCondTrap").imm(intcc).value().imm(trapcode));
    registry.insert(
        Builder::new("FloatCondTrap")
            .imm(floatcc)
            .value()
            .imm(trapcode),
    );

    registry
}

use crate::cdsl::operands::{OperandKind, OperandKindBuilder as Builder};

use std::collections::HashMap;

pub fn define() -> Vec<OperandKind> {
    let mut kinds = Vec::new();

    // A 64-bit immediate integer operand.
    //
    // This type of immediate integer can interact with SSA values with any
    // IntType type.
    let imm64 = Builder::new_imm("imm64")
        .doc("A 64-bit immediate integer.")
        .build();
    kinds.push(imm64);

    // An unsigned 8-bit immediate integer operand.
    //
    // This small operand is used to indicate lane indexes in SIMD vectors and
    // immediate bit counts on shift instructions.
    let uimm8 = Builder::new_imm("uimm8")
        .doc("An 8-bit immediate unsigned integer.")
        .build();
    kinds.push(uimm8);

    // An unsigned 32-bit immediate integer operand.
    let uimm32 = Builder::new_imm("uimm32")
        .doc("A 32-bit immediate unsigned integer.")
        .build();
    kinds.push(uimm32);

    // An unsigned 128-bit immediate integer operand.
    //
    // This operand is used to pass entire 128-bit vectors as immediates to
    // instructions like const.
    let uimm128 = Builder::new_imm("uimm128")
        .doc("A 128-bit immediate unsigned integer.")
        .rust_type("ir::Constant")
        .build();
    kinds.push(uimm128);

    // A 32-bit immediate signed offset.
    //
    // This is used to represent an immediate address offset in load/store
    // instructions.
    let offset32 = Builder::new_imm("offset32")
        .doc("A 32-bit immediate signed offset.")
        .default_member("offset")
        .build();
    kinds.push(offset32);

    // A 32-bit immediate floating point operand.
    //
    // IEEE 754-2008 binary32 interchange format.
    let ieee32 = Builder::new_imm("ieee32")
        .doc("A 32-bit immediate floating point number.")
        .build();
    kinds.push(ieee32);

    // A 64-bit immediate floating point operand.
    //
    // IEEE 754-2008 binary64 interchange format.
    let ieee64 = Builder::new_imm("ieee64")
        .doc("A 64-bit immediate floating point number.")
        .build();
    kinds.push(ieee64);

    // An immediate boolean operand.
    //
    // This type of immediate boolean can interact with SSA values with any
    // BoolType type.
    let boolean = Builder::new_imm("boolean")
        .doc("An immediate boolean.")
        .rust_type("bool")
        .build();
    kinds.push(boolean);

    // A condition code for comparing integer values.
    // This enumerated operand kind is used for the `icmp` instruction and corresponds to the
    // condcodes::IntCC` Rust type.
    let mut intcc_values = HashMap::new();
    intcc_values.insert("eq", "Equal");
    intcc_values.insert("ne", "NotEqual");
    intcc_values.insert("sge", "SignedGreaterThanOrEqual");
    intcc_values.insert("sgt", "SignedGreaterThan");
    intcc_values.insert("sle", "SignedLessThanOrEqual");
    intcc_values.insert("slt", "SignedLessThan");
    intcc_values.insert("uge", "UnsignedGreaterThanOrEqual");
    intcc_values.insert("ugt", "UnsignedGreaterThan");
    intcc_values.insert("ule", "UnsignedLessThanOrEqual");
    intcc_values.insert("ult", "UnsignedLessThan");
    let intcc = Builder::new_enum("intcc", intcc_values)
        .doc("An integer comparison condition code.")
        .default_member("cond")
        .rust_type("ir::condcodes::IntCC")
        .build();
    kinds.push(intcc);

    // A condition code for comparing floating point values.  This enumerated operand kind is used
    // for the `fcmp` instruction and corresponds to the `condcodes::FloatCC` Rust type.
    let mut floatcc_values = HashMap::new();
    floatcc_values.insert("ord", "Ordered");
    floatcc_values.insert("uno", "Unordered");
    floatcc_values.insert("eq", "Equal");
    floatcc_values.insert("ne", "NotEqual");
    floatcc_values.insert("one", "OrderedNotEqual");
    floatcc_values.insert("ueq", "UnorderedOrEqual");
    floatcc_values.insert("lt", "LessThan");
    floatcc_values.insert("le", "LessThanOrEqual");
    floatcc_values.insert("gt", "GreaterThan");
    floatcc_values.insert("ge", "GreaterThanOrEqual");
    floatcc_values.insert("ult", "UnorderedOrLessThan");
    floatcc_values.insert("ule", "UnorderedOrLessThanOrEqual");
    floatcc_values.insert("ugt", "UnorderedOrGreaterThan");
    floatcc_values.insert("uge", "UnorderedOrGreaterThanOrEqual");
    let floatcc = Builder::new_enum("floatcc", floatcc_values)
        .doc("A floating point comparison condition code")
        .default_member("cond")
        .rust_type("ir::condcodes::FloatCC")
        .build();
    kinds.push(floatcc);

    // Flags for memory operations like :clif:inst:`load` and :clif:inst:`store`.
    let memflags = Builder::new_imm("memflags")
        .doc("Memory operation flags")
        .default_member("flags")
        .rust_type("ir::MemFlags")
        .build();
    kinds.push(memflags);

    // A register unit in the current target ISA.
    let regunit = Builder::new_imm("regunit")
        .doc("A register unit in the target ISA")
        .rust_type("isa::RegUnit")
        .build();
    kinds.push(regunit);

    // A trap code indicating the reason for trapping.
    //
    // The Rust enum type also has a `User(u16)` variant for user-provided trap
    // codes.
    let mut trapcode_values = HashMap::new();
    trapcode_values.insert("stk_ovf", "StackOverflow");
    trapcode_values.insert("heap_oob", "HeapOutOfBounds");
    trapcode_values.insert("int_ovf", "IntegerOverflow");
    trapcode_values.insert("int_divz", "IntegerDivisionByZero");
    let trapcode = Builder::new_enum("trapcode", trapcode_values)
        .doc("A trap reason code.")
        .default_member("code")
        .rust_type("ir::TrapCode")
        .build();
    kinds.push(trapcode);

    return kinds;
}

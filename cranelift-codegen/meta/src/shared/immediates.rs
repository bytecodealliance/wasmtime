use crate::cdsl::operands::{OperandKind, OperandKindBuilder as Builder};

use std::collections::HashMap;

pub(crate) struct Immediates {
    /// A 64-bit immediate integer operand.
    ///
    /// This type of immediate integer can interact with SSA values with any IntType type.
    pub imm64: OperandKind,

    /// An unsigned 8-bit immediate integer operand.
    ///
    /// This small operand is used to indicate lane indexes in SIMD vectors and immediate bit
    /// counts on shift instructions.
    pub uimm8: OperandKind,

    /// An unsigned 32-bit immediate integer operand.
    pub uimm32: OperandKind,

    /// An unsigned 128-bit immediate integer operand.
    ///
    /// This operand is used to pass entire 128-bit vectors as immediates to instructions like
    /// const.
    pub uimm128: OperandKind,

    /// A 32-bit immediate signed offset.
    ///
    /// This is used to represent an immediate address offset in load/store instructions.
    pub offset32: OperandKind,

    /// A 32-bit immediate floating point operand.
    ///
    /// IEEE 754-2008 binary32 interchange format.
    pub ieee32: OperandKind,

    /// A 64-bit immediate floating point operand.
    ///
    /// IEEE 754-2008 binary64 interchange format.
    pub ieee64: OperandKind,

    /// An immediate boolean operand.
    ///
    /// This type of immediate boolean can interact with SSA values with any BoolType type.
    pub boolean: OperandKind,

    /// A condition code for comparing integer values.
    ///
    /// This enumerated operand kind is used for the `icmp` instruction and corresponds to the
    /// condcodes::IntCC` Rust type.
    pub intcc: OperandKind,

    /// A condition code for comparing floating point values.
    ///
    /// This enumerated operand kind is used for the `fcmp` instruction and corresponds to the
    /// `condcodes::FloatCC` Rust type.
    pub floatcc: OperandKind,

    /// Flags for memory operations like `load` and `store`.
    pub memflags: OperandKind,

    /// A register unit in the current target ISA.
    pub regunit: OperandKind,

    /// A trap code indicating the reason for trapping.
    ///
    /// The Rust enum type also has a `User(u16)` variant for user-provided trap codes.
    pub trapcode: OperandKind,
}

impl Immediates {
    pub fn new() -> Self {
        Self {
            imm64: Builder::new_imm("imm64")
                .doc("A 64-bit immediate integer.")
                .build(),

            uimm8: Builder::new_imm("uimm8")
                .doc("An 8-bit immediate unsigned integer.")
                .build(),

            uimm32: Builder::new_imm("uimm32")
                .doc("A 32-bit immediate unsigned integer.")
                .build(),

            uimm128: Builder::new_imm("uimm128")
                .doc("A 128-bit immediate unsigned integer.")
                .rust_type("ir::Constant")
                .build(),

            offset32: Builder::new_imm("offset32")
                .doc("A 32-bit immediate signed offset.")
                .default_member("offset")
                .build(),

            ieee32: Builder::new_imm("ieee32")
                .doc("A 32-bit immediate floating point number.")
                .build(),

            ieee64: Builder::new_imm("ieee64")
                .doc("A 64-bit immediate floating point number.")
                .build(),

            boolean: Builder::new_imm("boolean")
                .doc("An immediate boolean.")
                .rust_type("bool")
                .build(),

            intcc: {
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
                Builder::new_enum("intcc", intcc_values)
                    .doc("An integer comparison condition code.")
                    .default_member("cond")
                    .rust_type("ir::condcodes::IntCC")
                    .build()
            },

            floatcc: {
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
                Builder::new_enum("floatcc", floatcc_values)
                    .doc("A floating point comparison condition code")
                    .default_member("cond")
                    .rust_type("ir::condcodes::FloatCC")
                    .build()
            },

            memflags: Builder::new_imm("memflags")
                .doc("Memory operation flags")
                .default_member("flags")
                .rust_type("ir::MemFlags")
                .build(),

            regunit: Builder::new_imm("regunit")
                .doc("A register unit in the target ISA")
                .rust_type("isa::RegUnit")
                .build(),

            trapcode: {
                let mut trapcode_values = HashMap::new();
                trapcode_values.insert("stk_ovf", "StackOverflow");
                trapcode_values.insert("heap_oob", "HeapOutOfBounds");
                trapcode_values.insert("int_ovf", "IntegerOverflow");
                trapcode_values.insert("int_divz", "IntegerDivisionByZero");
                Builder::new_enum("trapcode", trapcode_values)
                    .doc("A trap reason code.")
                    .default_member("code")
                    .rust_type("ir::TrapCode")
                    .build()
            },
        }
    }
}

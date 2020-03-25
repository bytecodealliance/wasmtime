#![allow(non_snake_case)]

use crate::cdsl::instructions::{
    AllInstructions, InstructionBuilder as Inst, InstructionGroup, InstructionGroupBuilder,
};
use crate::cdsl::operands::Operand;
use crate::cdsl::types::ValueType;
use crate::cdsl::typevar::{Interval, TypeSetBuilder, TypeVar};
use crate::shared::entities::EntityRefs;
use crate::shared::formats::Formats;
use crate::shared::immediates::Immediates;
use crate::shared::types;

#[allow(clippy::many_single_char_names)]
pub(crate) fn define(
    mut all_instructions: &mut AllInstructions,
    formats: &Formats,
    immediates: &Immediates,
    entities: &EntityRefs,
) -> InstructionGroup {
    let mut ig = InstructionGroupBuilder::new(&mut all_instructions);

    let iflags: &TypeVar = &ValueType::Special(types::Flag::IFlags.into()).into();

    let iWord = &TypeVar::new(
        "iWord",
        "A scalar integer machine word",
        TypeSetBuilder::new().ints(32..64).build(),
    );
    let nlo = &Operand::new("nlo", iWord).with_doc("Low part of numerator");
    let nhi = &Operand::new("nhi", iWord).with_doc("High part of numerator");
    let d = &Operand::new("d", iWord).with_doc("Denominator");
    let q = &Operand::new("q", iWord).with_doc("Quotient");
    let r = &Operand::new("r", iWord).with_doc("Remainder");

    ig.push(
        Inst::new(
            "x86_udivmodx",
            r#"
        Extended unsigned division.

        Concatenate the bits in `nhi` and `nlo` to form the numerator.
        Interpret the bits as an unsigned number and divide by the unsigned
        denominator `d`. Trap when `d` is zero or if the quotient is larger
        than the range of the output.

        Return both quotient and remainder.
        "#,
            &formats.ternary,
        )
        .operands_in(vec![nlo, nhi, d])
        .operands_out(vec![q, r])
        .can_trap(true),
    );

    ig.push(
        Inst::new(
            "x86_sdivmodx",
            r#"
        Extended signed division.

        Concatenate the bits in `nhi` and `nlo` to form the numerator.
        Interpret the bits as a signed number and divide by the signed
        denominator `d`. Trap when `d` is zero or if the quotient is outside
        the range of the output.

        Return both quotient and remainder.
        "#,
            &formats.ternary,
        )
        .operands_in(vec![nlo, nhi, d])
        .operands_out(vec![q, r])
        .can_trap(true),
    );

    let argL = &Operand::new("argL", iWord);
    let argR = &Operand::new("argR", iWord);
    let resLo = &Operand::new("resLo", iWord);
    let resHi = &Operand::new("resHi", iWord);

    ig.push(
        Inst::new(
            "x86_umulx",
            r#"
        Unsigned integer multiplication, producing a double-length result.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
            &formats.binary,
        )
        .operands_in(vec![argL, argR])
        .operands_out(vec![resLo, resHi]),
    );

    ig.push(
        Inst::new(
            "x86_smulx",
            r#"
        Signed integer multiplication, producing a double-length result.

        Polymorphic over all scalar integer types, but does not support vector
        types.
        "#,
            &formats.binary,
        )
        .operands_in(vec![argL, argR])
        .operands_out(vec![resLo, resHi]),
    );

    let Float = &TypeVar::new(
        "Float",
        "A scalar or vector floating point number",
        TypeSetBuilder::new()
            .floats(Interval::All)
            .simd_lanes(Interval::All)
            .build(),
    );
    let IntTo = &TypeVar::new(
        "IntTo",
        "An integer type with the same number of lanes",
        TypeSetBuilder::new()
            .ints(32..64)
            .simd_lanes(Interval::All)
            .build(),
    );
    let x = &Operand::new("x", Float);
    let a = &Operand::new("a", IntTo);

    ig.push(
        Inst::new(
            "x86_cvtt2si",
            r#"
        Convert with truncation floating point to signed integer.

        The source floating point operand is converted to a signed integer by
        rounding towards zero. If the result can't be represented in the output
        type, returns the smallest signed value the output type can represent.

        This instruction does not trap.
        "#,
            &formats.unary,
        )
        .operands_in(vec![x])
        .operands_out(vec![a]),
    );

    let x = &Operand::new("x", Float);
    let a = &Operand::new("a", Float);
    let y = &Operand::new("y", Float);

    ig.push(
        Inst::new(
            "x86_fmin",
            r#"
        Floating point minimum with x86 semantics.

        This is equivalent to the C ternary operator `x < y ? x : y` which
        differs from `fmin` when either operand is NaN or when comparing
        +0.0 to -0.0.

        When the two operands don't compare as LT, `y` is returned unchanged,
        even if it is a signalling NaN.
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    ig.push(
        Inst::new(
            "x86_fmax",
            r#"
        Floating point maximum with x86 semantics.

        This is equivalent to the C ternary operator `x > y ? x : y` which
        differs from `fmax` when either operand is NaN or when comparing
        +0.0 to -0.0.

        When the two operands don't compare as GT, `y` is returned unchanged,
        even if it is a signalling NaN.
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    let x = &Operand::new("x", iWord);

    ig.push(
        Inst::new(
            "x86_push",
            r#"
    Pushes a value onto the stack.

    Decrements the stack pointer and stores the specified value on to the top.

    This is polymorphic in i32 and i64. However, it is only implemented for i64
    in 64-bit mode, and only for i32 in 32-bit mode.
    "#,
            &formats.unary,
        )
        .operands_in(vec![x])
        .other_side_effects(true)
        .can_store(true),
    );

    ig.push(
        Inst::new(
            "x86_pop",
            r#"
    Pops a value from the stack.

    Loads a value from the top of the stack and then increments the stack
    pointer.

    This is polymorphic in i32 and i64. However, it is only implemented for i64
    in 64-bit mode, and only for i32 in 32-bit mode.
    "#,
            &formats.nullary,
        )
        .operands_out(vec![x])
        .other_side_effects(true)
        .can_load(true),
    );

    let y = &Operand::new("y", iWord);
    let rflags = &Operand::new("rflags", iflags);

    ig.push(
        Inst::new(
            "x86_bsr",
            r#"
    Bit Scan Reverse -- returns the bit-index of the most significant 1
    in the word. Result is undefined if the argument is zero. However, it
    sets the Z flag depending on the argument, so it is at least easy to
    detect and handle that case.

    This is polymorphic in i32 and i64. It is implemented for both i64 and
    i32 in 64-bit mode, and only for i32 in 32-bit mode.
    "#,
            &formats.unary,
        )
        .operands_in(vec![x])
        .operands_out(vec![y, rflags]),
    );

    ig.push(
        Inst::new(
            "x86_bsf",
            r#"
    Bit Scan Forwards -- returns the bit-index of the least significant 1
    in the word. Is otherwise identical to 'bsr', just above.
    "#,
            &formats.unary,
        )
        .operands_in(vec![x])
        .operands_out(vec![y, rflags]),
    );

    let uimm8 = &immediates.uimm8;
    let TxN = &TypeVar::new(
        "TxN",
        "A SIMD vector type",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .floats(Interval::All)
            .bools(Interval::All)
            .simd_lanes(Interval::All)
            .includes_scalars(false)
            .build(),
    );
    let a = &Operand::new("a", TxN).with_doc("A vector value (i.e. held in an XMM register)");
    let b = &Operand::new("b", TxN).with_doc("A vector value (i.e. held in an XMM register)");
    let i = &Operand::new("i", uimm8).with_doc("An ordering operand controlling the copying of data from the source to the destination; see PSHUFD in Intel manual for details");

    ig.push(
        Inst::new(
            "x86_pshufd",
            r#"
    Packed Shuffle Doublewords -- copies data from either memory or lanes in an extended
    register and re-orders the data according to the passed immediate byte.
    "#,
            &formats.extract_lane,
        )
        .operands_in(vec![a, i]) // TODO allow copying from memory here (need more permissive type than TxN)
        .operands_out(vec![a]),
    );

    ig.push(
        Inst::new(
            "x86_pshufb",
            r#"
    Packed Shuffle Bytes -- re-orders data in an extended register using a shuffle
    mask from either memory or another extended register
    "#,
            &formats.binary,
        )
        .operands_in(vec![a, b]) // TODO allow re-ordering from memory here (need more permissive type than TxN)
        .operands_out(vec![a]),
    );

    let Idx = &Operand::new("Idx", uimm8).with_doc("Lane index");
    let x = &Operand::new("x", TxN);
    let a = &Operand::new("a", &TxN.lane_of());

    ig.push(
        Inst::new(
            "x86_pextr",
            r#"
        Extract lane ``Idx`` from ``x``.
        The lane index, ``Idx``, is an immediate value, not an SSA value. It
        must indicate a valid lane index for the type of ``x``.
        "#,
            &formats.extract_lane,
        )
        .operands_in(vec![x, Idx])
        .operands_out(vec![a]),
    );

    let IBxN = &TypeVar::new(
        "IBxN",
        "A SIMD vector type containing only booleans and integers",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .bools(Interval::All)
            .simd_lanes(Interval::All)
            .includes_scalars(false)
            .build(),
    );
    let x = &Operand::new("x", IBxN);
    let y = &Operand::new("y", &IBxN.lane_of()).with_doc("New lane value");
    let a = &Operand::new("a", IBxN);

    ig.push(
        Inst::new(
            "x86_pinsr",
            r#"
        Insert ``y`` into ``x`` at lane ``Idx``.
        The lane index, ``Idx``, is an immediate value, not an SSA value. It
        must indicate a valid lane index for the type of ``x``.
        "#,
            &formats.insert_lane,
        )
        .operands_in(vec![x, Idx, y])
        .operands_out(vec![a]),
    );

    let FxN = &TypeVar::new(
        "FxN",
        "A SIMD vector type containing floats",
        TypeSetBuilder::new()
            .floats(Interval::All)
            .simd_lanes(Interval::All)
            .includes_scalars(false)
            .build(),
    );
    let x = &Operand::new("x", FxN);
    let y = &Operand::new("y", &FxN.lane_of()).with_doc("New lane value");
    let a = &Operand::new("a", FxN);

    ig.push(
        Inst::new(
            "x86_insertps",
            r#"
        Insert a lane of ``y`` into ``x`` at using ``Idx`` to encode both which lane the value is
        extracted from and which it is inserted to. This is similar to x86_pinsr but inserts
        floats, which are already stored in an XMM register.
        "#,
            &formats.insert_lane,
        )
        .operands_in(vec![x, Idx, y])
        .operands_out(vec![a]),
    );

    let x = &Operand::new("x", TxN);
    let y = &Operand::new("y", TxN);
    let a = &Operand::new("a", TxN);

    ig.push(
        Inst::new(
            "x86_punpckh",
            r#"
        Unpack the high-order lanes of ``x`` and ``y`` and interleave into ``a``. With notional
        i8x4 vectors, where ``x = [x3, x2, x1, x0]`` and ``y = [y3, y2, y1, y0]``, this operation
        would result in ``a = [y3, x3, y2, x2]`` (using the Intel manual's right-to-left lane
        ordering). 
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    ig.push(
        Inst::new(
            "x86_punpckl",
            r#"
        Unpack the low-order lanes of ``x`` and ``y`` and interleave into ``a``. With notional
        i8x4 vectors, where ``x = [x3, x2, x1, x0]`` and ``y = [y3, y2, y1, y0]``, this operation
        would result in ``a = [y1, x1, y0, x0]`` (using the Intel manual's right-to-left lane
        ordering).
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    let I16xN = &TypeVar::new(
        "I16xN",
        "A SIMD vector type containing integers 16-bits wide and up",
        TypeSetBuilder::new()
            .ints(16..32)
            .simd_lanes(4..8)
            .includes_scalars(false)
            .build(),
    );

    let x = &Operand::new("x", I16xN);
    let y = &Operand::new("y", I16xN);
    let a = &Operand::new("a", &I16xN.split_lanes());

    ig.push(
        Inst::new(
            "x86_packss",
            r#"
        Convert packed signed integers the lanes of ``x`` and ``y`` into half-width integers, using
        signed saturation to handle overflows. For example, with notional i16x2 vectors, where 
        ``x = [x1, x0]`` and ``y = [y1, y0]``, this operation would result in 
        ``a = [y1', y0', x1', x0']`` (using the Intel manual's right-to-left lane ordering).
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    let x = &Operand::new("x", FxN);
    let y = &Operand::new("y", FxN);
    let a = &Operand::new("a", FxN);

    ig.push(
        Inst::new(
            "x86_movsd",
            r#"
        Move the low 64 bits of the float vector ``y`` to the low 64 bits of float vector ``x``
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    ig.push(
        Inst::new(
            "x86_movlhps",
            r#"
        Move the low 64 bits of the float vector ``y`` to the high 64 bits of float vector ``x``
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    let IxN = &TypeVar::new(
        "IxN",
        "A SIMD vector type containing integers",
        TypeSetBuilder::new()
            .ints(Interval::All)
            .simd_lanes(Interval::All)
            .includes_scalars(false)
            .build(),
    );
    let I64x2 = &TypeVar::new(
        "I64x2",
        "A SIMD vector type containing one large integer (the upper lane is concatenated with \
         the lower lane to form the integer)",
        TypeSetBuilder::new()
            .ints(64..64)
            .simd_lanes(2..2)
            .includes_scalars(false)
            .build(),
    );

    let x = &Operand::new("x", IxN).with_doc("Vector value to shift");
    let y = &Operand::new("y", I64x2).with_doc("Number of bits to shift");
    let a = &Operand::new("a", IxN);

    ig.push(
        Inst::new(
            "x86_psll",
            r#"
        Shift Packed Data Left Logical -- This implements the behavior of the shared instruction
        ``ishl`` but alters the shift operand to live in an XMM register as expected by the PSLL*
        family of instructions.
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    ig.push(
        Inst::new(
            "x86_psrl",
            r#"
        Shift Packed Data Right Logical -- This implements the behavior of the shared instruction
        ``ushr`` but alters the shift operand to live in an XMM register as expected by the PSRL*
        family of instructions.
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    ig.push(
        Inst::new(
            "x86_psra",
            r#"
        Shift Packed Data Right Arithmetic -- This implements the behavior of the shared
        instruction ``sshr`` but alters the shift operand to live in an XMM register as expected by
        the PSRA* family of instructions.
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    let x = &Operand::new("x", TxN);
    let y = &Operand::new("y", TxN);
    let f = &Operand::new("f", iflags);
    ig.push(
        Inst::new(
            "x86_ptest",
            r#"
        Logical Compare -- PTEST will set the ZF flag if all bits in the result are 0 of the
        bitwise AND of the first source operand (first operand) and the second source operand
        (second operand). PTEST sets the CF flag if all bits in the result are 0 of the bitwise
        AND of the second source operand (second operand) and the logical NOT of the destination
        operand (first operand).
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![f]),
    );

    let x = &Operand::new("x", IxN);
    let y = &Operand::new("y", IxN);
    let a = &Operand::new("a", IxN);
    ig.push(
        Inst::new(
            "x86_pmaxs",
            r#"
        Maximum of Packed Signed Integers -- Compare signed integers in the first and second
        operand and return the maximum values.
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    ig.push(
        Inst::new(
            "x86_pmaxu",
            r#"
        Maximum of Packed Unsigned Integers -- Compare unsigned integers in the first and second
        operand and return the maximum values.
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    ig.push(
        Inst::new(
            "x86_pmins",
            r#"
        Minimum of Packed Signed Integers -- Compare signed integers in the first and second
        operand and return the minimum values.
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    ig.push(
        Inst::new(
            "x86_pminu",
            r#"
        Minimum of Packed Unsigned Integers -- Compare unsigned integers in the first and second
        operand and return the minimum values.
        "#,
            &formats.binary,
        )
        .operands_in(vec![x, y])
        .operands_out(vec![a]),
    );

    let i64_t = &TypeVar::new(
        "i64_t",
        "A scalar 64bit integer",
        TypeSetBuilder::new().ints(64..64).build(),
    );

    let GV = &Operand::new("GV", &entities.global_value);
    let addr = &Operand::new("addr", i64_t);

    ig.push(
        Inst::new(
            "x86_elf_tls_get_addr",
            r#"
        Elf tls get addr -- This implements the GD TLS model for ELF. The clobber output should
        not be used.
            "#,
            &formats.unary_global_value,
        )
        // This is a bit overly broad to mark as clobbering *all* the registers, because it should
        // only preserve caller-saved registers. There's no way to indicate this to register
        // allocation yet, though, so mark as clobbering all registers instead.
        .clobbers_all_regs(true)
        .operands_in(vec![GV])
        .operands_out(vec![addr]),
    );
    ig.push(
        Inst::new(
            "x86_macho_tls_get_addr",
            r#"
        Mach-O tls get addr -- This implements TLS access for Mach-O. The clobber output should
        not be used.
            "#,
            &formats.unary_global_value,
        )
        // See above comment for x86_elf_tls_get_addr.
        .clobbers_all_regs(true)
        .operands_in(vec![GV])
        .operands_out(vec![addr]),
    );

    ig.build()
}

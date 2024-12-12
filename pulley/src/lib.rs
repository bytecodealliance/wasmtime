//! The pulley bytecode for fast interpreters.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(pulley_tail_calls, feature(explicit_tail_calls))]
#![cfg_attr(pulley_tail_calls, allow(incomplete_features, unstable_features))]
#![deny(missing_docs)]
#![no_std]
#![expect(clippy::allow_attributes_without_reason, reason = "crate not migrated")]

#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[allow(unused_extern_crates)] // Some cfg's don't use this.
extern crate alloc;

/// Calls the given macro with each opcode.
///
/// # Instruction Guidelines
///
/// We're inventing an instruction set here which naturally brings a whole set
/// of design questions. Note that this is explicitly intended to be only ever
/// used for Pulley where there are a different set of design constraints than
/// other instruction sets (e.g. general-purpose CPU ISAs). Some examples of
/// constraints for Pulley are:
///
/// * Instructions must be portable to many architectures.
/// * The Pulley ISA is mostly target-independent as the compilation target is
///   currently only parameterized on pointer width and endianness.
/// * Pulley instructions should be balance of time-to-decode and code size. For
///   example super fancy bit-packing tricks might be tough to decode in
///   software but might be worthwhile if it's quite common and greatly reduces
///   the size of bytecode. There's not a hard-and-fast answer here, but a
///   balance to be made.
/// * Many "macro ops" are present to reduce the size of compiled bytecode so
///   there is a wide set of duplicate functionality between opcodes (and this
///   is expected).
///
/// Given all this it's also useful to have a set of guidelines used to name and
/// develop Pulley instructions. As of the time of this writing it's still
/// pretty early days for Pulley so some of these guidelines may change over
/// time. Additionally instructions don't necessarily all follow these
/// conventions and that may also change over time. With that in mind, here's a
/// rough set of guidelines:
///
/// * Most instructions are prefixed with `x`, `f`, or `v`, indicating which
///   type of register they're operating on. (e.g. `xadd32` operates on the `x`
///   integer registers and `fadd32` operates on the `f` float registers).
///
/// * Most instructions are suffixed or otherwise contain the bit width they're
///   operating on. For example `xadd32` is a 32-bit addition.
///
/// * If an instruction operates on signed or unsigned data (such as division
///   and remainder), then the instruction is suffixed with `_s` or `_u`.
///
/// * Instructions operate on either 32 or 64-bit parts of a register.
///   Instructions modifying only 32-bits of a register always modify the "low"
///   part of a register and leave the upper part unmodified. This is intended
///   to help 32-bit platforms where if most operations are 32-bit there's no
///   need for extra instructions to sign or zero extend and modify the upper
///   half of the register.
///
/// * Binops use `BinaryOperands<T>` for the destination and argument registers.
///
/// * Instructions operating on memory contain a few pieces of information:
///
///   ```text
///   xload16le_u32_offset32
///   │└─┬┘└┤└┤ └┬┘ └──┬───┘
///   │  │  │ │  │     ▼
///   │  │  │ │  │     addressing mode
///   │  │  │ │  ▼
///   │  │  │ │  width of register modified + sign-extension (optional)
///   │  │  │ ▼
///   │  │  │ endianness of the operation (le/be)
///   │  │  ▼
///   │  │  bit-width of the operation
///   │  ▼
///   │  what's happening (load/store)
///   ▼
///   register being operated on (x/f/z)
///   ```
///
/// More guidelines might get added here over time, and if you have any
/// questions feel free to raise them and we can try to add them here as well!
#[macro_export]
macro_rules! for_each_op {
    ( $macro:ident ) => {
        $macro! {
            /// Transfer control the address in the `lr` register.
            ret = Ret;

            /// Transfer control to the PC at the given offset and set the `lr`
            /// register to the PC just after this instruction.
            call = Call { offset: PcRelOffset };

            /// Transfer control to the PC in `reg` and set `lr` to the PC just
            /// after this instruction.
            call_indirect = CallIndirect { reg: XReg };

            /// Unconditionally transfer control to the PC at the given offset.
            jump = Jump { offset: PcRelOffset };

            /// Conditionally transfer control to the given PC offset if
            /// `low32(cond)` contains a non-zero value.
            br_if32 = BrIf { cond: XReg, offset: PcRelOffset };

            /// Conditionally transfer control to the given PC offset if
            /// `low32(cond)` contains a zero value.
            br_if_not32 = BrIfNot { cond: XReg, offset: PcRelOffset };

            /// Branch if `a == b`.
            br_if_xeq32 = BrIfXeq32 { a: XReg, b: XReg, offset: PcRelOffset };
            /// Branch if `a != `b.
            br_if_xneq32 = BrIfXneq32 { a: XReg, b: XReg, offset: PcRelOffset };
            /// Branch if signed `a < b`.
            br_if_xslt32 = BrIfXslt32 { a: XReg, b: XReg, offset: PcRelOffset };
            /// Branch if signed `a <= b`.
            br_if_xslteq32 = BrIfXslteq32 { a: XReg, b: XReg, offset: PcRelOffset };
            /// Branch if unsigned `a < b`.
            br_if_xult32 = BrIfXult32 { a: XReg, b: XReg, offset: PcRelOffset };
            /// Branch if unsigned `a <= b`.
            br_if_xulteq32 = BrIfXulteq32 { a: XReg, b: XReg, offset: PcRelOffset };
            /// Branch if `a == b`.
            br_if_xeq64 = BrIfXeq64 { a: XReg, b: XReg, offset: PcRelOffset };
            /// Branch if `a != `b.
            br_if_xneq64 = BrIfXneq64 { a: XReg, b: XReg, offset: PcRelOffset };
            /// Branch if signed `a < b`.
            br_if_xslt64 = BrIfXslt64 { a: XReg, b: XReg, offset: PcRelOffset };
            /// Branch if signed `a <= b`.
            br_if_xslteq64 = BrIfXslteq64 { a: XReg, b: XReg, offset: PcRelOffset };
            /// Branch if unsigned `a < b`.
            br_if_xult64 = BrIfXult64 { a: XReg, b: XReg, offset: PcRelOffset };
            /// Branch if unsigned `a <= b`.
            br_if_xulteq64 = BrIfXulteq64 { a: XReg, b: XReg, offset: PcRelOffset };

            /// Branch to the label indicated by `low32(idx)`.
            ///
            /// After this instruction are `amt` instances of `PcRelOffset`
            /// and the `idx` selects which one will be branched to. The value
            /// of `idx` is clamped to `amt - 1` (e.g. the last offset is the
            /// "default" one.
            br_table32 = BrTable32 { idx: XReg, amt: u32 };

            /// Move between `x` registers.
            xmov = Xmov { dst: XReg, src: XReg };
            /// Move between `f` registers.
            fmov = Fmov { dst: FReg, src: FReg };
            /// Move between `v` registers.
            vmov = Vmov { dst: VReg, src: VReg };

            /// Set `dst = sign_extend(imm8)`.
            xconst8 = Xconst8 { dst: XReg, imm: i8 };
            /// Set `dst = sign_extend(imm16)`.
            xconst16 = Xconst16 { dst: XReg, imm: i16 };
            /// Set `dst = sign_extend(imm32)`.
            xconst32 = Xconst32 { dst: XReg, imm: i32 };
            /// Set `dst = imm64`.
            xconst64 = Xconst64 { dst: XReg, imm: i64 };

            /// 32-bit wrapping addition: `low32(dst) = low32(src1) + low32(src2)`.
            ///
            /// The upper 32-bits of `dst` are unmodified.
            xadd32 = Xadd32 { operands: BinaryOperands<XReg> };

            /// 64-bit wrapping addition: `dst = src1 + src2`.
            xadd64 = Xadd64 { operands: BinaryOperands<XReg> };

            /// 32-bit checked unsigned addition: `low32(dst) = low32(src1) +
            /// low32(src2)`.
            ///
            /// The upper 32-bits of `dst` are unmodified. Traps if the addition
            /// overflows.
            xadd32_uoverflow_trap = Xadd32UoverflowTrap { operands: BinaryOperands<XReg> };

            /// 64-bit checked unsigned addition: `dst = src1 + src2`.
            xadd64_uoverflow_trap = Xadd64UoverflowTrap { operands: BinaryOperands<XReg> };

            /// 32-bit wrapping subtraction: `low32(dst) = low32(src1) - low32(src2)`.
            ///
            /// The upper 32-bits of `dst` are unmodified.
            xsub32 = Xsub32 { operands: BinaryOperands<XReg> };

            /// 64-bit wrapping subtraction: `dst = src1 - src2`.
            xsub64 = Xsub64 { operands: BinaryOperands<XReg> };

            /// `low32(dst) = low32(src1) * low32(src2)`
            xmul32 = XMul32 { operands: BinaryOperands<XReg> };

            /// `dst = src1 * src2`
            xmul64 = XMul64 { operands: BinaryOperands<XReg> };

            /// `low32(dst) = trailing_zeros(low32(src))`
            xctz32 = Xctz32 { dst: XReg, src: XReg };
            /// `dst = trailing_zeros(src)`
            xctz64 = Xctz64 { dst: XReg, src: XReg };

            /// `low32(dst) = leading_zeros(low32(src))`
            xclz32 = Xclz32 { dst: XReg, src: XReg };
            /// `dst = leading_zeros(src)`
            xclz64 = Xclz64 { dst: XReg, src: XReg };

            /// `low32(dst) = count_ones(low32(src))`
            xpopcnt32 = Xpopcnt32 { dst: XReg, src: XReg };
            /// `dst = count_ones(src)`
            xpopcnt64 = Xpopcnt64 { dst: XReg, src: XReg };

            /// `low32(dst) = rotate_left(low32(src1), low32(src2))`
            xrotl32 = Xrotl32 { operands: BinaryOperands<XReg> };
            /// `dst = rotate_left(src1, src2)`
            xrotl64 = Xrotl64 { operands: BinaryOperands<XReg> };

            /// `low32(dst) = rotate_right(low32(src1), low32(src2))`
            xrotr32 = Xrotr32 { operands: BinaryOperands<XReg> };
            /// `dst = rotate_right(src1, src2)`
            xrotr64 = Xrotr64 { operands: BinaryOperands<XReg> };

            /// `low32(dst) = low32(src1) << low5(src2)`
            xshl32 = Xshl32 { operands: BinaryOperands<XReg> };
            /// `low32(dst) = low32(src1) >> low5(src2)`
            xshr32_s = Xshr32S { operands: BinaryOperands<XReg> };
            /// `low32(dst) = low32(src1) >> low5(src2)`
            xshr32_u = Xshr32U { operands: BinaryOperands<XReg> };
            /// `dst = src1 << low5(src2)`
            xshl64 = Xshl64 { operands: BinaryOperands<XReg> };
            /// `dst = src1 >> low6(src2)`
            xshr64_s = Xshr64S { operands: BinaryOperands<XReg> };
            /// `dst = src1 >> low6(src2)`
            xshr64_u = Xshr64U { operands: BinaryOperands<XReg> };

            /// `low32(dst) = src1 == src2`
            xeq64 = Xeq64 { operands: BinaryOperands<XReg> };
            /// `low32(dst) = src1 != src2`
            xneq64 = Xneq64 { operands: BinaryOperands<XReg> };
            /// `low32(dst) = src1 < src2` (signed)
            xslt64 = Xslt64 { operands: BinaryOperands<XReg> };
            /// `low32(dst) = src1 <= src2` (signed)
            xslteq64 = Xslteq64 { operands: BinaryOperands<XReg> };
            /// `low32(dst) = src1 < src2` (unsigned)
            xult64 = Xult64 { operands: BinaryOperands<XReg> };
            /// `low32(dst) = src1 <= src2` (unsigned)
            xulteq64 = Xulteq64 { operands: BinaryOperands<XReg> };
            /// `low32(dst) = low32(src1) == low32(src2)`
            xeq32 = Xeq32 { operands: BinaryOperands<XReg> };
            /// `low32(dst) = low32(src1) != low32(src2)`
            xneq32 = Xneq32 { operands: BinaryOperands<XReg> };
            /// `low32(dst) = low32(src1) < low32(src2)` (signed)
            xslt32 = Xslt32 { operands: BinaryOperands<XReg> };
            /// `low32(dst) = low32(src1) <= low32(src2)` (signed)
            xslteq32 = Xslteq32 { operands: BinaryOperands<XReg> };
            /// `low32(dst) = low32(src1) < low32(src2)` (unsigned)
            xult32 = Xult32 { operands: BinaryOperands<XReg> };
            /// `low32(dst) = low32(src1) <= low32(src2)` (unsigned)
            xulteq32 = Xulteq32 { operands: BinaryOperands<XReg> };

            /// `low32(dst) = zext(*(ptr + offset))`
            xload8_u32_offset32 = XLoad8U32Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `low32(dst) = sext(*(ptr + offset))`
            xload8_s32_offset32 = XLoad8S32Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `low32(dst) = zext(*(ptr + offset))`
            xload16le_u32_offset32 = XLoad16LeU32Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `low32(dst) = sext(*(ptr + offset))`
            xload16le_s32_offset32 = XLoad16LeS32Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `low32(dst) = *(ptr + offset)`
            xload32le_offset32 = XLoad32LeOffset32 { dst: XReg, ptr: XReg, offset: i32 };

            /// `dst = zext(*(ptr + offset))`
            xload8_u64_offset32 = XLoad8U64Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `dst = sext(*(ptr + offset))`
            xload8_s64_offset32 = XLoad8S64Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `dst = zext(*(ptr + offset))`
            xload16le_u64_offset32 = XLoad16LeU64Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `dst = sext(*(ptr + offset))`
            xload16le_s64_offset32 = XLoad16LeS64Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `dst = zext(*(ptr + offset))`
            xload32le_u64_offset32 = XLoad32LeU64Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `dst = sext(*(ptr + offset))`
            xload32le_s64_offset32 = XLoad32LeS64Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `dst = *(ptr + offset)`
            xload64le_offset32 = XLoad64LeOffset32 { dst: XReg, ptr: XReg, offset: i32 };

            /// `*(ptr + offset) = low8(src)`
            xstore8_offset32 = XStore8Offset32 { ptr: XReg, offset: i32, src: XReg };
            /// `*(ptr + offset) = low16(src)`
            xstore16le_offset32 = XStore16LeOffset32 { ptr: XReg, offset: i32, src: XReg };
            /// `*(ptr + offset) = low32(src)`
            xstore32le_offset32 = XStore32LeOffset32 { ptr: XReg, offset: i32, src: XReg };
            /// `*(ptr + offset) = low64(src)`
            xstore64le_offset32 = XStore64LeOffset32 { ptr: XReg, offset: i32, src: XReg };

            /// `low32(dst) = zext(*(ptr + offset))`
            fload32le_offset32 = Fload32LeOffset32 { dst: FReg, ptr: XReg, offset: i32 };
            /// `dst = *(ptr + offset)`
            fload64le_offset32 = Fload64LeOffset32 { dst: FReg, ptr: XReg, offset: i32 };
            /// `*(ptr + offset) = low32(src)`
            fstore32le_offset32 = Fstore32LeOffset32 { ptr: XReg, offset: i32, src: FReg };
            /// `*(ptr + offset) = src`
            fstore64le_offset32 = Fstore64LeOffset32 { ptr: XReg, offset: i32, src: FReg };

            /// `dst = *(ptr + offset)`
            vload128le_offset32 = VLoad128Offset32 { dst: VReg, ptr: XReg, offset: i32 };
            /// `*(ptr + offset) = src`
            vstore128le_offset32 = Vstore128LeOffset32 { ptr: XReg, offset: i32, src: VReg };

            /// `push lr; push fp; fp = sp`
            push_frame = PushFrame ;
            /// `sp = fp; pop fp; pop lr`
            pop_frame = PopFrame ;

            /// `*sp = low32(src); sp = sp.checked_add(4)`
            xpush32 = XPush32 { src: XReg };
            /// `for src in srcs { xpush32 src }`
            xpush32_many = XPush32Many { srcs: RegSet<XReg> };
            /// `*sp = src; sp = sp.checked_add(8)`
            xpush64 = XPush64 { src: XReg };
            /// `for src in srcs { xpush64 src }`
            xpush64_many = XPush64Many { srcs: RegSet<XReg> };

            /// `*dst = *sp; sp -= 4`
            xpop32 = XPop32 { dst: XReg };
            /// `for dst in dsts.rev() { xpop32 dst }`
            xpop32_many = XPop32Many { dsts: RegSet<XReg> };
            /// `*dst = *sp; sp -= 8`
            xpop64 = XPop64 { dst: XReg };
            /// `for dst in dsts.rev() { xpop64 dst }`
            xpop64_many = XPop64Many { dsts: RegSet<XReg> };

            /// `low32(dst) = bitcast low32(src) as i32`
            bitcast_int_from_float_32 = BitcastIntFromFloat32 { dst: XReg, src: FReg };
            /// `dst = bitcast src as i64`
            bitcast_int_from_float_64 = BitcastIntFromFloat64 { dst: XReg, src: FReg };
            /// `low32(dst) = bitcast low32(src) as f32`
            bitcast_float_from_int_32 = BitcastFloatFromInt32 { dst: FReg, src: XReg };
            /// `dst = bitcast src as f64`
            bitcast_float_from_int_64 = BitcastFloatFromInt64 { dst: FReg, src: XReg };

            /// `sp = sp.checked_sub(amt)`
            stack_alloc32 = StackAlloc32 { amt: u32 };

            /// `sp = sp + amt`
            stack_free32 = StackFree32 { amt: u32 };

            /// `dst = zext(low8(src))`
            zext8 = Zext8 { dst: XReg, src: XReg };
            /// `dst = zext(low16(src))`
            zext16 = Zext16 { dst: XReg, src: XReg };
            /// `dst = zext(low32(src))`
            zext32 = Zext32 { dst: XReg, src: XReg };
            /// `dst = sext(low8(src))`
            sext8 = Sext8 { dst: XReg, src: XReg };
            /// `dst = sext(low16(src))`
            sext16 = Sext16 { dst: XReg, src: XReg };
            /// `dst = sext(low32(src))`
            sext32 = Sext32 { dst: XReg, src: XReg };

            /// `low32(dst) = low32(src1) / low32(src2)` (signed)
            xdiv32_s = XDiv32S { operands: BinaryOperands<XReg> };

            /// `dst = src1 / src2` (signed)
            xdiv64_s = XDiv64S { operands: BinaryOperands<XReg> };

            /// `low32(dst) = low32(src1) / low32(src2)` (unsigned)
            xdiv32_u = XDiv32U { operands: BinaryOperands<XReg> };

            /// `dst = src1 / src2` (unsigned)
            xdiv64_u = XDiv64U { operands: BinaryOperands<XReg> };

            /// `low32(dst) = low32(src1) % low32(src2)` (signed)
            xrem32_s = XRem32S { operands: BinaryOperands<XReg> };

            /// `dst = src1 / src2` (signed)
            xrem64_s = XRem64S { operands: BinaryOperands<XReg> };

            /// `low32(dst) = low32(src1) % low32(src2)` (unsigned)
            xrem32_u = XRem32U { operands: BinaryOperands<XReg> };

            /// `dst = src1 / src2` (unsigned)
            xrem64_u = XRem64U { operands: BinaryOperands<XReg> };

            /// `low32(dst) = low32(src1) & low32(src2)`
            xband32 = XBand32 { operands: BinaryOperands<XReg> };
            /// `dst = src1 & src2`
            xband64 = XBand64 { operands: BinaryOperands<XReg> };
            /// `low32(dst) = low32(src1) | low32(src2)`
            xbor32 = XBor32 { operands: BinaryOperands<XReg> };
            /// `dst = src1 | src2`
            xbor64 = XBor64 { operands: BinaryOperands<XReg> };

            /// `low32(dst) = low32(src1) ^ low32(src2)`
            xbxor32 = XBxor32 { operands: BinaryOperands<XReg> };
            /// `dst = src1 ^ src2`
            xbxor64 = XBxor64 { operands: BinaryOperands<XReg> };

            /// `low32(dst) = bits`
            fconst32 = FConst32 { dst: FReg, bits: u32 };
            /// `dst = bits`
            fconst64 = FConst64 { dst: FReg, bits: u64 };

            /// `low32(dst) = zext(src1 == src2)`
            feq32 = Feq32 { dst: XReg, src1: FReg, src2: FReg };
            /// `low32(dst) = zext(src1 != src2)`
            fneq32 = Fneq32 { dst: XReg, src1: FReg, src2: FReg };
            /// `low32(dst) = zext(src1 < src2)`
            flt32 = Flt32 { dst: XReg, src1: FReg, src2: FReg };
            /// `low32(dst) = zext(src1 <= src2)`
            flteq32 = Flteq32 { dst: XReg, src1: FReg, src2: FReg };
            /// `low32(dst) = zext(src1 == src2)`
            feq64 = Feq64 { dst: XReg, src1: FReg, src2: FReg };
            /// `low32(dst) = zext(src1 != src2)`
            fneq64 = Fneq64 { dst: XReg, src1: FReg, src2: FReg };
            /// `low32(dst) = zext(src1 < src2)`
            flt64 = Flt64 { dst: XReg, src1: FReg, src2: FReg };
            /// `low32(dst) = zext(src1 <= src2)`
            flteq64 = Flteq64 { dst: XReg, src1: FReg, src2: FReg };

            /// `low32(dst) = low32(cond) ? low32(if_nonzero) : low32(if_zero)`
            xselect32 = XSelect32 { dst: XReg, cond: XReg, if_nonzero: XReg, if_zero: XReg };
            /// `dst = low32(cond) ? if_nonzero : if_zero`
            xselect64 = XSelect64 { dst: XReg, cond: XReg, if_nonzero: XReg, if_zero: XReg };
            /// `low32(dst) = low32(cond) ? low32(if_nonzero) : low32(if_zero)`
            fselect32 = FSelect32 { dst: FReg, cond: XReg, if_nonzero: FReg, if_zero: FReg };
            /// `dst = low32(cond) ? if_nonzero : if_zero`
            fselect64 = FSelect64 { dst: FReg, cond: XReg, if_nonzero: FReg, if_zero: FReg };

            /// `low32(dst) = checked_f32_from_signed(low32(src))`
            f32_from_x32_s = F32FromX32S { dst: FReg, src: XReg };
            /// `low32(dst) = checked_f32_from_unsigned(low32(src))`
            f32_from_x32_u = F32FromX32U { dst: FReg, src: XReg };
            /// `low32(dst) = checked_f32_from_signed(src)`
            f32_from_x64_s = F32FromX64S { dst: FReg, src: XReg };
            /// `low32(dst) = checked_f32_from_unsigned(src)`
            f32_from_x64_u = F32FromX64U { dst: FReg, src: XReg };
            /// `dst = checked_f64_from_signed(low32(src))`
            f64_from_x32_s = F64FromX32S { dst: FReg, src: XReg };
            /// `dst = checked_f64_from_unsigned(low32(src))`
            f64_from_x32_u = F64FromX32U { dst: FReg, src: XReg };
            /// `dst = checked_f64_from_signed(src)`
            f64_from_x64_s = F64FromX64S { dst: FReg, src: XReg };
            /// `dst = checked_f64_from_unsigned(src)`
            f64_from_x64_u = F64FromX64U { dst: FReg, src: XReg };

            /// `low32(dst) = checked_signed_from_f32(low32(src))`
            x32_from_f32_s = X32FromF32S { dst: XReg, src: FReg };
            /// `low32(dst) = checked_unsigned_from_f32(low32(src))`
            x32_from_f32_u = X32FromF32U { dst: XReg, src: FReg };
            /// `low32(dst) = checked_signed_from_f64(src)`
            x32_from_f64_s = X32FromF64S { dst: XReg, src: FReg };
            /// `low32(dst) = checked_unsigned_from_f64(src)`
            x32_from_f64_u = X32FromF64U { dst: XReg, src: FReg };
            /// `dst = checked_signed_from_f32(low32(src))`
            x64_from_f32_s = X64FromF32S { dst: XReg, src: FReg };
            /// `dst = checked_unsigned_from_f32(low32(src))`
            x64_from_f32_u = X64FromF32U { dst: XReg, src: FReg };
            /// `dst = checked_signed_from_f64(src)`
            x64_from_f64_s = X64FromF64S { dst: XReg, src: FReg };
            /// `dst = checked_unsigned_from_f64(src)`
            x64_from_f64_u = X64FromF64U { dst: XReg, src: FReg };

            /// `low32(dst) = saturating_signed_from_f32(low32(src))`
            x32_from_f32_s_sat = X32FromF32SSat { dst: XReg, src: FReg };
            /// `low32(dst) = saturating_unsigned_from_f32(low32(src))`
            x32_from_f32_u_sat = X32FromF32USat { dst: XReg, src: FReg };
            /// `low32(dst) = saturating_signed_from_f64(src)`
            x32_from_f64_s_sat = X32FromF64SSat { dst: XReg, src: FReg };
            /// `low32(dst) = saturating_unsigned_from_f64(src)`
            x32_from_f64_u_sat = X32FromF64USat { dst: XReg, src: FReg };
            /// `dst = saturating_signed_from_f32(low32(src))`
            x64_from_f32_s_sat = X64FromF32SSat { dst: XReg, src: FReg };
            /// `dst = saturating_unsigned_from_f32(low32(src))`
            x64_from_f32_u_sat = X64FromF32USat { dst: XReg, src: FReg };
            /// `dst = saturating_signed_from_f64(src)`
            x64_from_f64_s_sat = X64FromF64SSat { dst: XReg, src: FReg };
            /// `dst = saturating_unsigned_from_f64(src)`
            x64_from_f64_u_sat = X64FromF64USat { dst: XReg, src: FReg };

            /// `low32(dst) = demote(src)`
            f32_from_f64 = F32FromF64 { dst: FReg, src: FReg };
            /// `(st) = promote(low32(src))`
            f64_from_f32 = F64FromF32 { dst: FReg, src: FReg };

            /// `low32(dst) = abs(low32(src1)) * sign(low32(src2))`
            fcopysign32 = FCopySign32 { dst: FReg, src1: FReg, src2: FReg };
            /// `dst = abs(src1) * sign(src2)`
            fcopysign64 = FCopySign64 { dst: FReg, src1: FReg, src2: FReg };
        }
    };
}

/// Calls the given macro with each extended opcode.
#[macro_export]
macro_rules! for_each_extended_op {
    ( $macro:ident ) => {
        $macro! {
            /// Raise a trap.
            trap = Trap;

            /// Do nothing.
            nop = Nop;

            /// A special opcode to halt interpreter execution and yield control
            /// back to the host.
            ///
            /// This opcode results in `DoneReason::CallIndirectHost` where the
            /// `id` here is shepherded along to the embedder. It's up to the
            /// embedder to determine what to do with the `id` and the current
            /// state of registers and the stack.
            ///
            /// In Wasmtime this is used to implement interpreter-to-host calls.
            /// This is modeled as a `call` instruction where the first
            /// parameter is the native function pointer to invoke and all
            /// remaining parameters for the native function are in following
            /// parameter positions (e.g. `x1`, `x2`, ...). The results of the
            /// host call are then store in `x0`.
            ///
            /// Handling this in Wasmtime is done through a "relocation" which
            /// is resolved at link-time when raw bytecode from Cranelift is
            /// assembled into the final object that Wasmtime will interpret.
            call_indirect_host = CallIndirectHost { id: u8 };

            /// `dst = byteswap(low32(src))`
            bswap32 = Bswap32 { dst: XReg, src: XReg };
            /// `dst = byteswap(src)`
            bswap64 = Bswap64 { dst: XReg, src: XReg };


            /// `dst = zext(*(ptr + offset))`
            xload16be_u64_offset32 = XLoad16BeU64Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `dst = sext(*(ptr + offset))`
            xload16be_s64_offset32 = XLoad16BeS64Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `dst = zext(*(ptr + offset))`
            xload32be_u64_offset32 = XLoad32BeU64Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `dst = sext(*(ptr + offset))`
            xload32be_s64_offset32 = XLoad32BeS64Offset32 { dst: XReg, ptr: XReg, offset: i32 };
            /// `dst = *(ptr + offset)`
            xload64be_offset32 = XLoad64BeOffset32 { dst: XReg, ptr: XReg, offset: i32 };

            /// `*(ptr + offset) = low16(src)`
            xstore16be_offset32 = XStore16BeOffset32 { ptr: XReg, offset: i32, src: XReg };
            /// `*(ptr + offset) = low32(src)`
            xstore32be_offset32 = XStore32BeOffset32 { ptr: XReg, offset: i32, src: XReg };
            /// `*(ptr + offset) = low64(src)`
            xstore64be_offset32 = XStore64BeOffset32 { ptr: XReg, offset: i32, src: XReg };

            /// `low32(dst) = zext(*(ptr + offset))`
            fload32be_offset32 = Fload32BeOffset32 { dst: FReg, ptr: XReg, offset: i32 };
            /// `dst = *(ptr + offset)`
            fload64be_offset32 = Fload64BeOffset32 { dst: FReg, ptr: XReg, offset: i32 };
            /// `*(ptr + offset) = low32(src)`
            fstore32be_offset32 = Fstore32BeOffset32 { ptr: XReg, offset: i32, src: FReg };
            /// `*(ptr + offset) = src`
            fstore64be_offset32 = Fstore64BeOffset32 { ptr: XReg, offset: i32, src: FReg };
        }
    };
}

#[cfg(feature = "decode")]
pub mod decode;
#[cfg(feature = "disas")]
pub mod disas;
#[cfg(feature = "encode")]
pub mod encode;
#[cfg(feature = "interp")]
pub mod interp;

pub mod regs;
pub use regs::*;

pub mod imms;
pub use imms::*;

pub mod op;
pub use op::*;

pub mod opcode;
pub use opcode::*;

#[allow(dead_code)] // Unused in some `cfg`s.
pub(crate) unsafe fn unreachable_unchecked<T>() -> T {
    #[cfg(debug_assertions)]
    unreachable!();

    #[cfg_attr(debug_assertions, allow(unreachable_code))]
    unsafe {
        core::hint::unreachable_unchecked()
    }
}

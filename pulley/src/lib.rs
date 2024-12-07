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

            /// Conditionally transfer control to the given PC offset if `cond`
            /// contains a non-zero value.
            br_if = BrIf { cond: XReg, offset: PcRelOffset };

            /// Conditionally transfer control to the given PC offset if `cond`
            /// contains a zero value.
            br_if_not = BrIfNot { cond: XReg, offset: PcRelOffset };

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

            /// Branch to the label indicated by `idx`.
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

            /// 64-bit equality.
            xeq64 = Xeq64 { operands: BinaryOperands<XReg> };
            /// 64-bit inequality.
            xneq64 = Xneq64 { operands: BinaryOperands<XReg> };
            /// 64-bit signed less-than.
            xslt64 = Xslt64 { operands: BinaryOperands<XReg> };
            /// 64-bit signed less-than-equal.
            xslteq64 = Xslteq64 { operands: BinaryOperands<XReg> };
            /// 64-bit unsigned less-than.
            xult64 = Xult64 { operands: BinaryOperands<XReg> };
            /// 64-bit unsigned less-than-equal.
            xulteq64 = Xulteq64 { operands: BinaryOperands<XReg> };
            /// 32-bit equality.
            xeq32 = Xeq32 { operands: BinaryOperands<XReg> };
            /// 32-bit inequality.
            xneq32 = Xneq32 { operands: BinaryOperands<XReg> };
            /// 32-bit signed less-than.
            xslt32 = Xslt32 { operands: BinaryOperands<XReg> };
            /// 32-bit signed less-than-equal.
            xslteq32 = Xslteq32 { operands: BinaryOperands<XReg> };
            /// 32-bit unsigned less-than.
            xult32 = Xult32 { operands: BinaryOperands<XReg> };
            /// 32-bit unsigned less-than-equal.
            xulteq32 = Xulteq32 { operands: BinaryOperands<XReg> };

            /// `dst = zero_extend(load32_le(ptr))`
            load32_u = Load32U { dst: XReg, ptr: XReg };
            /// `dst = sign_extend(load32_le(ptr))`
            load32_s = Load32S { dst: XReg, ptr: XReg };
            /// `dst = load64_le(ptr)`
            load64 = Load64 { dst: XReg, ptr: XReg };

            /// `dst = zero_extend(load32_le(ptr + offset8))`
            load32_u_offset8 = Load32UOffset8 { dst: XReg, ptr: XReg, offset: i8 };
            /// `dst = sign_extend(load32_le(ptr + offset8))`
            load32_s_offset8 = Load32SOffset8 { dst: XReg, ptr: XReg, offset: i8 };
            /// `dst = load64_le(ptr + offset8)`
            load64_offset8 = Load64Offset8 { dst: XReg, ptr: XReg, offset: i8 };

            /// `dst = zero_extend(load32_le(ptr + offset64))`
            load32_u_offset64 = Load32UOffset64 { dst: XReg, ptr: XReg, offset: i64 };
            /// `dst = sign_extend(load32_le(ptr + offset64))`
            load32_s_offset64 = Load32SOffset64 { dst: XReg, ptr: XReg, offset: i64 };
            /// `dst = load64_le(ptr + offset64)`
            load64_offset64 = Load64Offset64 { dst: XReg, ptr: XReg, offset: i64 };

            /// `*ptr = low32(src.to_le())`
            store32 = Store32 { ptr: XReg, src: XReg };
            /// `*ptr = src.to_le()`
            store64 = Store64 { ptr: XReg, src: XReg };

            /// `*(ptr + sign_extend(offset8)) = low32(src).to_le()`
            store32_offset8 = Store32SOffset8 { ptr: XReg, offset: i8, src: XReg };
            /// `*(ptr + sign_extend(offset8)) = src.to_le()`
            store64_offset8 = Store64Offset8 { ptr: XReg, offset: i8, src: XReg };

            /// `*(ptr + sign_extend(offset64)) = low32(src).to_le()`
            store32_offset64 = Store32SOffset64 { ptr: XReg, offset: i64, src: XReg };
            /// `*(ptr + sign_extend(offset64)) = src.to_le()`
            store64_offset64 = Store64Offset64 { ptr: XReg, offset: i64, src: XReg };

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

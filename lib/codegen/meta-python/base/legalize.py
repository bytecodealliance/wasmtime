"""
Patterns for legalizing the `base` instruction set.

The base Cranelift instruction set is 'fat', and many instructions don't have
legal representations in a given target ISA. This module defines legalization
patterns that describe how base instructions can be transformed to other base
instructions that are legal.
"""
from __future__ import absolute_import
from .immediates import intcc, imm64, ieee32, ieee64
from . import instructions as insts
from . import types
from .instructions import uextend, sextend, ireduce
from .instructions import iadd, iadd_cout, iadd_cin, iadd_carry, iadd_imm
from .instructions import isub, isub_bin, isub_bout, isub_borrow, irsub_imm
from .instructions import imul, imul_imm
from .instructions import sdiv, sdiv_imm, udiv, udiv_imm
from .instructions import srem, srem_imm, urem, urem_imm
from .instructions import band, bor, bxor, isplit, iconcat
from .instructions import bnot, band_not, bor_not, bxor_not
from .instructions import band_imm, bor_imm, bxor_imm
from .instructions import icmp, icmp_imm, ifcmp, ifcmp_imm
from .instructions import iconst, bint, select
from .instructions import ishl, ishl_imm, sshr, sshr_imm, ushr, ushr_imm
from .instructions import rotl, rotl_imm, rotr, rotr_imm
from .instructions import f32const, f64const
from .instructions import store, load
from .instructions import br_table
from .instructions import bitrev
from cdsl.ast import Var
from cdsl.xform import Rtl, XFormGroup

try:
    from typing import TYPE_CHECKING # noqa
    if TYPE_CHECKING:
        from cdsl.instructions import Instruction # noqa
except ImportError:
    TYPE_CHECKING = False


narrow = XFormGroup('narrow', """
        Legalize instructions by narrowing.

        The transformations in the 'narrow' group work by expressing
        instructions in terms of smaller types. Operations on vector types are
        expressed in terms of vector types with fewer lanes, and integer
        operations are expressed in terms of smaller integer types.
        """)

widen = XFormGroup('widen', """
        Legalize instructions by widening.

        The transformations in the 'widen' group work by expressing
        instructions in terms of larger types.
        """)

expand = XFormGroup('expand', """
        Legalize instructions by expansion.

        Rewrite instructions in terms of other instructions, generally
        operating on the same types as the original instructions.
        """)

expand_flags = XFormGroup('expand_flags', """
        Instruction expansions for architectures with flags.

        Expand some instructions using CPU flags, then fall back to the normal
        expansions. Not all architectures support CPU flags, so these patterns
        are kept separate.
        """, chain=expand)


# Custom expansions for memory objects.
expand.custom_legalize(insts.global_value, 'expand_global_value')
expand.custom_legalize(insts.heap_addr, 'expand_heap_addr')
expand.custom_legalize(insts.table_addr, 'expand_table_addr')

# Custom expansions for calls.
expand.custom_legalize(insts.call, 'expand_call')

# Custom expansions that need to change the CFG.
# TODO: Add sufficient XForm syntax that we don't need to hand-code these.
expand.custom_legalize(insts.trapz, 'expand_cond_trap')
expand.custom_legalize(insts.trapnz, 'expand_cond_trap')
expand.custom_legalize(insts.br_table, 'expand_br_table')
expand.custom_legalize(insts.select, 'expand_select')

# Custom expansions for floating point constants.
# These expansions require bit-casting or creating constant pool entries.
expand.custom_legalize(insts.f32const, 'expand_fconst')
expand.custom_legalize(insts.f64const, 'expand_fconst')

# Custom expansions for stack memory accesses.
expand.custom_legalize(insts.stack_load, 'expand_stack_load')
expand.custom_legalize(insts.stack_store, 'expand_stack_store')

x = Var('x')
y = Var('y')
z = Var('z')
a = Var('a')
a1 = Var('a1')
a2 = Var('a2')
a3 = Var('a3')
a4 = Var('a4')
b = Var('b')
b1 = Var('b1')
b2 = Var('b2')
b3 = Var('b3')
b4 = Var('b4')
b_in = Var('b_in')
b_int = Var('b_int')
c = Var('c')
c1 = Var('c1')
c2 = Var('c2')
c3 = Var('c3')
c4 = Var('c4')
c_in = Var('c_in')
c_int = Var('c_int')
d = Var('d')
d1 = Var('d1')
d2 = Var('d2')
d3 = Var('d3')
d4 = Var('d4')
e = Var('e')
e1 = Var('e1')
e2 = Var('e2')
e3 = Var('e3')
e4 = Var('e4')
f = Var('f')
f1 = Var('f1')
f2 = Var('f2')
xl = Var('xl')
xh = Var('xh')
yl = Var('yl')
yh = Var('yh')
al = Var('al')
ah = Var('ah')
cc = Var('cc')
ptr = Var('ptr')
flags = Var('flags')
offset = Var('off')
ss = Var('ss')

narrow.legalize(
        a << iadd(x, y),
        Rtl(
            (xl, xh) << isplit(x),
            (yl, yh) << isplit(y),
            (al, c) << iadd_cout(xl, yl),
            ah << iadd_cin(xh, yh, c),
            a << iconcat(al, ah)
        ))

narrow.legalize(
        a << isub(x, y),
        Rtl(
            (xl, xh) << isplit(x),
            (yl, yh) << isplit(y),
            (al, b) << isub_bout(xl, yl),
            ah << isub_bin(xh, yh, b),
            a << iconcat(al, ah)
        ))

for bitop in [band, bor, bxor]:
    narrow.legalize(
            a << bitop(x, y),
            Rtl(
                (xl, xh) << isplit(x),
                (yl, yh) << isplit(y),
                al << bitop(xl, yl),
                ah << bitop(xh, yh),
                a << iconcat(al, ah)
            ))

narrow.legalize(
        a << select(c, x, y),
        Rtl(
            (xl, xh) << isplit(x),
            (yl, yh) << isplit(y),
            al << select(c, xl, yl),
            ah << select(c, xh, yh),
            a << iconcat(al, ah)
        ))


def widen_one_arg(signed, op):
    # type: (bool, Instruction) -> None
    for int_ty in [types.i8, types.i16]:
        if signed:
            widen.legalize(
                a << op.bind(int_ty)(b),
                Rtl(
                    x << sextend.i32(b),
                    z << op.i32(x),
                    a << ireduce.bind(int_ty)(z)
                ))
        else:
            widen.legalize(
                a << op.bind(int_ty)(b),
                Rtl(
                    x << uextend.i32(b),
                    z << op.i32(x),
                    a << ireduce.bind(int_ty)(z)
                ))


def widen_two_arg(signed, op):
    # type: (bool, Instruction) -> None
    for int_ty in [types.i8, types.i16]:
        if signed:
            widen.legalize(
                a << op.bind(int_ty)(b, c),
                Rtl(
                    x << sextend.i32(b),
                    y << sextend.i32(c),
                    z << op.i32(x, y),
                    a << ireduce.bind(int_ty)(z)
                ))
        else:
            widen.legalize(
                a << op.bind(int_ty)(b, c),
                Rtl(
                    x << uextend.i32(b),
                    y << uextend.i32(c),
                    z << op.i32(x, y),
                    a << ireduce.bind(int_ty)(z)
                ))


def widen_imm(signed, op):
    # type: (bool, Instruction) -> None
    for int_ty in [types.i8, types.i16]:
        if signed:
            widen.legalize(
                a << op.bind(int_ty)(b, c),
                Rtl(
                    x << sextend.i32(b),
                    z << op.i32(x, c),
                    a << ireduce.bind(int_ty)(z)
                ))
        else:
            widen.legalize(
                a << op.bind(int_ty)(b, c),
                Rtl(
                    x << uextend.i32(b),
                    z << op.i32(x, c),
                    a << ireduce.bind(int_ty)(z)
                ))


# int ops
for binop in [iadd, isub, imul, udiv, urem]:
    widen_two_arg(False, binop)

for binop in [sdiv, srem]:
    widen_two_arg(True, binop)

for binop in [iadd_imm, imul_imm, udiv_imm, urem_imm]:
    widen_imm(False, binop)

for binop in [sdiv_imm, srem_imm]:
    widen_imm(True, binop)

widen_imm(False, irsub_imm)

# bit ops
widen_one_arg(False, bnot)

for binop in [band, bor, bxor, band_not, bor_not, bxor_not]:
    widen_two_arg(False, binop)

for binop in [band_imm, bor_imm, bxor_imm]:
    widen_imm(False, binop)

widen_one_arg(False, insts.popcnt)

for (int_ty, num) in [(types.i8, 24), (types.i16, 16)]:
    widen.legalize(
        a << insts.clz.bind(int_ty)(b),
        Rtl(
            c << uextend.i32(b),
            d << insts.clz.i32(c),
            e << iadd_imm(d, imm64(-num)),
            a << ireduce.bind(int_ty)(e)
        ))

    widen.legalize(
        a << insts.cls.bind(int_ty)(b),
        Rtl(
            c << sextend.i32(b),
            d << insts.cls.i32(c),
            e << iadd_imm(d, imm64(-num)),
            a << ireduce.bind(int_ty)(e)
        ))

for (int_ty, num) in [(types.i8, 1 << 8), (types.i16, 1 << 16)]:
    widen.legalize(
        a << insts.ctz.bind(int_ty)(b),
        Rtl(
            c << uextend.i32(b),
            # When `b` is zero, returns the size of x in bits.
            d << bor_imm(c, imm64(num)),
            e << insts.ctz.i32(d),
            a << ireduce.bind(int_ty)(e)
        ))

# iconst
for int_ty in [types.i8, types.i16]:
    widen.legalize(
        a << iconst.bind(int_ty)(b),
        Rtl(
            c << iconst.i32(b),
            a << ireduce.bind(int_ty)(c)
        ))

widen.legalize(
    a << uextend.i16.i8(b),
    Rtl(
        c << uextend.i32(b),
        a << ireduce(c)
    ))

widen.legalize(
    a << sextend.i16.i8(b),
    Rtl(
        c << sextend.i32(b),
        a << ireduce(c)
    ))


widen.legalize(
    store.i8(flags, a, ptr, offset),
    Rtl(
        b << uextend.i32(a),
        insts.istore8(flags, b, ptr, offset)
    ))

widen.legalize(
    store.i16(flags, a, ptr, offset),
    Rtl(
        b << uextend.i32(a),
        insts.istore16(flags, b, ptr, offset)
    ))

widen.legalize(
    a << load.i8(flags, ptr, offset),
    Rtl(
        b << insts.uload8.i32(flags, ptr, offset),
        a << ireduce(b)
    ))

widen.legalize(
    a << load.i16(flags, ptr, offset),
    Rtl(
        b << insts.uload16.i32(flags, ptr, offset),
        a << ireduce(b)
    ))

for int_ty in [types.i8, types.i16]:
    widen.legalize(
        br_table.bind(int_ty)(x, y, z),
        Rtl(
            b << uextend.i32(x),
            br_table(b, y, z),
        )
    )

for int_ty in [types.i8, types.i16]:
    widen.legalize(
        a << insts.bint.bind(int_ty)(b),
        Rtl(
            x << insts.bint.i32(b),
            a << ireduce.bind(int_ty)(x)
        )
    )

for int_ty in [types.i8, types.i16]:
    for op in [ushr_imm, ishl_imm]:
        widen.legalize(
            a << op.bind(int_ty)(b, c),
            Rtl(
                x << uextend.i32(b),
                z << op.i32(x, c),
                a << ireduce.bind(int_ty)(z)
            ))

    widen.legalize(
        a << ishl.bind(int_ty)(b, c),
        Rtl(
            x << uextend.i32(b),
            z << ishl.i32(x, c),
            a << ireduce.bind(int_ty)(z)
        ))

    widen.legalize(
        a << ushr.bind(int_ty)(b, c),
        Rtl(
            x << uextend.i32(b),
            z << ushr.i32(x, c),
            a << ireduce.bind(int_ty)(z)
        ))

    widen.legalize(
        a << sshr.bind(int_ty)(b, c),
        Rtl(
            x << sextend.i32(b),
            z << sshr.i32(x, c),
            a << ireduce.bind(int_ty)(z)
        ))

    for w_cc in [
        intcc.eq, intcc.ne, intcc.ugt, intcc.ult, intcc.uge, intcc.ule
    ]:
        widen.legalize(
            a << insts.icmp_imm.bind(int_ty)(w_cc, b, c),
            Rtl(
                x << uextend.i32(b),
                a << insts.icmp_imm(w_cc, x, c)
            ))
        widen.legalize(
            a << insts.icmp.bind(int_ty)(w_cc, b, c),
            Rtl(
                x << uextend.i32(b),
                y << uextend.i32(c),
                a << insts.icmp.i32(w_cc, x, y)
            ))
    for w_cc in [intcc.sgt, intcc.slt, intcc.sge, intcc.sle]:
        widen.legalize(
            a << insts.icmp_imm.bind(int_ty)(w_cc, b, c),
            Rtl(
                x << sextend.i32(b),
                a << insts.icmp_imm(w_cc, x, c)
            ))
        widen.legalize(
            a << insts.icmp.bind(int_ty)(w_cc, b, c),
            Rtl(
                x << sextend.i32(b),
                y << sextend.i32(c),
                a << insts.icmp(w_cc, x, y)
            )
        )

# Expand integer operations with carry for RISC architectures that don't have
# the flags.
expand.legalize(
        (a, c) << iadd_cout(x, y),
        Rtl(
            a << iadd(x, y),
            c << icmp(intcc.ult, a, x)
        ))

expand.legalize(
        (a, b) << isub_bout(x, y),
        Rtl(
            a << isub(x, y),
            b << icmp(intcc.ugt, a, x)
        ))

expand.legalize(
        a << iadd_cin(x, y, c),
        Rtl(
            a1 << iadd(x, y),
            c_int << bint(c),
            a << iadd(a1, c_int)
        ))

expand.legalize(
        a << isub_bin(x, y, b),
        Rtl(
            a1 << isub(x, y),
            b_int << bint(b),
            a << isub(a1, b_int)
        ))

expand.legalize(
        (a, c) << iadd_carry(x, y, c_in),
        Rtl(
            (a1, c1) << iadd_cout(x, y),
            c_int << bint(c_in),
            (a, c2) << iadd_cout(a1, c_int),
            c << bor(c1, c2)
        ))

expand.legalize(
        (a, b) << isub_borrow(x, y, b_in),
        Rtl(
            (a1, b1) << isub_bout(x, y),
            b_int << bint(b_in),
            (a, b2) << isub_bout(a1, b_int),
            b << bor(b1, b2)
        ))

# Expansions for immediate operands that are out of range.
for inst_imm,      inst in [
        (iadd_imm, iadd),
        (imul_imm, imul),
        (sdiv_imm, sdiv),
        (udiv_imm, udiv),
        (srem_imm, srem),
        (urem_imm, urem),
        (band_imm, band),
        (bor_imm, bor),
        (bxor_imm, bor),
        (ifcmp_imm, ifcmp)]:
    expand.legalize(
            a << inst_imm(x, y),
            Rtl(
                a1 << iconst(y),
                a << inst(x, a1)
            ))
expand.legalize(
    a << irsub_imm(y, x),
    Rtl(
        a1 << iconst(x),
        a << isub(a1, y)
    ))

# Rotates and shifts.
for inst_imm,      inst in [
        (rotl_imm, rotl),
        (rotr_imm, rotr),
        (ishl_imm, ishl),
        (sshr_imm, sshr),
        (ushr_imm, ushr)]:
    expand.legalize(
            a << inst_imm(x, y),
            Rtl(
                a1 << iconst.i32(y),
                a << inst(x, a1)
            ))

expand.legalize(
        a << icmp_imm(cc, x, y),
        Rtl(
            a1 << iconst(y),
            a << icmp(cc, x, a1)
        ))

# Expansions for *_not variants of bitwise ops.
for inst_not,      inst in [
        (band_not, band),
        (bor_not,  bor),
        (bxor_not, bxor)]:
    expand.legalize(
            a << inst_not(x, y),
            Rtl(
                a1 << bnot(y),
                a << inst(x, a1)
            ))

# Expand bnot using xor.
expand.legalize(
        a << bnot(x),
        Rtl(
            y << iconst(imm64(-1)),
            a << bxor(x, y)
        ))

# Expand bitrev
# Adapted from Stack Overflow.
# https://stackoverflow.com/questions/746171/most-efficient-algorithm-for-bit-reversal-from-msb-lsb-to-lsb-msb-in-c
widen.legalize(
        a << bitrev.i8(x),
        Rtl(
            a1 << band_imm(x, imm64(0xaa)),
            a2 << ushr_imm(a1, imm64(1)),
            a3 << band_imm(x, imm64(0x55)),
            a4 << ishl_imm(a3, imm64(1)),
            b << bor(a2, a4),
            b1 << band_imm(b, imm64(0xcc)),
            b2 << ushr_imm(b1, imm64(2)),
            b3 << band_imm(b, imm64(0x33)),
            b4 << ushr_imm(b3, imm64(2)),
            c << bor(b2, b4),
            c1 << band_imm(c, imm64(0xf0)),
            c2 << ushr_imm(c1, imm64(4)),
            c3 << band_imm(c, imm64(0x0f)),
            c4 << ishl_imm(c3, imm64(4)),
            a << bor(c2, c4),
        ))

widen.legalize(
        a << bitrev.i16(x),
        Rtl(
            a1 << band_imm(x, imm64(0xaaaa)),
            a2 << ushr_imm(a1, imm64(1)),
            a3 << band_imm(x, imm64(0x5555)),
            a4 << ishl_imm(a3, imm64(1)),
            b << bor(a2, a4),
            b1 << band_imm(b, imm64(0xcccc)),
            b2 << ushr_imm(b1, imm64(2)),
            b3 << band_imm(b, imm64(0x3333)),
            b4 << ushr_imm(b3, imm64(2)),
            c << bor(b2, b4),
            c1 << band_imm(c, imm64(0xf0f0)),
            c2 << ushr_imm(c1, imm64(4)),
            c3 << band_imm(c, imm64(0x0f0f)),
            c4 << ishl_imm(c3, imm64(4)),
            d << bor(c2, c4),
            d1 << band_imm(d, imm64(0xff00)),
            d2 << ushr_imm(d1, imm64(8)),
            d3 << band_imm(d, imm64(0x00ff)),
            d4 << ishl_imm(d3, imm64(8)),
            a << bor(d2, d4),
        ))

expand.legalize(
        a << bitrev.i32(x),
        Rtl(
            a1 << band_imm(x, imm64(0xaaaaaaaa)),
            a2 << ushr_imm(a1, imm64(1)),
            a3 << band_imm(x, imm64(0x55555555)),
            a4 << ishl_imm(a3, imm64(1)),
            b << bor(a2, a4),
            b1 << band_imm(b, imm64(0xcccccccc)),
            b2 << ushr_imm(b1, imm64(2)),
            b3 << band_imm(b, imm64(0x33333333)),
            b4 << ushr_imm(b3, imm64(2)),
            c << bor(b2, b4),
            c1 << band_imm(c, imm64(0xf0f0f0f0)),
            c2 << ushr_imm(c1, imm64(4)),
            c3 << band_imm(c, imm64(0x0f0f0f0f)),
            c4 << ishl_imm(c3, imm64(4)),
            d << bor(c2, c4),
            d1 << band_imm(d, imm64(0xff00ff00)),
            d2 << ushr_imm(d1, imm64(8)),
            d3 << band_imm(d, imm64(0x00ff00ff)),
            d4 << ishl_imm(d3, imm64(8)),
            e << bor(d2, d4),
            e1 << ushr_imm(e, imm64(16)),
            e2 << ishl_imm(e, imm64(16)),
            a << bor(e1, e2),
        ))

expand.legalize(
        a << bitrev.i64(x),
        Rtl(
            a1 << band_imm(x, imm64(0xaaaaaaaaaaaaaaaa)),
            a2 << ushr_imm(a1, imm64(1)),
            a3 << band_imm(x, imm64(0x5555555555555555)),
            a4 << ishl_imm(a3, imm64(1)),
            b << bor(a2, a4),
            b1 << band_imm(b, imm64(0xcccccccccccccccc)),
            b2 << ushr_imm(b1, imm64(2)),
            b3 << band_imm(b, imm64(0x3333333333333333)),
            b4 << ushr_imm(b3, imm64(2)),
            c << bor(b2, b4),
            c1 << band_imm(c, imm64(0xf0f0f0f0f0f0f0f0)),
            c2 << ushr_imm(c1, imm64(4)),
            c3 << band_imm(c, imm64(0x0f0f0f0f0f0f0f0f)),
            c4 << ishl_imm(c3, imm64(4)),
            d << bor(c2, c4),
            d1 << band_imm(d, imm64(0xff00ff00ff00ff00)),
            d2 << ushr_imm(d1, imm64(8)),
            d3 << band_imm(d, imm64(0x00ff00ff00ff00ff)),
            d4 << ishl_imm(d3, imm64(8)),
            e << bor(d2, d4),
            e1 << band_imm(e, imm64(0xffff0000ffff0000)),
            e2 << ushr_imm(e1, imm64(16)),
            e3 << band_imm(e, imm64(0x0000ffff0000ffff)),
            e4 << ishl_imm(e3, imm64(16)),
            f << bor(e2, e4),
            f1 << ushr_imm(f, imm64(32)),
            f2 << ishl_imm(f, imm64(32)),
            a << bor(f1, f2),
        ))

# Floating-point sign manipulations.
for ty,             minus_zero in [
        (types.f32, f32const(ieee32.bits(0x80000000))),
        (types.f64, f64const(ieee64.bits(0x8000000000000000)))]:
    expand.legalize(
            a << insts.fabs.bind(ty)(x),
            Rtl(
                b << minus_zero,
                a << band_not(x, b),
            ))
    expand.legalize(
            a << insts.fneg.bind(ty)(x),
            Rtl(
                b << minus_zero,
                a << bxor(x, b),
            ))
    expand.legalize(
            a << insts.fcopysign.bind(ty)(x, y),
            Rtl(
                b << minus_zero,
                a1 << band_not(x, b),
                a2 << band(y, b),
                a << bor(a1, a2)
            ))

expand.custom_legalize(insts.br_icmp, 'expand_br_icmp')

# Expansions using CPU flags.

expand_flags.legalize(
    insts.trapnz(x, c),
    Rtl(
        a << insts.ifcmp_imm(x, imm64(0)),
        insts.trapif(intcc.ne, a, c)
    ))
expand_flags.legalize(
    insts.trapz(x, c),
    Rtl(
        a << insts.ifcmp_imm(x, imm64(0)),
        insts.trapif(intcc.eq, a, c)
    ))

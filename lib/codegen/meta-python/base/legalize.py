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
from cdsl.ast import Var
from cdsl.xform import Rtl, XFormGroup


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
a = Var('a')
a1 = Var('a1')
a2 = Var('a2')
b = Var('b')
b1 = Var('b1')
b2 = Var('b2')
b_in = Var('b_in')
b_int = Var('b_int')
c = Var('c')
c1 = Var('c1')
c2 = Var('c2')
c_in = Var('c_in')
c_int = Var('c_int')
d = Var('d')
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

for int_ty in [types.i8, types.i16]:
    widen.legalize(
        a << iconst.bind(int_ty)(b),
        Rtl(
            c << iconst.i32(b),
            a << ireduce.bind(int_ty)(c)
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

for binop in [iadd, isub, imul, udiv, band, bor, bxor]:
    for int_ty in [types.i8, types.i16]:
        widen.legalize(
            a << binop.bind(int_ty)(x, y),
            Rtl(
                b << uextend.i32(x),
                c << uextend.i32(y),
                d << binop(b, c),
                a << ireduce(d)
            )
        )

for binop in [sdiv]:
    for int_ty in [types.i8, types.i16]:
        widen.legalize(
            a << binop.bind(int_ty)(x, y),
            Rtl(
                b << sextend.i32(x),
                c << sextend.i32(y),
                d << binop(b, c),
                a << ireduce(d)
            )
        )

for unop in [bnot]:
    for int_ty in [types.i8, types.i16]:
        widen.legalize(
            a << unop.bind(int_ty)(x),
            Rtl(
                b << sextend.i32(x),
                d << unop(b),
                a << ireduce(d)
            )
        )

for binop in [iadd_imm, imul_imm, udiv_imm]:
    for int_ty in [types.i8, types.i16]:
        widen.legalize(
            a << binop.bind(int_ty)(x, y),
            Rtl(
                b << uextend.i32(x),
                c << binop(b, y),
                a << ireduce(c)
            )
        )

for binop in [sdiv_imm]:
    for int_ty in [types.i8, types.i16]:
        widen.legalize(
            a << binop.bind(int_ty)(x, y),
            Rtl(
                b << sextend.i32(x),
                c << binop(b, y),
                a << ireduce(c)
            )
        )

for int_ty in [types.i8, types.i16]:
    widen.legalize(
        br_table.bind(int_ty)(x, y),
        Rtl(
            b << uextend.i32(x),
            br_table(b, y),
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

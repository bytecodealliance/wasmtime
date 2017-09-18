"""
Patterns for legalizing the `base` instruction set.

The base Cretonne instruction set is 'fat', and many instructions don't have
legal representations in a given target ISA. This module defines legalization
patterns that describe how base instructions can be transformed to other base
instructions that are legal.
"""
from __future__ import absolute_import
from .immediates import intcc
from . import instructions as insts
from .instructions import iadd, iadd_cout, iadd_cin, iadd_carry, iadd_imm
from .instructions import isub, isub_bin, isub_bout, isub_borrow
from .instructions import imul, imul_imm
from .instructions import band, bor, bxor, isplit, iconcat
from .instructions import bnot, band_not, bor_not, bxor_not
from .instructions import icmp, icmp_imm
from .instructions import iconst, bint
from .instructions import ishl, ishl_imm, sshr, sshr_imm, ushr, ushr_imm
from .instructions import rotl, rotl_imm, rotr, rotr_imm
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

# Custom expansions for memory objects.
expand.custom_legalize(insts.global_addr, 'expand_global_addr')
expand.custom_legalize(insts.heap_addr, 'expand_heap_addr')

# Custom expansions that need to change the CFG.
# TODO: Add sufficient XForm syntax that we don't need to hand-code these.
expand.custom_legalize(insts.trapz, 'expand_cond_trap')
expand.custom_legalize(insts.trapnz, 'expand_cond_trap')

# Custom expansions for floating point constants.
# These expansions require bit-casting or creating constant pool entries.
expand.custom_legalize(insts.f32const, 'expand_fconst')
expand.custom_legalize(insts.f64const, 'expand_fconst')

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
xl = Var('xl')
xh = Var('xh')
yl = Var('yl')
yh = Var('yh')
al = Var('al')
ah = Var('ah')
cc = Var('cc')

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
        (imul_imm, imul)]:
    expand.legalize(
            a << inst_imm(x, y),
            Rtl(
                a1 << iconst(y),
                a << inst(x, a1)
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

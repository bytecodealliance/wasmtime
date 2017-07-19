"""
Patterns for legalizing the `base` instruction set.

The base Cretonne instruction set is 'fat', and many instructions don't have
legal representations in a given target ISA. This module defines legalization
patterns that describe how base instructions can be transformed to other base
instructions that are legal.
"""
from __future__ import absolute_import
from .immediates import intcc
from .instructions import iadd, iadd_cout, iadd_cin, iadd_carry, iadd_imm
from .instructions import isub, isub_bin, isub_bout, isub_borrow
from .instructions import band, bor, bxor, isplit, iconcat
from .instructions import icmp, icmp_imm
from .instructions import iconst, bint
from cdsl.ast import Var
from cdsl.xform import Rtl, XFormGroup


narrow = XFormGroup('narrow', """
        Legalize instructions by narrowing.

        The transformations in the 'narrow' group work by expressing
        instructions in terms of smaller types. Operations on vector types are
        expressed in terms of vector types with fewer lanes, and integer
        operations are expressed in terms of smaller integer types.
        """)

expand = XFormGroup('expand', """
        Legalize instructions by expansion.

        Rewrite instructions in terms of other instructions, generally
        operating on the same types as the original instructions.
        """)

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
expand.legalize(
        a << iadd_imm(x, y),
        Rtl(
            a1 << iconst(y),
            a << iadd(x, a1)
        ))

expand.legalize(
        a << icmp_imm(cc, x, y),
        Rtl(
            a1 << iconst(y),
            a << icmp(cc, x, a1)
        ))

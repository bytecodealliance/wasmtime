"""
Cranelift primitive instruction set.

This module defines a primitive instruction set, in terms of which the base set
is described. Most instructions in this set correspond 1-1 with an SMTLIB
bitvector function.
"""
from __future__ import absolute_import
from cdsl.operands import Operand
from cdsl.typevar import TypeVar
from cdsl.instructions import Instruction, InstructionGroup
from cdsl.ti import WiderOrEq
from base.types import b1
from base.immediates import imm64
import base.formats # noqa

GROUP = InstructionGroup("primitive", "Primitive instruction set")

BV = TypeVar('BV', 'A bitvector type.', bitvecs=True)
BV1 = TypeVar('BV1', 'A single bit bitvector.', bitvecs=(1, 1))
Real = TypeVar('Real', 'Any real type.', ints=True, floats=True,
               bools=True, simd=True)

x = Operand('x', BV, doc="A semantic value X")
y = Operand('x', BV, doc="A semantic value Y (same width as X)")
a = Operand('a', BV, doc="A semantic value A (same width as X)")
cond = Operand('b', TypeVar.singleton(b1), doc='A b1 value')

real = Operand('real', Real, doc="A real cranelift value")
fromReal = Operand('fromReal', Real.to_bitvec(),
                   doc="A real cranelift value converted to a BV")

#
# BV Conversion/Materialization
#
prim_to_bv = Instruction(
        'prim_to_bv', r"""
        Convert an SSA Value to a flat bitvector
        """,
        ins=(real), outs=(fromReal))

prim_from_bv = Instruction(
        'prim_from_bv', r"""
        Convert a flat bitvector to a real SSA Value.
        """,
        ins=(fromReal), outs=(real))

N = Operand('N', imm64)
bv_from_imm64 = Instruction(
        'bv_from_imm64', r"""Materialize an imm64 as a bitvector.""",
        ins=(N), outs=a)

#
# Generics
#
bvite = Instruction(
        'bvite', r"""Bitvector ternary operator""",
        ins=(cond, x, y), outs=a)


xh = Operand('xh', BV.half_width(),
             doc="A semantic value representing the upper half of X")
xl = Operand('xl', BV.half_width(),
             doc="A semantic value representing the lower half of X")
bvsplit = Instruction(
        'bvsplit', r"""
        """,
        ins=(x), outs=(xh, xl))

xy = Operand('xy', BV.double_width(),
             doc="A semantic value representing the concatenation of X and Y")
bvconcat = Instruction(
        'bvconcat', r"""
        """,
        ins=(x, y), outs=xy)

bvadd = Instruction(
        'bvadd', r"""
        Standard 2's complement addition. Equivalent to wrapping integer
        addition: :math:`a := x + y \pmod{2^B}`.

        This instruction does not depend on the signed/unsigned interpretation
        of the operands.
        """,
        ins=(x, y), outs=a)
#
# Bitvector comparisons
#

bveq = Instruction(
        'bveq', r"""Unsigned bitvector equality""",
        ins=(x, y), outs=cond)
bvne = Instruction(
        'bveq', r"""Unsigned bitvector inequality""",
        ins=(x, y), outs=cond)
bvsge = Instruction(
        'bvsge', r"""Signed bitvector greater or equal""",
        ins=(x, y), outs=cond)
bvsgt = Instruction(
        'bvsgt', r"""Signed bitvector greater than""",
        ins=(x, y), outs=cond)
bvsle = Instruction(
        'bvsle', r"""Signed bitvector less than or equal""",
        ins=(x, y), outs=cond)
bvslt = Instruction(
        'bvslt', r"""Signed bitvector less than""",
        ins=(x, y), outs=cond)
bvuge = Instruction(
        'bvuge', r"""Unsigned bitvector greater or equal""",
        ins=(x, y), outs=cond)
bvugt = Instruction(
        'bvugt', r"""Unsigned bitvector greater than""",
        ins=(x, y), outs=cond)
bvule = Instruction(
        'bvule', r"""Unsigned bitvector less than or equal""",
        ins=(x, y), outs=cond)
bvult = Instruction(
        'bvult', r"""Unsigned bitvector less than""",
        ins=(x, y), outs=cond)

# Extensions
ToBV = TypeVar('ToBV', 'A bitvector type.', bitvecs=True)
x1 = Operand('x1', ToBV, doc="")

bvzeroext = Instruction(
        'bvzeroext', r"""Unsigned bitvector extension""",
        ins=x, outs=x1, constraints=WiderOrEq(ToBV, BV))

bvsignext = Instruction(
        'bvsignext', r"""Signed bitvector extension""",
        ins=x, outs=x1, constraints=WiderOrEq(ToBV, BV))

GROUP.close()

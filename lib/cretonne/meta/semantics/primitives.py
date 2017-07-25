"""
Cretonne primitive instruction set.

This module defines a primitive instruction set, in terms of which the base set
is described. Most instructions in this set correspond 1-1 with an SMTLIB
bitvector function.
"""
from __future__ import absolute_import
from cdsl.operands import Operand
from cdsl.typevar import TypeVar
from cdsl.instructions import Instruction, InstructionGroup
import base.formats # noqa

GROUP = InstructionGroup("primitive", "Primitive instruction set")

BV = TypeVar('BV', 'A bitvector type.', bitvecs=True)
Real = TypeVar('Real', 'Any real type.', ints=True, floats=True,
               bools=True, simd=True)

x = Operand('x', BV, doc="A semantic value X")
y = Operand('x', BV, doc="A semantic value Y (same width as X)")
a = Operand('a', BV, doc="A semantic value A (same width as X)")

real = Operand('real', Real, doc="A real cretonne value")
fromReal = Operand('fromReal', Real.to_bitvec(),
                   doc="A real cretonne value converted to a BV")

prim_to_bv = Instruction(
        'prim_to_bv', r"""
        Convert an SSA Value to a flat bitvector
        """,
        ins=(real), outs=(fromReal))

# Note that when converting from BV->real values, we use a constraint and not a
# derived function. This reflects that fact that to_bitvec() is not a
# bijection.
prim_from_bv = Instruction(
        'prim_from_bv', r"""
        Convert a flat bitvector to a real SSA Value.
        """,
        ins=(fromReal), outs=(real))

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

GROUP.close()

"""
Useful semantics "macro" instructions built on top of
the primitives.
"""
from __future__ import absolute_import
from cdsl.operands import Operand
from cdsl.typevar import TypeVar
from cdsl.instructions import Instruction, InstructionGroup
from base.types import b1
from base.immediates import imm64
from cdsl.ast import Var
from cdsl.xform import Rtl
from semantics.primitives import bv_from_imm64, bvite
import base.formats # noqa

GROUP = InstructionGroup("primitive_macros", "Semantic macros instruction set")
AnyBV = TypeVar('AnyBV', bitvecs=True, doc="")
x = Var('x')
y = Var('y')
imm = Var('imm')
a = Var('a')

#
# Bool-to-bv1
#
BV1 = TypeVar("BV1", bitvecs=(1, 1), doc="")
bv1_op = Operand('bv1_op', BV1, doc="")
cond_op = Operand("cond", b1, doc="")
bool2bv = Instruction(
        'bool2bv', r"""Convert a b1 value to a 1-bit BV""",
        ins=cond_op, outs=bv1_op)

v1 = Var('v1')
v2 = Var('v2')
bvone = Var('bvone')
bvzero = Var('bvzero')
bool2bv.set_semantics(
        v1 << bool2bv(v2),
        Rtl(
            bvone << bv_from_imm64(imm64(1)),
            bvzero << bv_from_imm64(imm64(0)),
            v1 << bvite(v2, bvone, bvzero)
        ))

GROUP.close()

"""
Custom legalization patterns for Intel.
"""
from __future__ import absolute_import
from cdsl.ast import Var
from cdsl.xform import Rtl, XFormGroup
from base.immediates import imm64
from base.types import i32, i64
from base import legalize as shared
from base import instructions as insts
from . import instructions as x86
from .defs import ISA

intel_expand = XFormGroup(
        'intel_expand',
        """
        Legalize instructions by expansion.

        Use Intel-specific instructions if needed.
        """,
        isa=ISA, chain=shared.expand)

a = Var('a')
dead = Var('dead')
x = Var('x')
xhi = Var('xhi')
y = Var('y')

#
# Division and remainder.
#
intel_expand.legalize(
        a << insts.udiv(x, y),
        Rtl(
            xhi << insts.iconst(imm64(0)),
            (a, dead) << x86.udivmodx(x, xhi, y)
        ))

intel_expand.legalize(
        a << insts.urem(x, y),
        Rtl(
            xhi << insts.iconst(imm64(0)),
            (dead, a) << x86.udivmodx(x, xhi, y)
        ))

for ty in [i32, i64]:
    intel_expand.legalize(
            a << insts.sdiv.bind(ty)(x, y),
            Rtl(
                xhi << insts.sshr_imm(x, imm64(ty.lane_bits() - 1)),
                (a, dead) << x86.sdivmodx(x, xhi, y)
            ))
    intel_expand.legalize(
            a << insts.srem.bind(ty)(x, y),
            Rtl(
                xhi << insts.sshr_imm(x, imm64(ty.lane_bits() - 1)),
                (dead, a) << x86.sdivmodx(x, xhi, y)
            ))

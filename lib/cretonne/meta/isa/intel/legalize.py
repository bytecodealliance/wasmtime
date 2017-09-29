"""
Custom legalization patterns for Intel.
"""
from __future__ import absolute_import
from cdsl.ast import Var
from cdsl.xform import Rtl, XFormGroup
from base.immediates import imm64, floatcc
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
a1 = Var('a1')
a2 = Var('a2')

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

# The srem expansion requires custom code because srem INT_MIN, -1 is not
# allowed to trap.
intel_expand.custom_legalize(insts.srem, 'expand_srem')

# Floating point condition codes.
#
# The 8 condition codes in `supported_floatccs` are directly supported by a
# `ucomiss` or `ucomisd` instruction. The remaining codes need legalization
# patterns.

# Equality needs an explicit `ord` test which checks the parity bit.
intel_expand.legalize(
        a << insts.fcmp(floatcc.eq, x, y),
        Rtl(
            a1 << insts.fcmp(floatcc.ord, x, y),
            a2 << insts.fcmp(floatcc.ueq, x, y),
            a << insts.band(a1, a2)
        ))
intel_expand.legalize(
        a << insts.fcmp(floatcc.ne, x, y),
        Rtl(
            a1 << insts.fcmp(floatcc.uno, x, y),
            a2 << insts.fcmp(floatcc.one, x, y),
            a << insts.bor(a1, a2)
        ))

# Inequalities that need to be reversed.
for cc,               rev_cc in [
        (floatcc.lt,  floatcc.gt),
        (floatcc.le,  floatcc.ge),
        (floatcc.ugt, floatcc.ult),
        (floatcc.uge, floatcc.ule)]:
    intel_expand.legalize(
            a << insts.fcmp(cc, x, y),
            Rtl(
                a << insts.fcmp(rev_cc, y, x)
            ))

# We need to modify the CFG for min/max legalization.
intel_expand.custom_legalize(insts.fmin, 'expand_minmax')
intel_expand.custom_legalize(insts.fmax, 'expand_minmax')

# Conversions from unsigned need special handling.
intel_expand.custom_legalize(insts.fcvt_from_uint, 'expand_fcvt_from_uint')
# Conversions from float to int can trap.
intel_expand.custom_legalize(insts.fcvt_to_sint, 'expand_fcvt_to_sint')
intel_expand.custom_legalize(insts.fcvt_to_uint, 'expand_fcvt_to_uint')

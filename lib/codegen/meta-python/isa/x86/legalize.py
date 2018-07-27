"""
Custom legalization patterns for x86.
"""
from __future__ import absolute_import
from cdsl.ast import Var
from cdsl.xform import Rtl, XFormGroup
from base.immediates import imm64, intcc, floatcc
from base import legalize as shared
from base import instructions as insts
from . import instructions as x86
from .defs import ISA

x86_expand = XFormGroup(
        'x86_expand',
        """
        Legalize instructions by expansion.

        Use x86-specific instructions if needed.
        """,
        isa=ISA, chain=shared.expand_flags)

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
# The srem expansion requires custom code because srem INT_MIN, -1 is not
# allowed to trap. The other ops need to check avoid_div_traps.
x86_expand.custom_legalize(insts.sdiv, 'expand_sdivrem')
x86_expand.custom_legalize(insts.srem, 'expand_sdivrem')
x86_expand.custom_legalize(insts.udiv, 'expand_udivrem')
x86_expand.custom_legalize(insts.urem, 'expand_udivrem')

#
# Double length (widening) multiplication
#
resLo = Var('resLo')
resHi = Var('resHi')
x86_expand.legalize(
        resHi << insts.umulhi(x, y),
        Rtl(
            (resLo, resHi) << x86.umulx(x, y)
        ))

x86_expand.legalize(
        resHi << insts.smulhi(x, y),
        Rtl(
            (resLo, resHi) << x86.smulx(x, y)
        ))

# Floating point condition codes.
#
# The 8 condition codes in `supported_floatccs` are directly supported by a
# `ucomiss` or `ucomisd` instruction. The remaining codes need legalization
# patterns.

# Equality needs an explicit `ord` test which checks the parity bit.
x86_expand.legalize(
        a << insts.fcmp(floatcc.eq, x, y),
        Rtl(
            a1 << insts.fcmp(floatcc.ord, x, y),
            a2 << insts.fcmp(floatcc.ueq, x, y),
            a << insts.band(a1, a2)
        ))
x86_expand.legalize(
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
    x86_expand.legalize(
            a << insts.fcmp(cc, x, y),
            Rtl(
                a << insts.fcmp(rev_cc, y, x)
            ))

# We need to modify the CFG for min/max legalization.
x86_expand.custom_legalize(insts.fmin, 'expand_minmax')
x86_expand.custom_legalize(insts.fmax, 'expand_minmax')

# Conversions from unsigned need special handling.
x86_expand.custom_legalize(insts.fcvt_from_uint, 'expand_fcvt_from_uint')
# Conversions from float to int can trap and modify the control flow graph.
x86_expand.custom_legalize(insts.fcvt_to_sint, 'expand_fcvt_to_sint')
x86_expand.custom_legalize(insts.fcvt_to_uint, 'expand_fcvt_to_uint')
x86_expand.custom_legalize(insts.fcvt_to_sint_sat, 'expand_fcvt_to_sint_sat')
x86_expand.custom_legalize(insts.fcvt_to_uint_sat, 'expand_fcvt_to_uint_sat')

# Count leading and trailing zeroes, for baseline x86_64
c_minus_one = Var('c_minus_one')
c_thirty_one = Var('c_thirty_one')
c_thirty_two = Var('c_thirty_two')
c_sixty_three = Var('c_sixty_three')
c_sixty_four = Var('c_sixty_four')
index1 = Var('index1')
r2flags = Var('r2flags')
index2 = Var('index2')

x86_expand.legalize(
    a << insts.clz.i64(x),
    Rtl(
        c_minus_one << insts.iconst(imm64(-1)),
        c_sixty_three << insts.iconst(imm64(63)),
        (index1, r2flags) << x86.bsr(x),
        index2 << insts.selectif(intcc.eq, r2flags, c_minus_one, index1),
        a << insts.isub(c_sixty_three, index2),
    ))

x86_expand.legalize(
    a << insts.clz.i32(x),
    Rtl(
        c_minus_one << insts.iconst(imm64(-1)),
        c_thirty_one << insts.iconst(imm64(31)),
        (index1, r2flags) << x86.bsr(x),
        index2 << insts.selectif(intcc.eq, r2flags, c_minus_one, index1),
        a << insts.isub(c_thirty_one, index2),
    ))

x86_expand.legalize(
    a << insts.ctz.i64(x),
    Rtl(
        c_sixty_four << insts.iconst(imm64(64)),
        (index1, r2flags) << x86.bsf(x),
        a << insts.selectif(intcc.eq, r2flags, c_sixty_four, index1),
    ))

x86_expand.legalize(
    a << insts.ctz.i32(x),
    Rtl(
        c_thirty_two << insts.iconst(imm64(32)),
        (index1, r2flags) << x86.bsf(x),
        a << insts.selectif(intcc.eq, r2flags, c_thirty_two, index1),
    ))


# Population count for baseline x86_64
qv1 = Var('qv1')
qv3 = Var('qv3')
qv4 = Var('qv4')
qv5 = Var('qv5')
qv6 = Var('qv6')
qv7 = Var('qv7')
qv8 = Var('qv8')
qv9 = Var('qv9')
qv10 = Var('qv10')
qv11 = Var('qv11')
qv12 = Var('qv12')
qv13 = Var('qv13')
qv14 = Var('qv14')
qv15 = Var('qv15')
qv16 = Var('qv16')
qc77 = Var('qc77')
qc0F = Var('qc0F')
qc01 = Var('qc01')
x86_expand.legalize(
    qv16 << insts.popcnt.i64(qv1),
    Rtl(
        qv3 << insts.ushr_imm(qv1, imm64(1)),
        qc77 << insts.iconst(imm64(0x7777777777777777)),
        qv4 << insts.band(qv3, qc77),
        qv5 << insts.isub(qv1, qv4),
        qv6 << insts.ushr_imm(qv4, imm64(1)),
        qv7 << insts.band(qv6, qc77),
        qv8 << insts.isub(qv5, qv7),
        qv9 << insts.ushr_imm(qv7, imm64(1)),
        qv10 << insts.band(qv9, qc77),
        qv11 << insts.isub(qv8, qv10),
        qv12 << insts.ushr_imm(qv11, imm64(4)),
        qv13 << insts.iadd(qv11, qv12),
        qc0F << insts.iconst(imm64(0x0F0F0F0F0F0F0F0F)),
        qv14 << insts.band(qv13, qc0F),
        qc01 << insts.iconst(imm64(0x0101010101010101)),
        qv15 << insts.imul(qv14, qc01),
        qv16 << insts.ushr_imm(qv15, imm64(56))
    ))

lv1 = Var('lv1')
lv3 = Var('lv3')
lv4 = Var('lv4')
lv5 = Var('lv5')
lv6 = Var('lv6')
lv7 = Var('lv7')
lv8 = Var('lv8')
lv9 = Var('lv9')
lv10 = Var('lv10')
lv11 = Var('lv11')
lv12 = Var('lv12')
lv13 = Var('lv13')
lv14 = Var('lv14')
lv15 = Var('lv15')
lv16 = Var('lv16')
lc77 = Var('lc77')
lc0F = Var('lc0F')
lc01 = Var('lc01')
x86_expand.legalize(
    lv16 << insts.popcnt.i32(lv1),
    Rtl(
        lv3 << insts.ushr_imm(lv1, imm64(1)),
        lc77 << insts.iconst(imm64(0x77777777)),
        lv4 << insts.band(lv3, lc77),
        lv5 << insts.isub(lv1, lv4),
        lv6 << insts.ushr_imm(lv4, imm64(1)),
        lv7 << insts.band(lv6, lc77),
        lv8 << insts.isub(lv5, lv7),
        lv9 << insts.ushr_imm(lv7, imm64(1)),
        lv10 << insts.band(lv9, lc77),
        lv11 << insts.isub(lv8, lv10),
        lv12 << insts.ushr_imm(lv11, imm64(4)),
        lv13 << insts.iadd(lv11, lv12),
        lc0F << insts.iconst(imm64(0x0F0F0F0F)),
        lv14 << insts.band(lv13, lc0F),
        lc01 << insts.iconst(imm64(0x01010101)),
        lv15 << insts.imul(lv14, lc01),
        lv16 << insts.ushr_imm(lv15, imm64(24))
    ))

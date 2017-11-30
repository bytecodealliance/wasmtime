"""
Intel Encodings.
"""
from __future__ import absolute_import
from cdsl.predicates import IsUnsignedInt, Not
from base import instructions as base
from base.formats import UnaryImm
from .defs import I32, I64
from . import recipes as r
from . import settings as cfg
from . import instructions as x86
from .legalize import intel_expand
from base.legalize import narrow, expand
from base.settings import allones_funcaddrs
from .settings import use_sse41

try:
    from typing import TYPE_CHECKING, Any  # noqa
    if TYPE_CHECKING:
        from cdsl.instructions import MaybeBoundInst  # noqa
except ImportError:
    pass


I32.legalize_monomorphic(expand)
I32.legalize_type(
        default=narrow,
        b1=expand,
        i32=intel_expand,
        f32=intel_expand,
        f64=intel_expand)

I64.legalize_monomorphic(expand)
I64.legalize_type(
        default=narrow,
        b1=expand,
        i32=intel_expand,
        i64=intel_expand,
        f32=intel_expand,
        f64=intel_expand)


#
# Helper functions for generating encodings.
#

def enc_i64(inst, recipe, *args, **kwargs):
    # type: (MaybeBoundInst, r.TailRecipe, *int, **int) -> None
    """
    Add encodings for `inst` to I64 with and without a REX prefix.
    """
    I64.enc(inst, *recipe.rex(*args, **kwargs))
    I64.enc(inst, *recipe(*args, **kwargs))


def enc_both(inst, recipe, *args, **kwargs):
    # type: (MaybeBoundInst, r.TailRecipe, *int, **Any) -> None
    """
    Add encodings for `inst` to both I32 and I64.
    """
    I32.enc(inst, *recipe(*args, **kwargs))
    enc_i64(inst, recipe, *args, **kwargs)


def enc_i32_i64(inst, recipe, *args, **kwargs):
    # type: (MaybeBoundInst, r.TailRecipe, *int, **int) -> None
    """
    Add encodings for `inst.i32` to I32.
    Add encodings for `inst.i32` to I64 with and without REX.
    Add encodings for `inst.i64` to I64 with a REX.W prefix.
    """
    I32.enc(inst.i32, *recipe(*args, **kwargs))

    # REX-less encoding must come after REX encoding so we don't use it by
    # default. Otherwise reg-alloc would never use r8 and up.
    I64.enc(inst.i32, *recipe.rex(*args, **kwargs))
    I64.enc(inst.i32, *recipe(*args, **kwargs))

    I64.enc(inst.i64, *recipe.rex(*args, w=1, **kwargs))


def enc_i32_i64_ld_st(inst, w_bit, recipe, *args, **kwargs):
    # type: (MaybeBoundInst, bool, r.TailRecipe, *int, **int) -> None
    """
    Add encodings for `inst.i32` to I32.
    Add encodings for `inst.i32` to I64 with and without REX.
    Add encodings for `inst.i64` to I64 with a REX prefix, using the `w_bit`
    argument to determine whether or not to set the REX.W bit.
    """
    I32.enc(inst.i32.any, *recipe(*args, **kwargs))

    # REX-less encoding must come after REX encoding so we don't use it by
    # default. Otherwise reg-alloc would never use r8 and up.
    I64.enc(inst.i32.any, *recipe.rex(*args, **kwargs))
    I64.enc(inst.i32.any, *recipe(*args, **kwargs))

    if w_bit:
        I64.enc(inst.i64.any, *recipe.rex(*args, w=1, **kwargs))
    else:
        I64.enc(inst.i64.any, *recipe.rex(*args, **kwargs))
        I64.enc(inst.i64.any, *recipe(*args, **kwargs))


for inst,           opc in [
        (base.iadd, 0x01),
        (base.isub, 0x29),
        (base.band, 0x21),
        (base.bor,  0x09),
        (base.bxor, 0x31)]:
    enc_i32_i64(inst, r.rr, opc)

# Also add a `b1` encodings for the logic instructions.
# TODO: Should this be done with 8-bit instructions? It would improve
# partial register dependencies.
enc_both(base.band.b1, r.rr, 0x21)
enc_both(base.bor.b1,  r.rr, 0x09)
enc_both(base.bxor.b1, r.rr, 0x31)

enc_i32_i64(base.imul, r.rrx, 0x0f, 0xaf)
enc_i32_i64(x86.sdivmodx, r.div, 0xf7, rrr=7)
enc_i32_i64(x86.udivmodx, r.div, 0xf7, rrr=6)

enc_i32_i64(base.copy, r.umr, 0x89)
enc_both(base.copy.b1, r.umr, 0x89)
enc_i32_i64(base.regmove, r.rmov, 0x89)
enc_both(base.regmove.b1, r.rmov, 0x89)

# Immediate instructions with sign-extended 8-bit and 32-bit immediate.
for inst,               rrr in [
        (base.iadd_imm, 0),
        (base.band_imm, 4),
        (base.bor_imm,  1),
        (base.bxor_imm, 6)]:
    enc_i32_i64(inst, r.rib, 0x83, rrr=rrr)
    enc_i32_i64(inst, r.rid, 0x81, rrr=rrr)

# TODO: band_imm.i64 with an unsigned 32-bit immediate can be encoded as
# band_imm.i32. Can even use the single-byte immediate for 0xffff_ffXX masks.

# Immediate constants.
I32.enc(base.iconst.i32, *r.puid(0xb8))

I64.enc(base.iconst.i32, *r.puid.rex(0xb8))
I64.enc(base.iconst.i32, *r.puid(0xb8))
# The 32-bit immediate movl also zero-extends to 64 bits.
I64.enc(base.iconst.i64, *r.puid.rex(0xb8),
        instp=IsUnsignedInt(UnaryImm.imm, 32))
I64.enc(base.iconst.i64, *r.puid(0xb8),
        instp=IsUnsignedInt(UnaryImm.imm, 32))
# Sign-extended 32-bit immediate.
I64.enc(base.iconst.i64, *r.uid.rex(0xc7, rrr=0, w=1))
# Finally, the 0xb8 opcode takes an 8-byte immediate with a REX.W prefix.
I64.enc(base.iconst.i64, *r.puiq.rex(0xb8, w=1))

# Shifts and rotates.
# Note that the dynamic shift amount is only masked by 5 or 6 bits; the 8-bit
# and 16-bit shifts would need explicit masking.
for inst,           rrr in [
        (base.rotl, 0),
        (base.rotr, 1),
        (base.ishl, 4),
        (base.ushr, 5),
        (base.sshr, 7)]:
    I32.enc(inst.i32.any, *r.rc(0xd3, rrr=rrr))
    I64.enc(inst.i64.any, *r.rc.rex(0xd3, rrr=rrr, w=1))
    I64.enc(inst.i32.any, *r.rc.rex(0xd3, rrr=rrr))
    I64.enc(inst.i32.any, *r.rc(0xd3, rrr=rrr))

# Population count.
I32.enc(base.popcnt.i32, *r.urm(0xf3, 0x0f, 0xb8), isap=cfg.use_popcnt)
I64.enc(base.popcnt.i64, *r.urm.rex(0xf3, 0x0f, 0xb8, w=1),
        isap=cfg.use_popcnt)
I64.enc(base.popcnt.i32, *r.urm.rex(0xf3, 0x0f, 0xb8), isap=cfg.use_popcnt)
I64.enc(base.popcnt.i32, *r.urm(0xf3, 0x0f, 0xb8), isap=cfg.use_popcnt)

# Count leading zero bits.
I32.enc(base.clz.i32, *r.urm(0xf3, 0x0f, 0xbd), isap=cfg.use_lzcnt)
I64.enc(base.clz.i64, *r.urm.rex(0xf3, 0x0f, 0xbd, w=1),
        isap=cfg.use_lzcnt)
I64.enc(base.clz.i32, *r.urm.rex(0xf3, 0x0f, 0xbd), isap=cfg.use_lzcnt)
I64.enc(base.clz.i32, *r.urm(0xf3, 0x0f, 0xbd), isap=cfg.use_lzcnt)

# Count trailing zero bits.
I32.enc(base.ctz.i32, *r.urm(0xf3, 0x0f, 0xbc), isap=cfg.use_bmi1)
I64.enc(base.ctz.i64, *r.urm.rex(0xf3, 0x0f, 0xbc, w=1),
        isap=cfg.use_bmi1)
I64.enc(base.ctz.i32, *r.urm.rex(0xf3, 0x0f, 0xbc), isap=cfg.use_bmi1)
I64.enc(base.ctz.i32, *r.urm(0xf3, 0x0f, 0xbc), isap=cfg.use_bmi1)

#
# Loads and stores.
#
for recipe in [r.st, r.stDisp8, r.stDisp32]:
    enc_i32_i64_ld_st(base.store, True, recipe, 0x89)
    enc_i64(base.istore32.i64.any, recipe, 0x89)
    enc_i32_i64_ld_st(base.istore16, False, recipe, 0x66, 0x89)

# Byte stores are more complicated because the registers they can address
# depends of the presence of a REX prefix. The st*_abcd recipes fall back to
# the corresponding st* recipes when a REX prefix is applied.
for recipe in [r.st_abcd, r.stDisp8_abcd, r.stDisp32_abcd]:
    enc_both(base.istore8.i32.any, recipe, 0x88)
    enc_i64(base.istore8.i64.any, recipe, 0x88)

enc_i32_i64(base.spill, r.spSib32, 0x89)
enc_i32_i64(base.regspill, r.rsp32, 0x89)

# Use a 32-bit write for spilling `b1` to avoid constraining the permitted
# registers.
# See MIN_SPILL_SLOT_SIZE which makes this safe.
enc_both(base.spill.b1, r.spSib32, 0x89)
enc_both(base.regspill.b1, r.rsp32, 0x89)

for recipe in [r.ld, r.ldDisp8, r.ldDisp32]:
    enc_i32_i64_ld_st(base.load, True, recipe, 0x8b)
    enc_i64(base.uload32.i64, recipe, 0x8b)
    I64.enc(base.sload32.i64, *recipe.rex(0x63, w=1))
    enc_i32_i64_ld_st(base.uload16, True, recipe, 0x0f, 0xb7)
    enc_i32_i64_ld_st(base.sload16, True, recipe, 0x0f, 0xbf)
    enc_i32_i64_ld_st(base.uload8, True, recipe, 0x0f, 0xb6)
    enc_i32_i64_ld_st(base.sload8, True, recipe, 0x0f, 0xbe)

enc_i32_i64(base.fill, r.fiSib32, 0x8b)
enc_i32_i64(base.regfill, r.rfi32, 0x8b)

# Load 32 bits from `b1` spill slots. See `spill.b1` above.
enc_both(base.fill.b1, r.fiSib32, 0x8b)
enc_both(base.regfill.b1, r.rfi32, 0x8b)

# Push and Pop
enc_i64(x86.push.i64, r.pushq, 0x50)
enc_i64(x86.pop.i64, r.popq, 0x58)

# Copy Special
I64.enc(base.copy_special, *r.copysp.rex(0x89, w=1))

# Adjust SP Imm
I64.enc(base.adjust_sp_imm, *r.adjustsp.rex(0x81, w=1))

#
# Float loads and stores.
#

enc_both(base.load.f32.any, r.fld, 0x66, 0x0f, 0x6e)
enc_both(base.load.f32.any, r.fldDisp8, 0x66, 0x0f, 0x6e)
enc_both(base.load.f32.any, r.fldDisp32, 0x66, 0x0f, 0x6e)

enc_both(base.load.f64.any, r.fld, 0xf3, 0x0f, 0x7e)
enc_both(base.load.f64.any, r.fldDisp8, 0xf3, 0x0f, 0x7e)
enc_both(base.load.f64.any, r.fldDisp32, 0xf3, 0x0f, 0x7e)

enc_both(base.store.f32.any, r.fst, 0x66, 0x0f, 0x7e)
enc_both(base.store.f32.any, r.fstDisp8, 0x66, 0x0f, 0x7e)
enc_both(base.store.f32.any, r.fstDisp32, 0x66, 0x0f, 0x7e)

enc_both(base.store.f64.any, r.fst, 0x66, 0x0f, 0xd6)
enc_both(base.store.f64.any, r.fstDisp8, 0x66, 0x0f, 0xd6)
enc_both(base.store.f64.any, r.fstDisp32, 0x66, 0x0f, 0xd6)

enc_both(base.fill.f32, r.ffiSib32, 0x66, 0x0f, 0x6e)
enc_both(base.regfill.f32, r.frfi32, 0x66, 0x0f, 0x6e)
enc_both(base.fill.f64, r.ffiSib32, 0xf3, 0x0f, 0x7e)
enc_both(base.regfill.f64, r.frfi32, 0xf3, 0x0f, 0x7e)

enc_both(base.spill.f32, r.fspSib32, 0x66, 0x0f, 0x7e)
enc_both(base.regspill.f32, r.frsp32, 0x66, 0x0f, 0x7e)
enc_both(base.spill.f64, r.fspSib32, 0x66, 0x0f, 0xd6)
enc_both(base.regspill.f64, r.frsp32, 0x66, 0x0f, 0xd6)

#
# Function addresses.
#

I32.enc(base.func_addr.i32, *r.fnaddr4(0xb8),
        isap=Not(allones_funcaddrs))
I64.enc(base.func_addr.i64, *r.fnaddr8.rex(0xb8, w=1),
        isap=Not(allones_funcaddrs))

I32.enc(base.func_addr.i32, *r.allones_fnaddr4(0xb8),
        isap=allones_funcaddrs)
I64.enc(base.func_addr.i64, *r.allones_fnaddr8.rex(0xb8, w=1),
        isap=allones_funcaddrs)

#
# Global addresses.
#

I32.enc(base.globalsym_addr.i32, *r.gvaddr4(0xb8))
I64.enc(base.globalsym_addr.i64, *r.gvaddr8.rex(0xb8, w=1))

#
# Call/return
#
I32.enc(base.call, *r.call_id(0xe8))
I64.enc(base.call, *r.call_id(0xe8))

I32.enc(base.call_indirect.i32, *r.call_r(0xff, rrr=2))
I64.enc(base.call_indirect.i64, *r.call_r.rex(0xff, rrr=2))
I64.enc(base.call_indirect.i64, *r.call_r(0xff, rrr=2))

I32.enc(base.x_return, *r.ret(0xc3))
I64.enc(base.x_return, *r.ret(0xc3))

#
# Branches
#
enc_both(base.jump, r.jmpb, 0xeb)
enc_both(base.jump, r.jmpd, 0xe9)

enc_both(base.brif, r.brib, 0x70)
enc_both(base.brif, r.brid, 0x0f, 0x80)

# Not all float condition codes are legal, see `supported_floatccs`.
enc_both(base.brff, r.brfb, 0x70)
enc_both(base.brff, r.brfd, 0x0f, 0x80)

# Note that the tjccd opcode will be prefixed with 0x0f.
enc_i32_i64(base.brz, r.tjccb, 0x74)
enc_i32_i64(base.brz, r.tjccd, 0x84)
enc_i32_i64(base.brnz, r.tjccb, 0x75)
enc_i32_i64(base.brnz, r.tjccd, 0x85)

# Branch on a b1 value in a register only looks at the low 8 bits. See also
# bint encodings below.
#
# Start with the worst-case encoding for I32 only. The register allocator can't
# handle a branch with an ABCD-constrained operand.
I32.enc(base.brz.b1, *r.t8jccd_long(0x84))
I32.enc(base.brnz.b1, *r.t8jccd_long(0x85))

enc_both(base.brz.b1, r.t8jccb_abcd, 0x74)
enc_both(base.brz.b1, r.t8jccd_abcd, 0x84)
enc_both(base.brnz.b1, r.t8jccb_abcd, 0x75)
enc_both(base.brnz.b1, r.t8jccd_abcd, 0x85)

#
# Trap as ud2
#
I32.enc(base.trap, *r.trap(0x0f, 0x0b))
I64.enc(base.trap, *r.trap(0x0f, 0x0b))

#
# Comparisons
#
enc_i32_i64(base.icmp, r.icscc, 0x39)
enc_i32_i64(base.ifcmp, r.rcmp, 0x39)

#
# Convert flags to bool.
#
# This encodes `b1` as an 8-bit low register with the value 0 or 1.
enc_both(base.trueif, r.seti_abcd, 0x0f, 0x90)
enc_both(base.trueff, r.setf_abcd, 0x0f, 0x90)

#
# Convert bool to int.
#
# This assumes that b1 is represented as an 8-bit low register with the value 0
# or 1.
I32.enc(base.bint.i32.b1, *r.urm_abcd(0x0f, 0xb6))
I64.enc(base.bint.i64.b1, *r.urm.rex(0x0f, 0xb6))   # zext to i64 implicit.
I64.enc(base.bint.i64.b1, *r.urm_abcd(0x0f, 0xb6))  # zext to i64 implicit.
I64.enc(base.bint.i32.b1, *r.urm.rex(0x0f, 0xb6))
I64.enc(base.bint.i32.b1, *r.urm_abcd(0x0f, 0xb6))

# Numerical conversions.

# Converting i64 to i32 is a no-op in 64-bit mode.
I64.enc(base.ireduce.i32.i64, r.null, 0)
I64.enc(base.sextend.i64.i32, *r.urm.rex(0x63, w=1))
# A 32-bit register copy clears the high 32 bits.
I64.enc(base.uextend.i64.i32, *r.umr.rex(0x89))
I64.enc(base.uextend.i64.i32, *r.umr(0x89))


#
# Floating point
#

# movd
enc_both(base.bitcast.f32.i32, r.frurm, 0x66, 0x0f, 0x6e)
enc_both(base.bitcast.i32.f32, r.rfumr, 0x66, 0x0f, 0x7e)

# movq
I64.enc(base.bitcast.f64.i64, *r.frurm.rex(0x66, 0x0f, 0x6e, w=1))
I64.enc(base.bitcast.i64.f64, *r.rfumr.rex(0x66, 0x0f, 0x7e, w=1))

# movaps
enc_both(base.copy.f32, r.furm, 0x0f, 0x28)
enc_both(base.copy.f64, r.furm, 0x0f, 0x28)
enc_both(base.regmove.f32, r.frmov, 0x0f, 0x28)
enc_both(base.regmove.f64, r.frmov, 0x0f, 0x28)

# cvtsi2ss
enc_i32_i64(base.fcvt_from_sint.f32, r.frurm, 0xf3, 0x0f, 0x2a)

# cvtsi2sd
enc_i32_i64(base.fcvt_from_sint.f64, r.frurm, 0xf2, 0x0f, 0x2a)

# cvtss2sd
enc_both(base.fpromote.f64.f32, r.furm, 0xf3, 0x0f, 0x5a)

# cvtsd2ss
enc_both(base.fdemote.f32.f64, r.furm, 0xf2, 0x0f, 0x5a)

# cvttss2si
enc_both(x86.cvtt2si.i32.f32, r.rfurm, 0xf3, 0x0f, 0x2c)
I64.enc(x86.cvtt2si.i64.f32, *r.rfurm.rex(0xf3, 0x0f, 0x2c, w=1))

# cvttsd2si
enc_both(x86.cvtt2si.i32.f64, r.rfurm, 0xf2, 0x0f, 0x2c)
I64.enc(x86.cvtt2si.i64.f64, *r.rfurm.rex(0xf2, 0x0f, 0x2c, w=1))

# Exact square roots.
enc_both(base.sqrt.f32, r.furm, 0xf3, 0x0f, 0x51)
enc_both(base.sqrt.f64, r.furm, 0xf2, 0x0f, 0x51)

# Rounding. The recipe looks at the opcode to pick an immediate.
for inst in [
        base.nearest,
        base.floor,
        base.ceil,
        base.trunc]:
    enc_both(inst.f32, r.furmi_rnd, 0x66, 0x0f, 0x3a, 0x0a, isap=use_sse41)
    enc_both(inst.f64, r.furmi_rnd, 0x66, 0x0f, 0x3a, 0x0b, isap=use_sse41)


# Binary arithmetic ops.
for inst,           opc in [
        (base.fadd, 0x58),
        (base.fsub, 0x5c),
        (base.fmul, 0x59),
        (base.fdiv, 0x5e),
        (x86.fmin,  0x5d),
        (x86.fmax,  0x5f)]:
    enc_both(inst.f32, r.fa, 0xf3, 0x0f, opc)
    enc_both(inst.f64, r.fa, 0xf2, 0x0f, opc)

# Binary bitwise ops.
for inst,               opc in [
        (base.band,     0x54),
        (base.bor,      0x56),
        (base.bxor,     0x57)]:
    enc_both(inst.f32, r.fa, 0x0f, opc)
    enc_both(inst.f64, r.fa, 0x0f, opc)

# The `andnps(x,y)` instruction computes `~x&y`, while band_not(x,y)` is `x&~y.
enc_both(base.band_not.f32, r.fax, 0x0f, 0x55)
enc_both(base.band_not.f64, r.fax, 0x0f, 0x55)

# Comparisons.
#
# This only covers the condition codes in `supported_floatccs`, the rest are
# handled by legalization patterns.
enc_both(base.fcmp.f32, r.fcscc, 0x0f, 0x2e)
enc_both(base.fcmp.f64, r.fcscc, 0x66, 0x0f, 0x2e)

enc_both(base.ffcmp.f32, r.fcmp, 0x0f, 0x2e)
enc_both(base.ffcmp.f64, r.fcmp, 0x66, 0x0f, 0x2e)

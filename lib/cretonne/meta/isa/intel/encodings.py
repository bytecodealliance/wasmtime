"""
Intel Encodings.
"""
from __future__ import absolute_import
from cdsl.predicates import IsUnsignedInt
from base import instructions as base
from base.formats import UnaryImm
from .defs import I32, I64
from . import recipes as r
from . import settings as cfg
from . import instructions as x86
from .legalize import intel_expand
from base.legalize import narrow, expand

try:
    from typing import TYPE_CHECKING
    if TYPE_CHECKING:
        from cdsl.instructions import MaybeBoundInst  # noqa
except ImportError:
    pass


I32.legalize_monomorphic(expand)
I32.legalize_type(
        default=narrow,
        b1=expand,
        i32=intel_expand,
        f32=expand,
        f64=expand)

I64.legalize_monomorphic(expand)
I64.legalize_type(
        default=narrow,
        b1=expand,
        i32=intel_expand,
        i64=intel_expand,
        f32=expand,
        f64=expand)


#
# Helper functions for generating encodings.
#

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
    argument to determine wheter or not to set the REX.W bit.
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


def enc_flt(inst, recipe, *args, **kwargs):
    # type: (MaybeBoundInst, r.TailRecipe, *int, **int) -> None
    """
    Add encodings for floating point instruction `inst` to both I32 and I64.
    """
    I32.enc(inst, *recipe(*args, **kwargs))
    I64.enc(inst, *recipe.rex(*args, **kwargs))
    I64.enc(inst, *recipe(*args, **kwargs))


for inst,           opc in [
        (base.iadd, 0x01),
        (base.isub, 0x29),
        (base.band, 0x21),
        (base.bor,  0x09),
        (base.bxor, 0x31)]:
    enc_i32_i64(inst, r.rr, opc)

enc_i32_i64(base.imul, r.rrx, 0x0f, 0xaf)
enc_i32_i64(x86.sdivmodx, r.div, 0xf7, rrr=7)
enc_i32_i64(x86.udivmodx, r.div, 0xf7, rrr=6)

enc_i32_i64(base.copy, r.umr, 0x89)
enc_i32_i64(base.regmove, r.rmov, 0x89)

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
enc_i32_i64_ld_st(base.store, True, r.st, 0x89)
enc_i32_i64_ld_st(base.store, True, r.stDisp8, 0x89)
enc_i32_i64_ld_st(base.store, True, r.stDisp32, 0x89)

I64.enc(base.istore32.i64.any, *r.st.rex(0x89))
I64.enc(base.istore32.i64.any, *r.stDisp8.rex(0x89))
I64.enc(base.istore32.i64.any, *r.stDisp32.rex(0x89))

enc_i32_i64_ld_st(base.istore16, False, r.st, 0x66, 0x89)
enc_i32_i64_ld_st(base.istore16, False, r.stDisp8, 0x66, 0x89)
enc_i32_i64_ld_st(base.istore16, False, r.stDisp32, 0x66, 0x89)

# Byte stores are more complicated because the registers they can address
# depends of the presence of a REX prefix
I32.enc(base.istore8.i32.any, *r.st_abcd(0x88))
I64.enc(base.istore8.i32.any, *r.st_abcd(0x88))
I64.enc(base.istore8.i64.any, *r.st.rex(0x88))
I32.enc(base.istore8.i32.any, *r.stDisp8_abcd(0x88))
I64.enc(base.istore8.i32.any, *r.stDisp8_abcd(0x88))
I64.enc(base.istore8.i64.any, *r.stDisp8.rex(0x88))
I32.enc(base.istore8.i32.any, *r.stDisp32_abcd(0x88))
I64.enc(base.istore8.i32.any, *r.stDisp32_abcd(0x88))
I64.enc(base.istore8.i64.any, *r.stDisp32.rex(0x88))

enc_i32_i64_ld_st(base.load, True, r.ld, 0x8b)
enc_i32_i64_ld_st(base.load, True, r.ldDisp8, 0x8b)
enc_i32_i64_ld_st(base.load, True, r.ldDisp32, 0x8b)

I64.enc(base.uload32.i64, *r.ld.rex(0x8b))
I64.enc(base.uload32.i64, *r.ldDisp8.rex(0x8b))
I64.enc(base.uload32.i64, *r.ldDisp32.rex(0x8b))

I64.enc(base.sload32.i64, *r.ld.rex(0x63, w=1))
I64.enc(base.sload32.i64, *r.ldDisp8.rex(0x63, w=1))
I64.enc(base.sload32.i64, *r.ldDisp32.rex(0x63, w=1))

enc_i32_i64_ld_st(base.uload16, True, r.ld, 0x0f, 0xb7)
enc_i32_i64_ld_st(base.uload16, True, r.ldDisp8, 0x0f, 0xb7)
enc_i32_i64_ld_st(base.uload16, True, r.ldDisp32, 0x0f, 0xb7)

enc_i32_i64_ld_st(base.sload16, True, r.ld, 0x0f, 0xbf)
enc_i32_i64_ld_st(base.sload16, True, r.ldDisp8, 0x0f, 0xbf)
enc_i32_i64_ld_st(base.sload16, True, r.ldDisp32, 0x0f, 0xbf)

enc_i32_i64_ld_st(base.uload8, True, r.ld, 0x0f, 0xb6)
enc_i32_i64_ld_st(base.uload8, True, r.ldDisp8, 0x0f, 0xb6)
enc_i32_i64_ld_st(base.uload8, True, r.ldDisp32, 0x0f, 0xb6)

enc_i32_i64_ld_st(base.sload8, True, r.ld, 0x0f, 0xbe)
enc_i32_i64_ld_st(base.sload8, True, r.ldDisp8, 0x0f, 0xbe)
enc_i32_i64_ld_st(base.sload8, True, r.ldDisp32, 0x0f, 0xbe)

#
# Float loads and stores.
#

enc_flt(base.load.f32.any, r.fld, 0x66, 0x0f, 0x6e)
enc_flt(base.load.f32.any, r.fldDisp8, 0x66, 0x0f, 0x6e)
enc_flt(base.load.f32.any, r.fldDisp32, 0x66, 0x0f, 0x6e)

enc_flt(base.load.f64.any, r.fld, 0xf3, 0x0f, 0x7e)
enc_flt(base.load.f64.any, r.fldDisp8, 0xf3, 0x0f, 0x7e)
enc_flt(base.load.f64.any, r.fldDisp32, 0xf3, 0x0f, 0x7e)

enc_flt(base.store.f32.any, r.fst, 0x66, 0x0f, 0x7e)
enc_flt(base.store.f32.any, r.fstDisp8, 0x66, 0x0f, 0x7e)
enc_flt(base.store.f32.any, r.fstDisp32, 0x66, 0x0f, 0x7e)

enc_flt(base.store.f64.any, r.fst, 0x66, 0x0f, 0xd6)
enc_flt(base.store.f64.any, r.fstDisp8, 0x66, 0x0f, 0xd6)
enc_flt(base.store.f64.any, r.fstDisp32, 0x66, 0x0f, 0xd6)

#
# Function addresses.
#

I32.enc(base.func_addr.i32, *r.fnaddr4(0xb8))
I64.enc(base.func_addr.i64, *r.fnaddr8.rex(0xb8, w=1))

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
I32.enc(base.jump, *r.jmpb(0xeb))
I32.enc(base.jump, *r.jmpd(0xe9))
I64.enc(base.jump, *r.jmpb(0xeb))
I64.enc(base.jump, *r.jmpd(0xe9))

enc_i32_i64(base.brz, r.tjccb, 0x74)
enc_i32_i64(base.brnz, r.tjccb, 0x75)

# Branch on a b1 value in a register only looks at the low 8 bits. See also
# bint encodings below.
I32.enc(base.brz.b1, *r.t8jccb_abcd(0x74))
I64.enc(base.brz.b1, *r.t8jccb_abcd.rex(0x74))
I64.enc(base.brz.b1, *r.t8jccb_abcd(0x74))
I32.enc(base.brnz.b1, *r.t8jccb_abcd(0x75))
I64.enc(base.brnz.b1, *r.t8jccb_abcd.rex(0x75))
I64.enc(base.brnz.b1, *r.t8jccb_abcd(0x75))


#
# Trap as ud2
#
I32.enc(base.trap, *r.trap(0x0f, 0x0b))
I64.enc(base.trap, *r.trap(0x0f, 0x0b))

#
# Comparisons
#
enc_i32_i64(base.icmp, r.icscc, 0x39)

#
# Convert bool to int.
#
# This assumes that b1 is represented as an 8-bit low register with the value 0
# or 1.
I32.enc(base.bint.i32.b1, *r.urm_abcd(0x0f, 0xb6))
I64.enc(base.bint.i64.b1, *r.urm.rex(0x0f, 0xb6, w=1))
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
enc_flt(base.bitcast.f32.i32, r.frurm, 0x66, 0x0f, 0x6e)
enc_flt(base.bitcast.i32.f32, r.rfumr, 0x66, 0x0f, 0x7e)

# movq
I64.enc(base.bitcast.f64.i64, *r.frurm.rex(0x66, 0x0f, 0x6e, w=1))
I64.enc(base.bitcast.i64.f64, *r.rfumr.rex(0x66, 0x0f, 0x7e, w=1))

# cvtsi2ss
enc_i32_i64(base.fcvt_from_sint.f32, r.frurm, 0xf3, 0x0f, 0x2a)

# cvtsi2sd
enc_i32_i64(base.fcvt_from_sint.f64, r.frurm, 0xf2, 0x0f, 0x2a)

# cvtss2sd
enc_flt(base.fpromote.f64.f32, r.furm, 0xf3, 0x0f, 0x5a)

# cvtsd2ss
enc_flt(base.fdemote.f32.f64, r.furm, 0xf2, 0x0f, 0x5a)


# Binary arithmetic ops.
for inst,           opc in [
        (base.fadd, 0x58),
        (base.fsub, 0x5c),
        (base.fmul, 0x59),
        (base.fdiv, 0x5e)]:
    enc_flt(inst.f32, r.frm, 0xf3, 0x0f, opc)
    enc_flt(inst.f64, r.frm, 0xf2, 0x0f, opc)

# Binary bitwise ops.
for inst,               opc in [
        (base.band,     0x54),
        (base.band_not, 0x55),
        (base.bor,      0x56),
        (base.bxor,     0x57)]:
    enc_flt(inst.f32, r.frm, 0x0f, opc)
    enc_flt(inst.f64, r.frm, 0x0f, opc)

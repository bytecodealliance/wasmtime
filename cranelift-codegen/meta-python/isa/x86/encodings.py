"""
x86 Encodings.
"""
from __future__ import absolute_import
from cdsl.predicates import IsZero32BitFloat, IsZero64BitFloat
from cdsl.predicates import IsUnsignedInt
from base.predicates import IsColocatedFunc, IsColocatedData, LengthEquals
from base import instructions as base
from base import types
from base.formats import UnaryIeee32, UnaryIeee64, UnaryImm
from base.formats import FuncAddr, Call, LoadComplex, StoreComplex
from .defs import X86_64, X86_32
from . import recipes as r
from . import settings as cfg
from . import instructions as x86
from .legalize import x86_expand
from base.legalize import narrow, widen, expand_flags
from .settings import use_sse41, not_all_ones_funcaddrs_and_not_is_pic, \
    all_ones_funcaddrs_and_not_is_pic, is_pic, not_is_pic

try:
    from typing import TYPE_CHECKING, Any  # noqa
    if TYPE_CHECKING:
        from cdsl.instructions import MaybeBoundInst  # noqa
        from cdsl.predicates import FieldPredicate # noqa
except ImportError:
    pass


X86_32.legalize_monomorphic(expand_flags)
X86_32.legalize_type(
    default=narrow,
    b1=expand_flags,
    i8=widen,
    i16=widen,
    i32=x86_expand,
    f32=x86_expand,
    f64=x86_expand)

X86_64.legalize_monomorphic(expand_flags)
X86_64.legalize_type(
    default=narrow,
    b1=expand_flags,
    i8=widen,
    i16=widen,
    i32=x86_expand,
    i64=x86_expand,
    f32=x86_expand,
    f64=x86_expand)


#
# Helper functions for generating encodings.
#

def enc_x86_64(inst, recipe, *args, **kwargs):
    # type: (MaybeBoundInst, r.TailRecipe, *int, **int) -> None
    """
    Add encodings for `inst` to X86_64 with and without a REX prefix.
    """
    X86_64.enc(inst, *recipe.rex(*args, **kwargs))
    X86_64.enc(inst, *recipe(*args, **kwargs))


def enc_x86_64_instp(inst, recipe, instp, *args, **kwargs):
    # type: (MaybeBoundInst, r.TailRecipe, FieldPredicate, *int, **int) -> None
    """
    Add encodings for `inst` to X86_64 with and without a REX prefix.
    """
    X86_64.enc(inst, *recipe.rex(*args, **kwargs), instp=instp)
    X86_64.enc(inst, *recipe(*args, **kwargs), instp=instp)


def enc_both(inst, recipe, *args, **kwargs):
    # type: (MaybeBoundInst, r.TailRecipe, *int, **Any) -> None
    """
    Add encodings for `inst` to both X86_32 and X86_64.
    """
    X86_32.enc(inst, *recipe(*args, **kwargs))
    enc_x86_64(inst, recipe, *args, **kwargs)


def enc_both_instp(inst, recipe, instp, *args, **kwargs):
    # type: (MaybeBoundInst, r.TailRecipe, FieldPredicate, *int, **Any) -> None
    """
    Add encodings for `inst` to both X86_32 and X86_64.
    """
    X86_32.enc(inst, *recipe(*args, **kwargs), instp=instp)
    enc_x86_64_instp(inst, recipe, instp, *args, **kwargs)


def enc_i32_i64(inst, recipe, *args, **kwargs):
    # type: (MaybeBoundInst, r.TailRecipe, *int, **int) -> None
    """
    Add encodings for `inst.i32` to X86_32.
    Add encodings for `inst.i32` to X86_64 with and without REX.
    Add encodings for `inst.i64` to X86_64 with a REX.W prefix.
    """
    X86_32.enc(inst.i32, *recipe(*args, **kwargs))

    # REX-less encoding must come after REX encoding so we don't use it by
    # default. Otherwise reg-alloc would never use r8 and up.
    X86_64.enc(inst.i32, *recipe.rex(*args, **kwargs))
    X86_64.enc(inst.i32, *recipe(*args, **kwargs))

    X86_64.enc(inst.i64, *recipe.rex(*args, w=1, **kwargs))


def enc_i32_i64_instp(inst, recipe, instp, *args, **kwargs):
    # type: (MaybeBoundInst, r.TailRecipe, FieldPredicate, *int, **int) -> None
    """
    Add encodings for `inst.i32` to X86_32.
    Add encodings for `inst.i32` to X86_64 with and without REX.
    Add encodings for `inst.i64` to X86_64 with a REX.W prefix.

    Similar to `enc_i32_i64` but applies `instp` to each encoding.
    """
    X86_32.enc(inst.i32, *recipe(*args, **kwargs), instp=instp)

    # REX-less encoding must come after REX encoding so we don't use it by
    # default. Otherwise reg-alloc would never use r8 and up.
    X86_64.enc(inst.i32, *recipe.rex(*args, **kwargs), instp=instp)
    X86_64.enc(inst.i32, *recipe(*args, **kwargs), instp=instp)

    X86_64.enc(inst.i64, *recipe.rex(*args, w=1, **kwargs), instp=instp)


def enc_i32_i64_ld_st(inst, w_bit, recipe, *args, **kwargs):
    # type: (MaybeBoundInst, bool, r.TailRecipe, *int, **int) -> None
    """
    Add encodings for `inst.i32` to X86_32.
    Add encodings for `inst.i32` to X86_64 with and without REX.
    Add encodings for `inst.i64` to X86_64 with a REX prefix, using the `w_bit`
    argument to determine whether or not to set the REX.W bit.
    """
    X86_32.enc(inst.i32.any, *recipe(*args, **kwargs))

    # REX-less encoding must come after REX encoding so we don't use it by
    # default. Otherwise reg-alloc would never use r8 and up.
    X86_64.enc(inst.i32.any, *recipe.rex(*args, **kwargs))
    X86_64.enc(inst.i32.any, *recipe(*args, **kwargs))

    if w_bit:
        X86_64.enc(inst.i64.any, *recipe.rex(*args, w=1, **kwargs))
    else:
        X86_64.enc(inst.i64.any, *recipe.rex(*args, **kwargs))
        X86_64.enc(inst.i64.any, *recipe(*args, **kwargs))


for inst,           opc in [
        (base.iadd, 0x01),
        (base.isub, 0x29),
        (base.band, 0x21),
        (base.bor,  0x09),
        (base.bxor, 0x31)]:
    enc_i32_i64(inst, r.rr, opc)

# x86 has a bitwise not instruction NOT.
enc_i32_i64(base.bnot, r.ur, 0xf7, rrr=2)

# Also add a `b1` encodings for the logic instructions.
# TODO: Should this be done with 8-bit instructions? It would improve
# partial register dependencies.
enc_both(base.band.b1, r.rr, 0x21)
enc_both(base.bor.b1,  r.rr, 0x09)
enc_both(base.bxor.b1, r.rr, 0x31)

enc_i32_i64(base.imul, r.rrx, 0x0f, 0xaf)
enc_i32_i64(x86.sdivmodx, r.div, 0xf7, rrr=7)
enc_i32_i64(x86.udivmodx, r.div, 0xf7, rrr=6)

enc_i32_i64(x86.smulx, r.mulx, 0xf7, rrr=5)
enc_i32_i64(x86.umulx, r.mulx, 0xf7, rrr=4)

enc_i32_i64(base.copy, r.umr, 0x89)
for ty in [types.b1, types.i8, types.i16]:
    enc_both(base.copy.bind(ty), r.umr, 0x89)

# For x86-64, only define REX forms for now, since we can't describe the
# special regunit immediate operands with the current constraint language.
for ty in [types.i8, types.i16, types.i32]:
    X86_32.enc(base.regmove.bind(ty), *r.rmov(0x89))
    X86_64.enc(base.regmove.bind(ty), *r.rmov.rex(0x89))
X86_64.enc(base.regmove.i64, *r.rmov.rex(0x89, w=1))

enc_both(base.regmove.b1, r.rmov, 0x89)
enc_both(base.regmove.i8, r.rmov, 0x89)

# Immediate instructions with sign-extended 8-bit and 32-bit immediate.
for inst,               rrr in [
        (base.iadd_imm, 0),
        (base.band_imm, 4),
        (base.bor_imm,  1),
        (base.bxor_imm, 6)]:
    enc_i32_i64(inst, r.r_ib, 0x83, rrr=rrr)
    enc_i32_i64(inst, r.r_id, 0x81, rrr=rrr)

# TODO: band_imm.i64 with an unsigned 32-bit immediate can be encoded as
# band_imm.i32. Can even use the single-byte immediate for 0xffff_ffXX masks.

# Immediate constants.
X86_32.enc(base.iconst.i32, *r.pu_id(0xb8))

X86_64.enc(base.iconst.i32, *r.pu_id.rex(0xb8))
X86_64.enc(base.iconst.i32, *r.pu_id(0xb8))
# The 32-bit immediate movl also zero-extends to 64 bits.
X86_64.enc(base.iconst.i64, *r.pu_id.rex(0xb8),
           instp=IsUnsignedInt(UnaryImm.imm, 32))
X86_64.enc(base.iconst.i64, *r.pu_id(0xb8),
           instp=IsUnsignedInt(UnaryImm.imm, 32))
# Sign-extended 32-bit immediate.
X86_64.enc(base.iconst.i64, *r.u_id.rex(0xc7, rrr=0, w=1))
# Finally, the 0xb8 opcode takes an 8-byte immediate with a REX.W prefix.
X86_64.enc(base.iconst.i64, *r.pu_iq.rex(0xb8, w=1))

# bool constants.
enc_both(base.bconst.b1, r.pu_id_bool, 0xb8)

# Shifts and rotates.
# Note that the dynamic shift amount is only masked by 5 or 6 bits; the 8-bit
# and 16-bit shifts would need explicit masking.
for inst,           rrr in [
        (base.rotl, 0),
        (base.rotr, 1),
        (base.ishl, 4),
        (base.ushr, 5),
        (base.sshr, 7)]:
    # Cannot use enc_i32_i64 for this pattern because instructions require
    # .any suffix.
    X86_32.enc(inst.i32.any, *r.rc(0xd3, rrr=rrr))
    X86_64.enc(inst.i64.any, *r.rc.rex(0xd3, rrr=rrr, w=1))
    X86_64.enc(inst.i32.any, *r.rc.rex(0xd3, rrr=rrr))
    X86_64.enc(inst.i32.any, *r.rc(0xd3, rrr=rrr))

for inst,           rrr in [
        (base.ishl_imm, 4),
        (base.ushr_imm, 5),
        (base.sshr_imm, 7)]:
    enc_i32_i64(inst, r.r_ib, 0xc1, rrr=rrr)

# Population count.
X86_32.enc(base.popcnt.i32, *r.urm(0xf3, 0x0f, 0xb8), isap=cfg.use_popcnt)
X86_64.enc(base.popcnt.i64, *r.urm.rex(0xf3, 0x0f, 0xb8, w=1),
           isap=cfg.use_popcnt)
X86_64.enc(base.popcnt.i32, *r.urm.rex(0xf3, 0x0f, 0xb8), isap=cfg.use_popcnt)
X86_64.enc(base.popcnt.i32, *r.urm(0xf3, 0x0f, 0xb8), isap=cfg.use_popcnt)

# Count leading zero bits.
X86_32.enc(base.clz.i32, *r.urm(0xf3, 0x0f, 0xbd), isap=cfg.use_lzcnt)
X86_64.enc(base.clz.i64, *r.urm.rex(0xf3, 0x0f, 0xbd, w=1),
           isap=cfg.use_lzcnt)
X86_64.enc(base.clz.i32, *r.urm.rex(0xf3, 0x0f, 0xbd), isap=cfg.use_lzcnt)
X86_64.enc(base.clz.i32, *r.urm(0xf3, 0x0f, 0xbd), isap=cfg.use_lzcnt)

# Count trailing zero bits.
X86_32.enc(base.ctz.i32, *r.urm(0xf3, 0x0f, 0xbc), isap=cfg.use_bmi1)
X86_64.enc(base.ctz.i64, *r.urm.rex(0xf3, 0x0f, 0xbc, w=1),
           isap=cfg.use_bmi1)
X86_64.enc(base.ctz.i32, *r.urm.rex(0xf3, 0x0f, 0xbc), isap=cfg.use_bmi1)
X86_64.enc(base.ctz.i32, *r.urm(0xf3, 0x0f, 0xbc), isap=cfg.use_bmi1)

#
# Loads and stores.
#

ldcomplexp = LengthEquals(LoadComplex, 2)
for recipe in [r.ldWithIndex, r.ldWithIndexDisp8, r.ldWithIndexDisp32]:
    enc_i32_i64_instp(base.load_complex, recipe, ldcomplexp, 0x8b)
    enc_x86_64_instp(base.uload32_complex, recipe, ldcomplexp, 0x8b)
    X86_64.enc(base.sload32_complex, *recipe.rex(0x63, w=1),
               instp=ldcomplexp)
    enc_i32_i64_instp(base.uload16_complex, recipe, ldcomplexp, 0x0f, 0xb7)
    enc_i32_i64_instp(base.sload16_complex, recipe, ldcomplexp, 0x0f, 0xbf)
    enc_i32_i64_instp(base.uload8_complex, recipe, ldcomplexp, 0x0f, 0xb6)
    enc_i32_i64_instp(base.sload8_complex, recipe, ldcomplexp, 0x0f, 0xbe)

stcomplexp = LengthEquals(StoreComplex, 3)
for recipe in [r.stWithIndex, r.stWithIndexDisp8, r.stWithIndexDisp32]:
    enc_i32_i64_instp(base.store_complex, recipe, stcomplexp, 0x89)
    enc_x86_64_instp(base.istore32_complex, recipe, stcomplexp, 0x89)
    enc_both_instp(base.istore16_complex.i32, recipe, stcomplexp, 0x66, 0x89)
    enc_x86_64_instp(base.istore16_complex.i64, recipe, stcomplexp, 0x66, 0x89)

for recipe in [r.stWithIndex_abcd,
               r.stWithIndexDisp8_abcd,
               r.stWithIndexDisp32_abcd]:
    enc_both_instp(base.istore8_complex.i32, recipe, stcomplexp, 0x88)
    enc_x86_64_instp(base.istore8_complex.i64, recipe, stcomplexp, 0x88)

for recipe in [r.st, r.stDisp8, r.stDisp32]:
    enc_i32_i64_ld_st(base.store, True, recipe, 0x89)
    enc_x86_64(base.istore32.i64.any, recipe, 0x89)
    enc_i32_i64_ld_st(base.istore16, False, recipe, 0x66, 0x89)

# Byte stores are more complicated because the registers they can address
# depends of the presence of a REX prefix. The st*_abcd recipes fall back to
# the corresponding st* recipes when a REX prefix is applied.
for recipe in [r.st_abcd, r.stDisp8_abcd, r.stDisp32_abcd]:
    enc_both(base.istore8.i32.any, recipe, 0x88)
    enc_x86_64(base.istore8.i64.any, recipe, 0x88)

enc_i32_i64(base.spill, r.spillSib32, 0x89)
enc_i32_i64(base.regspill, r.regspill32, 0x89)

# Use a 32-bit write for spilling `b1`, `i8` and `i16` to avoid
# constraining the permitted registers.
# See MIN_SPILL_SLOT_SIZE which makes this safe.
for ty in [types.b1, types.i8, types.i16]:
    enc_both(base.spill.bind(ty), r.spillSib32, 0x89)
    enc_both(base.regspill.bind(ty), r.regspill32, 0x89)

for recipe in [r.ld, r.ldDisp8, r.ldDisp32]:
    enc_i32_i64_ld_st(base.load, True, recipe, 0x8b)
    enc_x86_64(base.uload32.i64, recipe, 0x8b)
    X86_64.enc(base.sload32.i64, *recipe.rex(0x63, w=1))
    enc_i32_i64_ld_st(base.uload16, True, recipe, 0x0f, 0xb7)
    enc_i32_i64_ld_st(base.sload16, True, recipe, 0x0f, 0xbf)
    enc_i32_i64_ld_st(base.uload8, True, recipe, 0x0f, 0xb6)
    enc_i32_i64_ld_st(base.sload8, True, recipe, 0x0f, 0xbe)

enc_i32_i64(base.fill, r.fillSib32, 0x8b)
enc_i32_i64(base.regfill, r.regfill32, 0x8b)

# Load 32 bits from `b1`, `i8` and `i16` spill slots. See `spill.b1` above.
for ty in [types.b1, types.i8, types.i16]:
    enc_both(base.fill.bind(ty), r.fillSib32, 0x8b)
    enc_both(base.regfill.bind(ty), r.regfill32, 0x8b)

# Push and Pop
X86_32.enc(x86.push.i32, *r.pushq(0x50))
enc_x86_64(x86.push.i64, r.pushq, 0x50)

X86_32.enc(x86.pop.i32, *r.popq(0x58))
enc_x86_64(x86.pop.i64, r.popq, 0x58)

# Copy Special
# For x86-64, only define REX forms for now, since we can't describe the
# special regunit immediate operands with the current constraint language.
X86_64.enc(base.copy_special, *r.copysp.rex(0x89, w=1))
X86_32.enc(base.copy_special, *r.copysp(0x89))

# Adjust SP down by a dynamic value (or up, with a negative operand).
X86_32.enc(base.adjust_sp_down.i32, *r.adjustsp(0x29))
X86_64.enc(base.adjust_sp_down.i64, *r.adjustsp.rex(0x29, w=1))

# Adjust SP up by an immediate (or down, with a negative immediate)
X86_32.enc(base.adjust_sp_up_imm, *r.adjustsp_ib(0x83))
X86_32.enc(base.adjust_sp_up_imm, *r.adjustsp_id(0x81))
X86_64.enc(base.adjust_sp_up_imm, *r.adjustsp_ib.rex(0x83, w=1))
X86_64.enc(base.adjust_sp_up_imm, *r.adjustsp_id.rex(0x81, w=1))

# Adjust SP down by an immediate (or up, with a negative immediate)
X86_32.enc(base.adjust_sp_down_imm, *r.adjustsp_ib(0x83, rrr=5))
X86_32.enc(base.adjust_sp_down_imm, *r.adjustsp_id(0x81, rrr=5))
X86_64.enc(base.adjust_sp_down_imm, *r.adjustsp_ib.rex(0x83, rrr=5, w=1))
X86_64.enc(base.adjust_sp_down_imm, *r.adjustsp_id.rex(0x81, rrr=5, w=1))

#
# Float loads and stores.
#

enc_both(base.load.f32.any, r.fld, 0xf3, 0x0f, 0x10)
enc_both(base.load.f32.any, r.fldDisp8, 0xf3, 0x0f, 0x10)
enc_both(base.load.f32.any, r.fldDisp32, 0xf3, 0x0f, 0x10)

enc_both(base.load_complex.f32, r.fldWithIndex, 0xf3, 0x0f, 0x10)
enc_both(base.load_complex.f32, r.fldWithIndexDisp8, 0xf3, 0x0f, 0x10)
enc_both(base.load_complex.f32, r.fldWithIndexDisp32, 0xf3, 0x0f, 0x10)

enc_both(base.load.f64.any, r.fld, 0xf2, 0x0f, 0x10)
enc_both(base.load.f64.any, r.fldDisp8, 0xf2, 0x0f, 0x10)
enc_both(base.load.f64.any, r.fldDisp32, 0xf2, 0x0f, 0x10)

enc_both(base.load_complex.f64, r.fldWithIndex, 0xf2, 0x0f, 0x10)
enc_both(base.load_complex.f64, r.fldWithIndexDisp8, 0xf2, 0x0f, 0x10)
enc_both(base.load_complex.f64, r.fldWithIndexDisp32, 0xf2, 0x0f, 0x10)

enc_both(base.store.f32.any, r.fst, 0xf3, 0x0f, 0x11)
enc_both(base.store.f32.any, r.fstDisp8, 0xf3, 0x0f, 0x11)
enc_both(base.store.f32.any, r.fstDisp32, 0xf3, 0x0f, 0x11)

enc_both(base.store_complex.f32, r.fstWithIndex, 0xf3, 0x0f, 0x11)
enc_both(base.store_complex.f32, r.fstWithIndexDisp8, 0xf3, 0x0f, 0x11)
enc_both(base.store_complex.f32, r.fstWithIndexDisp32, 0xf3, 0x0f, 0x11)

enc_both(base.store.f64.any, r.fst, 0xf2, 0x0f, 0x11)
enc_both(base.store.f64.any, r.fstDisp8, 0xf2, 0x0f, 0x11)
enc_both(base.store.f64.any, r.fstDisp32, 0xf2, 0x0f, 0x11)

enc_both(base.store_complex.f64, r.fstWithIndex, 0xf2, 0x0f, 0x11)
enc_both(base.store_complex.f64, r.fstWithIndexDisp8, 0xf2, 0x0f, 0x11)
enc_both(base.store_complex.f64, r.fstWithIndexDisp32, 0xf2, 0x0f, 0x11)

enc_both(base.fill.f32, r.ffillSib32, 0xf3, 0x0f, 0x10)
enc_both(base.regfill.f32, r.fregfill32, 0xf3, 0x0f, 0x10)
enc_both(base.fill.f64, r.ffillSib32, 0xf2, 0x0f, 0x10)
enc_both(base.regfill.f64, r.fregfill32, 0xf2, 0x0f, 0x10)

enc_both(base.spill.f32, r.fspillSib32, 0xf3, 0x0f, 0x11)
enc_both(base.regspill.f32, r.fregspill32, 0xf3, 0x0f, 0x11)
enc_both(base.spill.f64, r.fspillSib32, 0xf2, 0x0f, 0x11)
enc_both(base.regspill.f64, r.fregspill32, 0xf2, 0x0f, 0x11)

#
# Function addresses.
#

# Non-PIC, all-ones funcaddresses.
X86_32.enc(base.func_addr.i32, *r.fnaddr4(0xb8),
           isap=not_all_ones_funcaddrs_and_not_is_pic)
X86_64.enc(base.func_addr.i64, *r.fnaddr8.rex(0xb8, w=1),
           isap=not_all_ones_funcaddrs_and_not_is_pic)

# Non-PIC, all-zeros funcaddresses.
X86_32.enc(base.func_addr.i32, *r.allones_fnaddr4(0xb8),
           isap=all_ones_funcaddrs_and_not_is_pic)
X86_64.enc(base.func_addr.i64, *r.allones_fnaddr8.rex(0xb8, w=1),
           isap=all_ones_funcaddrs_and_not_is_pic)

# 64-bit, colocated, both PIC and non-PIC. Use the lea instruction's
# pc-relative field.
X86_64.enc(base.func_addr.i64, *r.pcrel_fnaddr8.rex(0x8d, w=1),
           instp=IsColocatedFunc(FuncAddr.func_ref))

# 64-bit, non-colocated, PIC.
X86_64.enc(base.func_addr.i64, *r.got_fnaddr8.rex(0x8b, w=1),
           isap=is_pic)

#
# Global addresses.
#

# Non-PIC
X86_32.enc(base.symbol_value.i32, *r.gvaddr4(0xb8),
           isap=not_is_pic)
X86_64.enc(base.symbol_value.i64, *r.gvaddr8.rex(0xb8, w=1),
           isap=not_is_pic)

# PIC, colocated
X86_64.enc(base.symbol_value.i64, *r.pcrel_gvaddr8.rex(0x8d, w=1),
           isap=is_pic,
           instp=IsColocatedData())

# PIC, non-colocated
X86_64.enc(base.symbol_value.i64, *r.got_gvaddr8.rex(0x8b, w=1),
           isap=is_pic)

#
# Stack addresses.
#
# TODO: Add encoding rules for stack_load and stack_store, so that they
# don't get legalized to stack_addr + load/store.
#
X86_32.enc(base.stack_addr.i32, *r.spaddr4_id(0x8d))
X86_64.enc(base.stack_addr.i64, *r.spaddr8_id.rex(0x8d, w=1))

#
# Call/return
#

# 32-bit, both PIC and non-PIC.
X86_32.enc(base.call, *r.call_id(0xe8))

# 64-bit, colocated, both PIC and non-PIC. Use the call instruction's
# pc-relative field.
X86_64.enc(base.call, *r.call_id(0xe8),
           instp=IsColocatedFunc(Call.func_ref))

# 64-bit, non-colocated, PIC. There is no 64-bit non-colocated non-PIC version,
# since non-PIC is currently using the large model, which requires calls be
# lowered to func_addr+call_indirect.
X86_64.enc(base.call, *r.call_plt_id(0xe8), isap=is_pic)

X86_32.enc(base.call_indirect.i32, *r.call_r(0xff, rrr=2))
X86_64.enc(base.call_indirect.i64, *r.call_r.rex(0xff, rrr=2))
X86_64.enc(base.call_indirect.i64, *r.call_r(0xff, rrr=2))

X86_32.enc(base.x_return, *r.ret(0xc3))
X86_64.enc(base.x_return, *r.ret(0xc3))

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
# Start with the worst-case encoding for X86_32 only. The register allocator
# can't handle a branch with an ABCD-constrained operand.
X86_32.enc(base.brz.b1, *r.t8jccd_long(0x84))
X86_32.enc(base.brnz.b1, *r.t8jccd_long(0x85))

enc_both(base.brz.b1, r.t8jccb_abcd, 0x74)
enc_both(base.brz.b1, r.t8jccd_abcd, 0x84)
enc_both(base.brnz.b1, r.t8jccb_abcd, 0x75)
enc_both(base.brnz.b1, r.t8jccd_abcd, 0x85)

#
# Jump tables
#
X86_64.enc(base.jump_table_entry.i64.any.any, *r.jt_entry.rex(0x63, w=1))
X86_32.enc(base.jump_table_entry.i32.any.any, *r.jt_entry(0x8b))

X86_64.enc(base.jump_table_base.i64, *r.jt_base.rex(0x8d, w=1))
X86_32.enc(base.jump_table_base.i32, *r.jt_base(0x8d))

enc_x86_64(base.indirect_jump_table_br.i64, r.indirect_jmp, 0xff, rrr=4)
X86_32.enc(base.indirect_jump_table_br.i32, *r.indirect_jmp(0xff, rrr=4))

#
# Trap as ud2
#
X86_32.enc(base.trap, *r.trap(0x0f, 0x0b))
X86_64.enc(base.trap, *r.trap(0x0f, 0x0b))

# Debug trap as int3
X86_32.enc(base.debugtrap, r.debugtrap, 0)
X86_64.enc(base.debugtrap, r.debugtrap, 0)

# Using a standard EncRecipe, not the TailRecipe.
X86_32.enc(base.trapif, r.trapif, 0)
X86_64.enc(base.trapif, r.trapif, 0)
X86_32.enc(base.trapff, r.trapff, 0)
X86_64.enc(base.trapff, r.trapff, 0)

#
# Comparisons
#
enc_i32_i64(base.icmp, r.icscc, 0x39)
enc_i32_i64(base.icmp_imm, r.icscc_ib, 0x83, rrr=7)
enc_i32_i64(base.icmp_imm, r.icscc_id, 0x81, rrr=7)
enc_i32_i64(base.ifcmp, r.rcmp, 0x39)
enc_i32_i64(base.ifcmp_imm, r.rcmp_ib, 0x83, rrr=7)
enc_i32_i64(base.ifcmp_imm, r.rcmp_id, 0x81, rrr=7)
# TODO: We could special-case ifcmp_imm(x, 0) to TEST(x, x).

X86_32.enc(base.ifcmp_sp.i32, *r.rcmp_sp(0x39))
X86_64.enc(base.ifcmp_sp.i64, *r.rcmp_sp.rex(0x39, w=1))

#
# Convert flags to bool.
#
# This encodes `b1` as an 8-bit low register with the value 0 or 1.
enc_both(base.trueif, r.seti_abcd, 0x0f, 0x90)
enc_both(base.trueff, r.setf_abcd, 0x0f, 0x90)

#
# Conditional move (a.k.a integer select)
#
enc_i32_i64(base.selectif, r.cmov, 0x0F, 0x40)

#
# Bit scan forwards and reverse
#
enc_i32_i64(x86.bsf, r.bsf_and_bsr, 0x0F, 0xBC)
enc_i32_i64(x86.bsr, r.bsf_and_bsr, 0x0F, 0xBD)

#
# Convert bool to int.
#
# This assumes that b1 is represented as an 8-bit low register with the value 0
# or 1.
#
# Encode movzbq as movzbl, because it's equivalent and shorter.
X86_32.enc(base.bint.i32.b1, *r.urm_noflags_abcd(0x0f, 0xb6))
X86_64.enc(base.bint.i64.b1, *r.urm_noflags.rex(0x0f, 0xb6))
X86_64.enc(base.bint.i64.b1, *r.urm_noflags_abcd(0x0f, 0xb6))
X86_64.enc(base.bint.i32.b1, *r.urm_noflags.rex(0x0f, 0xb6))
X86_64.enc(base.bint.i32.b1, *r.urm_noflags_abcd(0x0f, 0xb6))

# Numerical conversions.

# Reducing an integer is a no-op.
X86_32.enc(base.ireduce.i8.i16, r.null, 0)
X86_32.enc(base.ireduce.i8.i32, r.null, 0)
X86_32.enc(base.ireduce.i16.i32, r.null, 0)

X86_64.enc(base.ireduce.i8.i16, r.null, 0)
X86_64.enc(base.ireduce.i8.i32, r.null, 0)
X86_64.enc(base.ireduce.i16.i32, r.null, 0)
X86_64.enc(base.ireduce.i8.i64, r.null, 0)
X86_64.enc(base.ireduce.i16.i64, r.null, 0)
X86_64.enc(base.ireduce.i32.i64, r.null, 0)

# TODO: Add encodings for cbw, cwde, cdqe, which are sign-extending
# instructions for %al/%ax/%eax to %ax/%eax/%rax.

# movsbl
X86_32.enc(base.sextend.i32.i8, *r.urm_noflags_abcd(0x0f, 0xbe))
X86_64.enc(base.sextend.i32.i8, *r.urm_noflags.rex(0x0f, 0xbe))
X86_64.enc(base.sextend.i32.i8, *r.urm_noflags_abcd(0x0f, 0xbe))

# movswl
X86_32.enc(base.sextend.i32.i16, *r.urm_noflags(0x0f, 0xbf))
X86_64.enc(base.sextend.i32.i16, *r.urm_noflags.rex(0x0f, 0xbf))
X86_64.enc(base.sextend.i32.i16, *r.urm_noflags(0x0f, 0xbf))

# movsbq
X86_64.enc(base.sextend.i64.i8, *r.urm_noflags.rex(0x0f, 0xbe, w=1))

# movswq
X86_64.enc(base.sextend.i64.i16, *r.urm_noflags.rex(0x0f, 0xbf, w=1))

# movslq
X86_64.enc(base.sextend.i64.i32, *r.urm_noflags.rex(0x63, w=1))

# movzbl
X86_32.enc(base.uextend.i32.i8, *r.urm_noflags_abcd(0x0f, 0xb6))
X86_64.enc(base.uextend.i32.i8, *r.urm_noflags.rex(0x0f, 0xb6))
X86_64.enc(base.uextend.i32.i8, *r.urm_noflags_abcd(0x0f, 0xb6))

# movzwl
X86_32.enc(base.uextend.i32.i16, *r.urm_noflags(0x0f, 0xb7))
X86_64.enc(base.uextend.i32.i16, *r.urm_noflags.rex(0x0f, 0xb7))
X86_64.enc(base.uextend.i32.i16, *r.urm_noflags(0x0f, 0xb7))

# movzbq, encoded as movzbl because it's equivalent and shorter
X86_64.enc(base.uextend.i64.i8, *r.urm_noflags.rex(0x0f, 0xb6))
X86_64.enc(base.uextend.i64.i8, *r.urm_noflags_abcd(0x0f, 0xb6))

# movzwq, encoded as movzwl because it's equivalent and shorter
X86_64.enc(base.uextend.i64.i16, *r.urm_noflags.rex(0x0f, 0xb7))
X86_64.enc(base.uextend.i64.i16, *r.urm_noflags(0x0f, 0xb7))

# A 32-bit register copy clears the high 32 bits.
X86_64.enc(base.uextend.i64.i32, *r.umr.rex(0x89))
X86_64.enc(base.uextend.i64.i32, *r.umr(0x89))


#
# Floating point
#

# floating-point constants equal to 0.0 can be encoded using either
# `xorps` or `xorpd`, for 32-bit and 64-bit floats respectively.
X86_32.enc(base.f32const, *r.f32imm_z(0x0f, 0x57),
           instp=IsZero32BitFloat(UnaryIeee32.imm))
X86_32.enc(base.f64const, *r.f64imm_z(0x66, 0x0f, 0x57),
           instp=IsZero64BitFloat(UnaryIeee64.imm))

enc_x86_64_instp(base.f32const, r.f32imm_z,
                 IsZero32BitFloat(UnaryIeee32.imm), 0x0f, 0x57)
enc_x86_64_instp(base.f64const, r.f64imm_z,
                 IsZero64BitFloat(UnaryIeee64.imm), 0x66, 0x0f, 0x57)

# movd
enc_both(base.bitcast.f32.i32, r.frurm, 0x66, 0x0f, 0x6e)
enc_both(base.bitcast.i32.f32, r.rfumr, 0x66, 0x0f, 0x7e)

# movq
X86_64.enc(base.bitcast.f64.i64, *r.frurm.rex(0x66, 0x0f, 0x6e, w=1))
X86_64.enc(base.bitcast.i64.f64, *r.rfumr.rex(0x66, 0x0f, 0x7e, w=1))

# movaps
enc_both(base.copy.f32, r.furm, 0x0f, 0x28)
enc_both(base.copy.f64, r.furm, 0x0f, 0x28)

# For x86-64, only define REX forms for now, since we can't describe the
# special regunit immediate operands with the current constraint language.
X86_32.enc(base.regmove.f32, *r.frmov(0x0f, 0x28))
X86_64.enc(base.regmove.f32, *r.frmov.rex(0x0f, 0x28))

# For x86-64, only define REX forms for now, since we can't describe the
# special regunit immediate operands with the current constraint language.
X86_32.enc(base.regmove.f64, *r.frmov(0x0f, 0x28))
X86_64.enc(base.regmove.f64, *r.frmov.rex(0x0f, 0x28))

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
X86_64.enc(x86.cvtt2si.i64.f32, *r.rfurm.rex(0xf3, 0x0f, 0x2c, w=1))

# cvttsd2si
enc_both(x86.cvtt2si.i32.f64, r.rfurm, 0xf2, 0x0f, 0x2c)
X86_64.enc(x86.cvtt2si.i64.f64, *r.rfurm.rex(0xf2, 0x0f, 0x2c, w=1))

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

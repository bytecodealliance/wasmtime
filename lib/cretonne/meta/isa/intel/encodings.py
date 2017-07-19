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

for inst,           opc in [
        (base.iadd, 0x01),
        (base.isub, 0x29),
        (base.band, 0x21),
        (base.bor,  0x09),
        (base.bxor, 0x31)]:
    I32.enc(inst.i32, *r.rr(opc))

    I64.enc(inst.i64, *r.rr.rex(opc, w=1))
    I64.enc(inst.i32, *r.rr.rex(opc))
    # REX-less encoding must come after REX encoding so we don't use it by
    # default. Otherwise reg-alloc would never use r8 and up.
    I64.enc(inst.i32, *r.rr(opc))

I32.enc(base.imul.i32, *r.rrx(0x0f, 0xaf))
I64.enc(base.imul.i64, *r.rrx.rex(0x0f, 0xaf, w=1))
I64.enc(base.imul.i32, *r.rrx.rex(0x0f, 0xaf))
I64.enc(base.imul.i32, *r.rrx(0x0f, 0xaf))

for inst,              rrr in [
        (x86.sdivmodx, 7),
        (x86.udivmodx, 6)]:
    I32.enc(inst.i32, *r.div(0xf7, rrr=rrr))
    I64.enc(inst.i64, *r.div.rex(0xf7, rrr=rrr, w=1))
    I64.enc(inst.i32, *r.div.rex(0xf7, rrr=rrr))
    I64.enc(inst.i32, *r.div(0xf7, rrr=rrr))

I32.enc(base.copy.i32, *r.umr(0x89))
I64.enc(base.copy.i64, *r.umr.rex(0x89, w=1))
I64.enc(base.copy.i32, *r.umr.rex(0x89))
I64.enc(base.copy.i32, *r.umr(0x89))

I32.enc(base.regmove.i32, *r.rmov(0x89))
I64.enc(base.regmove.i64, *r.rmov.rex(0x89, w=1))
I64.enc(base.regmove.i32, *r.rmov.rex(0x89))
I64.enc(base.regmove.i32, *r.rmov(0x89))

# Immediate instructions with sign-extended 8-bit and 32-bit immediate.
for inst,               rrr in [
        (base.iadd_imm, 0),
        (base.band_imm, 4),
        (base.bor_imm,  1),
        (base.bxor_imm, 6)]:
    I32.enc(inst.i32, *r.rib(0x83, rrr=rrr))
    I32.enc(inst.i32, *r.rid(0x81, rrr=rrr))

    I64.enc(inst.i64, *r.rib.rex(0x83, rrr=rrr, w=1))
    I64.enc(inst.i64, *r.rid.rex(0x81, rrr=rrr, w=1))
    I64.enc(inst.i32, *r.rib.rex(0x83, rrr=rrr))
    I64.enc(inst.i32, *r.rid.rex(0x81, rrr=rrr))
    I64.enc(inst.i32, *r.rib(0x83, rrr=rrr))
    I64.enc(inst.i32, *r.rid(0x81, rrr=rrr))

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
    I32.enc(inst.i32.i32, *r.rc(0xd3, rrr=rrr))
    I64.enc(inst.i64.i64, *r.rc.rex(0xd3, rrr=rrr, w=1))
    I64.enc(inst.i32.i32, *r.rc.rex(0xd3, rrr=rrr))
    I64.enc(inst.i32.i32, *r.rc(0xd3, rrr=rrr))

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

# Loads and stores.
I32.enc(base.store.i32.i32, *r.st(0x89))
I32.enc(base.store.i32.i32, *r.stDisp8(0x89))
I32.enc(base.store.i32.i32, *r.stDisp32(0x89))

I32.enc(base.istore16.i32.i32, *r.st(0x66, 0x89))
I32.enc(base.istore16.i32.i32, *r.stDisp8(0x66, 0x89))
I32.enc(base.istore16.i32.i32, *r.stDisp32(0x66, 0x89))

I32.enc(base.istore8.i32.i32, *r.st_abcd(0x88))
I32.enc(base.istore8.i32.i32, *r.stDisp8_abcd(0x88))
I32.enc(base.istore8.i32.i32, *r.stDisp32_abcd(0x88))

I32.enc(base.load.i32.i32, *r.ld(0x8b))
I32.enc(base.load.i32.i32, *r.ldDisp8(0x8b))
I32.enc(base.load.i32.i32, *r.ldDisp32(0x8b))

I32.enc(base.uload16.i32.i32, *r.ld(0x0f, 0xb7))
I32.enc(base.uload16.i32.i32, *r.ldDisp8(0x0f, 0xb7))
I32.enc(base.uload16.i32.i32, *r.ldDisp32(0x0f, 0xb7))

I32.enc(base.sload16.i32.i32, *r.ld(0x0f, 0xbf))
I32.enc(base.sload16.i32.i32, *r.ldDisp8(0x0f, 0xbf))
I32.enc(base.sload16.i32.i32, *r.ldDisp32(0x0f, 0xbf))

I32.enc(base.uload8.i32.i32, *r.ld(0x0f, 0xb6))
I32.enc(base.uload8.i32.i32, *r.ldDisp8(0x0f, 0xb6))
I32.enc(base.uload8.i32.i32, *r.ldDisp32(0x0f, 0xb6))

I32.enc(base.sload8.i32.i32, *r.ld(0x0f, 0xbe))
I32.enc(base.sload8.i32.i32, *r.ldDisp8(0x0f, 0xbe))
I32.enc(base.sload8.i32.i32, *r.ldDisp32(0x0f, 0xbe))

#
# Call/return
#
I32.enc(base.call, *r.call_id(0xe8))
I32.enc(base.call_indirect.i32, *r.call_r(0xff, rrr=2))
I32.enc(base.x_return, *r.ret(0xc3))
I64.enc(base.x_return, *r.ret(0xc3))

#
# Branches
#
I32.enc(base.jump, *r.jmpb(0xeb))
I32.enc(base.jump, *r.jmpd(0xe9))
I64.enc(base.jump, *r.jmpb(0xeb))
I64.enc(base.jump, *r.jmpd(0xe9))

I32.enc(base.brz.i32, *r.tjccb(0x74))
I64.enc(base.brz.i64, *r.tjccb.rex(0x74, w=1))
I64.enc(base.brz.i32, *r.tjccb.rex(0x74))
I64.enc(base.brz.i32, *r.tjccb(0x74))

I32.enc(base.brnz.i32, *r.tjccb(0x75))
I64.enc(base.brnz.i64, *r.tjccb.rex(0x75, w=1))
I64.enc(base.brnz.i32, *r.tjccb.rex(0x75))
I64.enc(base.brnz.i32, *r.tjccb(0x75))

#
# Comparisons
#
I32.enc(base.icmp.i32, *r.icscc(0x39))
I64.enc(base.icmp.i64, *r.icscc.rex(0x39, w=1))
I64.enc(base.icmp.i32, *r.icscc.rex(0x39))
I64.enc(base.icmp.i32, *r.icscc(0x39))

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

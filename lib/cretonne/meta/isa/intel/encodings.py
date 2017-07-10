"""
Intel Encodings.
"""
from __future__ import absolute_import
from cdsl.predicates import IsUnsignedInt
from base import instructions as base
from base.formats import UnaryImm
from .defs import I32, I64
from . import recipes as r

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

I32.enc(base.copy.i32, *r.ur(0x89))

I64.enc(base.copy.i64, *r.ur.rex(0x89, w=1))
I64.enc(base.copy.i32, *r.ur.rex(0x89))
I64.enc(base.copy.i32, *r.ur(0x89))

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

# 32-bit shifts and rotates.
# Note that the dynamic shift amount is only masked by 5 or 6 bits; the 8-bit
# and 16-bit shifts would need explicit masking.
I32.enc(base.ishl.i32.i32, *r.rc(0xd3, rrr=4))
I32.enc(base.ushr.i32.i32, *r.rc(0xd3, rrr=5))
I32.enc(base.sshr.i32.i32, *r.rc(0xd3, rrr=7))

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

"""
Intel Encodings.
"""
from __future__ import absolute_import
from base import instructions as base
from .defs import I32
from . import recipes as r

I32.enc(base.iadd.i32, *r.rr(0x01))
I32.enc(base.isub.i32, *r.rr(0x29))

I32.enc(base.band.i32, *r.rr(0x21))
I32.enc(base.bor.i32,  *r.rr(0x09))
I32.enc(base.bxor.i32, *r.rr(0x31))

I32.enc(base.copy.i32, *r.ur(0x89))

# Immediate instructions with sign-extended 8-bit and 32-bit immediate.
for inst,                   rrr in [
        (base.iadd_imm.i32, 0),
        (base.band_imm.i32, 4),
        (base.bor_imm.i32,  1),
        (base.bxor_imm.i32, 6)]:
    I32.enc(inst, *r.rib(0x83, rrr=rrr))
    I32.enc(inst, *r.rid(0x81, rrr=rrr))

# Immediate constant.
I32.enc(base.iconst.i32, *r.uid(0xb8))

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

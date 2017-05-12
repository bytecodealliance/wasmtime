"""
Intel Encodings.
"""
from __future__ import absolute_import
from base import instructions as base
from .defs import I32
from . import recipes as rcp
from .recipes import OP, OP0F, MP66

I32.enc(base.iadd.i32, rcp.Op1rr, OP(0x01))
I32.enc(base.isub.i32, rcp.Op1rr, OP(0x29))

I32.enc(base.band.i32, rcp.Op1rr, OP(0x21))
I32.enc(base.bor.i32,  rcp.Op1rr, OP(0x09))
I32.enc(base.bxor.i32, rcp.Op1rr, OP(0x31))

# Immediate instructions with sign-extended 8-bit and 32-bit immediate.
for inst,                   r in [
        (base.iadd_imm.i32, 0),
        (base.band_imm.i32, 4),
        (base.bor_imm.i32,  1),
        (base.bxor_imm.i32, 6)]:
    I32.enc(inst, rcp.Op1rib, OP(0x83, rrr=r))
    I32.enc(inst, rcp.Op1rid, OP(0x81, rrr=r))

# 32-bit shifts and rotates.
# Note that the dynamic shift amount is only masked by 5 or 6 bits; the 8-bit
# and 16-bit shifts would need explicit masking.
I32.enc(base.ishl.i32.i32, rcp.Op1rc, OP(0xd3, rrr=4))
I32.enc(base.ushr.i32.i32, rcp.Op1rc, OP(0xd3, rrr=5))
I32.enc(base.sshr.i32.i32, rcp.Op1rc, OP(0xd3, rrr=7))

# Loads and stores.
I32.enc(base.store.i32.i32, rcp.Op1st,       OP(0x89))
I32.enc(base.store.i32.i32, rcp.Op1stDisp8,  OP(0x89))
I32.enc(base.store.i32.i32, rcp.Op1stDisp32, OP(0x89))

I32.enc(base.istore16.i32.i32, rcp.Mp1st,       MP66(0x89))
I32.enc(base.istore16.i32.i32, rcp.Mp1stDisp8,  MP66(0x89))
I32.enc(base.istore16.i32.i32, rcp.Mp1stDisp32, MP66(0x89))

I32.enc(base.istore8.i32.i32, rcp.Op1st_abcd,       OP(0x88))
I32.enc(base.istore8.i32.i32, rcp.Op1stDisp8_abcd,  OP(0x88))
I32.enc(base.istore8.i32.i32, rcp.Op1stDisp32_abcd, OP(0x88))

I32.enc(base.load.i32.i32, rcp.Op1ld,       OP(0x8b))
I32.enc(base.load.i32.i32, rcp.Op1ldDisp8,  OP(0x8b))
I32.enc(base.load.i32.i32, rcp.Op1ldDisp32, OP(0x8b))

I32.enc(base.uload16.i32.i32, rcp.Op2ld,       OP0F(0xb7))
I32.enc(base.uload16.i32.i32, rcp.Op2ldDisp8,  OP0F(0xb7))
I32.enc(base.uload16.i32.i32, rcp.Op2ldDisp32, OP0F(0xb7))

I32.enc(base.sload16.i32.i32, rcp.Op2ld,       OP0F(0xbf))
I32.enc(base.sload16.i32.i32, rcp.Op2ldDisp8,  OP0F(0xbf))
I32.enc(base.sload16.i32.i32, rcp.Op2ldDisp32, OP0F(0xbf))

I32.enc(base.uload8.i32.i32, rcp.Op2ld,       OP0F(0xb6))
I32.enc(base.uload8.i32.i32, rcp.Op2ldDisp8,  OP0F(0xb6))
I32.enc(base.uload8.i32.i32, rcp.Op2ldDisp32, OP0F(0xb6))

I32.enc(base.sload8.i32.i32, rcp.Op2ld,       OP0F(0xbe))
I32.enc(base.sload8.i32.i32, rcp.Op2ldDisp8,  OP0F(0xbe))
I32.enc(base.sload8.i32.i32, rcp.Op2ldDisp32, OP0F(0xbe))

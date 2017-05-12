"""
Intel Encodings.
"""
from __future__ import absolute_import
from base import instructions as base
from .defs import I32
from .recipes import Op1rr, Op1rc, Op1rib, Op1rid
from .recipes import OP

I32.enc(base.iadd.i32, Op1rr, OP(0x01))
I32.enc(base.isub.i32, Op1rr, OP(0x29))

I32.enc(base.band.i32, Op1rr, OP(0x21))
I32.enc(base.bor.i32, Op1rr, OP(0x09))
I32.enc(base.bxor.i32, Op1rr, OP(0x31))

# Immediate instructions with sign-extended 8-bit and 32-bit immediate.
for inst,                   r in [
        (base.iadd_imm.i32, 0),
        (base.band_imm.i32, 4),
        (base.bor_imm.i32,  1),
        (base.bxor_imm.i32, 6)]:
    I32.enc(inst, Op1rib, OP(0x83, rrr=r))
    I32.enc(inst, Op1rid, OP(0x81, rrr=r))

# 32-bit shifts and rotates.
# Note that the dynamic shift amount is only masked by 5 or 6 bits; the 8-bit
# and 16-bit shifts would need explicit masking.
I32.enc(base.ishl.i32.i32, Op1rc, OP(0xd3, rrr=4))
I32.enc(base.ushr.i32.i32, Op1rc, OP(0xd3, rrr=5))
I32.enc(base.sshr.i32.i32, Op1rc, OP(0xd3, rrr=7))

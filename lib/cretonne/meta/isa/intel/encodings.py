"""
Intel Encodings.
"""
from __future__ import absolute_import
from base import instructions as base
from .defs import I32
from .recipes import Op1rr, Op1rc
from .recipes import OP

I32.enc(base.iadd.i32, Op1rr, OP(0x01))

# 32-bit shifts and rotates.
# Note that the dynamic shift amount is only masked by 5 or 6 bits; the 8-bit
# and 16-bit shifts would need explicit masking.
I32.enc(base.ishl.i32.i32, Op1rc, OP(0xd3, rrr=4))
I32.enc(base.ushr.i32.i32, Op1rc, OP(0xd3, rrr=5))
I32.enc(base.sshr.i32.i32, Op1rc, OP(0xd3, rrr=7))

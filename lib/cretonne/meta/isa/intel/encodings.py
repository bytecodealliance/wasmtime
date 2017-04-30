"""
Intel Encodings.
"""
from __future__ import absolute_import
from base import instructions as base
from .defs import I32
from .recipes import Op1rr
from .recipes import OP

I32.enc(base.iadd.i32, Op1rr, OP(0x01))

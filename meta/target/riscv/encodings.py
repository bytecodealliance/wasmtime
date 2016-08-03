"""
RISC-V Encodings.
"""
from cretonne import base
from cretonne.types import i32, i64
from defs import RV32, RV64
from recipes import OP, R

# Basic arithmetic binary instructions are encoded in an R-type instruction.
for inst,           f3,    f7 in [
        (base.iadd, 0b000, 0b0000000),
        (base.isub, 0b000, 0b0100000),
        (base.ishl, 0b001, 0b0000000),
        (base.ushr, 0b101, 0b0000000),
        (base.sshr, 0b101, 0b0100000),
        (base.bxor, 0b100, 0b0000000),
        (base.bor,  0b110, 0b0000000),
        (base.band, 0b111, 0b0000000)
        ]:
    RV32.enc(inst, i32, R, OP(f3, f7))
    RV64.enc(inst, i64, R, OP(f3, f7))

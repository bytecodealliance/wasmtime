"""
RISC-V Encodings.
"""
from __future__ import absolute_import
from cretonne import base
from .defs import RV32, RV64
from .recipes import OPIMM, OPIMM32, OP, OP32, R, Rshamt, I

# Basic arithmetic binary instructions are encoded in an R-type instruction.
for inst,           inst_imm,      f3,    f7 in [
        (base.iadd, base.iadd_imm, 0b000, 0b0000000),
        (base.isub, None,          0b000, 0b0100000),
        (base.bxor, base.bxor_imm, 0b100, 0b0000000),
        (base.bor,  base.bor_imm,  0b110, 0b0000000),
        (base.band, base.band_imm, 0b111, 0b0000000)
        ]:
    RV32.enc(inst.i32, R, OP(f3, f7))
    RV64.enc(inst.i64, R, OP(f3, f7))

    # Immediate versions for add/xor/or/and.
    if inst_imm:
        RV32.enc(inst_imm.i32, I, OPIMM(f3))
        RV64.enc(inst_imm.i64, I, OPIMM(f3))

# 32-bit ops in RV64.
RV64.enc(base.iadd.i32, R, OP32(0b000, 0b0000000))
RV64.enc(base.isub.i32, R, OP32(0b000, 0b0100000))
# There are no andiw/oriw/xoriw variations.
RV64.enc(base.iadd_imm.i32, I, OPIMM32(0b000))

# Dynamic shifts have the same masking semantics as the cton base instructions.
for inst,           inst_imm,      f3,    f7 in [
        (base.ishl, base.ishl_imm, 0b001, 0b0000000),
        (base.ushr, base.ushr_imm, 0b101, 0b0000000),
        (base.sshr, base.sshr_imm, 0b101, 0b0100000),
        ]:
    RV32.enc(inst.i32.i32, R, OP(f3, f7))
    RV64.enc(inst.i64.i64, R, OP(f3, f7))
    RV64.enc(inst.i32.i32, R, OP32(f3, f7))
    # Allow i32 shift amounts in 64-bit shifts.
    RV64.enc(inst.i64.i32, R, OP(f3, f7))
    RV64.enc(inst.i32.i64, R, OP32(f3, f7))

    # Immediate shifts.
    RV32.enc(inst_imm.i32, Rshamt, OPIMM(f3, f7))
    RV64.enc(inst_imm.i64, Rshamt, OPIMM(f3, f7))
    RV64.enc(inst_imm.i32, Rshamt, OPIMM32(f3, f7))

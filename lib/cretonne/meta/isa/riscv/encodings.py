"""
RISC-V Encodings.
"""
from __future__ import absolute_import
from base import instructions as base
from base.immediates import intcc
from .defs import RV32, RV64
from .recipes import OPIMM, OPIMM32, OP, OP32
from .recipes import JALR, R, Rshamt, Ricmp, I, Iicmp, Iret
from .settings import use_m
from cdsl.ast import Var

# Dummies for instruction predicates.
x = Var('x')
y = Var('y')

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

# Signed and unsigned integer 'less than'. There are no 'w' variants for
# comparing 32-bit numbers in RV64.
RV32.enc(base.icmp.i32(intcc.slt, x, y), Ricmp, OP(0b010, 0b0000000))
RV64.enc(base.icmp.i64(intcc.slt, x, y), Ricmp, OP(0b010, 0b0000000))
RV32.enc(base.icmp.i32(intcc.ult, x, y), Ricmp, OP(0b011, 0b0000000))
RV64.enc(base.icmp.i64(intcc.ult, x, y), Ricmp, OP(0b011, 0b0000000))

RV32.enc(base.icmp_imm.i32(intcc.slt, x, y), Iicmp, OPIMM(0b010))
RV64.enc(base.icmp_imm.i64(intcc.slt, x, y), Iicmp, OPIMM(0b010))
RV32.enc(base.icmp_imm.i32(intcc.ult, x, y), Iicmp, OPIMM(0b011))
RV64.enc(base.icmp_imm.i64(intcc.ult, x, y), Iicmp, OPIMM(0b011))

# "M" Standard Extension for Integer Multiplication and Division.
# Gated by the `use_m` flag.
RV32.enc(base.imul.i32, R, OP(0b000, 0b0000001), isap=use_m)
RV64.enc(base.imul.i64, R, OP(0b000, 0b0000001), isap=use_m)
RV64.enc(base.imul.i32, R, OP32(0b000, 0b0000001), isap=use_m)

# Control flow.

# Returns are a special case of JALR.
# Note: Return stack predictors will only recognize this as a return when the
# return address is provided in `x1`. We may want a special encoding to enforce
# that.
RV32.enc(base.return_reg.i32, Iret, JALR())
RV64.enc(base.return_reg.i64, Iret, JALR())

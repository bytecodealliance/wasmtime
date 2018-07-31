"""
RISC-V Encodings.
"""
from __future__ import absolute_import
from base import instructions as base
from base.immediates import intcc
from .defs import RV32, RV64
from .recipes import OPIMM, OPIMM32, OP, OP32, LUI, BRANCH, JALR, JAL
from .recipes import LOAD, STORE
from .recipes import R, Rshamt, Ricmp, Ii, Iz, Iicmp, Iret, Icall, Icopy
from .recipes import U, UJ, UJcall, SB, SBzero, GPsp, GPfi, Irmov
from .settings import use_m
from cdsl.ast import Var
from base.legalize import narrow, expand

RV32.legalize_monomorphic(expand)
RV32.legalize_type(
        default=narrow,
        i32=expand,
        f32=expand,
        f64=expand)

RV64.legalize_monomorphic(expand)
RV64.legalize_type(
        default=narrow,
        i32=expand,
        i64=expand,
        f32=expand,
        f64=expand)

# Dummies for instruction predicates.
x = Var('x')
y = Var('y')
dest = Var('dest')
args = Var('args')

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
        RV32.enc(inst_imm.i32, Ii, OPIMM(f3))
        RV64.enc(inst_imm.i64, Ii, OPIMM(f3))

# 32-bit ops in RV64.
RV64.enc(base.iadd.i32, R, OP32(0b000, 0b0000000))
RV64.enc(base.isub.i32, R, OP32(0b000, 0b0100000))
# There are no andiw/oriw/xoriw variations.
RV64.enc(base.iadd_imm.i32, Ii, OPIMM32(0b000))

# Use iadd_imm with %x0 to materialize constants.
RV32.enc(base.iconst.i32, Iz, OPIMM(0b000))
RV64.enc(base.iconst.i32, Iz, OPIMM(0b000))
RV64.enc(base.iconst.i64, Iz, OPIMM(0b000))

# Dynamic shifts have the same masking semantics as the clif base instructions.
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

# Integer constants with the low 12 bits clear are materialized by lui.
RV32.enc(base.iconst.i32, U, LUI())
RV64.enc(base.iconst.i32, U, LUI())
RV64.enc(base.iconst.i64, U, LUI())

# "M" Standard Extension for Integer Multiplication and Division.
# Gated by the `use_m` flag.
RV32.enc(base.imul.i32, R, OP(0b000, 0b0000001), isap=use_m)
RV64.enc(base.imul.i64, R, OP(0b000, 0b0000001), isap=use_m)
RV64.enc(base.imul.i32, R, OP32(0b000, 0b0000001), isap=use_m)

# Control flow.

# Unconditional branches.
RV32.enc(base.jump, UJ, JAL())
RV64.enc(base.jump, UJ, JAL())
RV32.enc(base.call, UJcall, JAL())
RV64.enc(base.call, UJcall, JAL())

# Conditional branches.
for cond,           f3 in [
        (intcc.eq,  0b000),
        (intcc.ne,  0b001),
        (intcc.slt, 0b100),
        (intcc.sge, 0b101),
        (intcc.ult, 0b110),
        (intcc.uge, 0b111)
        ]:
    RV32.enc(base.br_icmp.i32(cond, x, y, dest, args), SB, BRANCH(f3))
    RV64.enc(base.br_icmp.i64(cond, x, y, dest, args), SB, BRANCH(f3))

for inst,           f3 in [
        (base.brz,  0b000),
        (base.brnz, 0b001)
        ]:
    RV32.enc(inst.i32, SBzero, BRANCH(f3))
    RV64.enc(inst.i64, SBzero, BRANCH(f3))
    RV32.enc(inst.b1, SBzero, BRANCH(f3))
    RV64.enc(inst.b1, SBzero, BRANCH(f3))

# Returns are a special case of JALR using %x1 to hold the return address.
# The return address is provided by a special-purpose `link` return value that
# is added by legalize_signature().
RV32.enc(base.x_return, Iret, JALR())
RV64.enc(base.x_return, Iret, JALR())
RV32.enc(base.call_indirect.i32, Icall, JALR())
RV64.enc(base.call_indirect.i64, Icall, JALR())

# Spill and fill.
RV32.enc(base.spill.i32, GPsp, STORE(0b010))
RV64.enc(base.spill.i32, GPsp, STORE(0b010))
RV64.enc(base.spill.i64, GPsp, STORE(0b011))
RV32.enc(base.fill.i32, GPfi, LOAD(0b010))
RV64.enc(base.fill.i32, GPfi, LOAD(0b010))
RV64.enc(base.fill.i64, GPfi, LOAD(0b011))

# Register copies.
RV32.enc(base.copy.i32, Icopy, OPIMM(0b000))
RV64.enc(base.copy.i64, Icopy, OPIMM(0b000))
RV64.enc(base.copy.i32, Icopy, OPIMM32(0b000))

RV32.enc(base.regmove.i32, Irmov, OPIMM(0b000))
RV64.enc(base.regmove.i64, Irmov, OPIMM(0b000))
RV64.enc(base.regmove.i32, Irmov, OPIMM32(0b000))

RV32.enc(base.copy.b1, Icopy, OPIMM(0b000))
RV64.enc(base.copy.b1, Icopy, OPIMM(0b000))
RV32.enc(base.regmove.b1, Irmov, OPIMM(0b000))
RV64.enc(base.regmove.b1, Irmov, OPIMM(0b000))

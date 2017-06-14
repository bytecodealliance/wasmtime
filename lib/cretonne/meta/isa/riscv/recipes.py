"""
RISC-V Encoding recipes.

The encoding recipes defined here more or less correspond to the RISC-V native
instruction formats described in the reference:

    The RISC-V Instruction Set Manual
    Volume I: User-Level ISA
    Version 2.1
"""
from __future__ import absolute_import
from cdsl.isa import EncRecipe
from cdsl.predicates import IsSignedInt
from cdsl.registers import Stack
from base.formats import Binary, BinaryImm, MultiAry, IntCompare, IntCompareImm
from base.formats import Unary, UnaryImm, BranchIcmp, Branch, Jump
from base.formats import Call, IndirectCall
from .registers import GPR

# The low 7 bits of a RISC-V instruction is the base opcode. All 32-bit
# instructions have 11 as the two low bits, with bits 6:2 determining the base
# opcode.
#
# Encbits for the 32-bit recipes are opcode[6:2] | (funct3 << 5) | ...
# The functions below encode the encbits.


def LOAD(funct3):
    # type: (int) -> int
    assert funct3 <= 0b111
    return 0b00000 | (funct3 << 5)


def STORE(funct3):
    # type: (int) -> int
    assert funct3 <= 0b111
    return 0b01000 | (funct3 << 5)


def BRANCH(funct3):
    # type: (int) -> int
    assert funct3 <= 0b111
    return 0b11000 | (funct3 << 5)


def JALR(funct3=0):
    # type: (int) -> int
    assert funct3 <= 0b111
    return 0b11001 | (funct3 << 5)


def JAL():
    # type: () -> int
    return 0b11011


def OPIMM(funct3, funct7=0):
    # type: (int, int) -> int
    assert funct3 <= 0b111
    return 0b00100 | (funct3 << 5) | (funct7 << 8)


def OPIMM32(funct3, funct7=0):
    # type: (int, int) -> int
    assert funct3 <= 0b111
    return 0b00110 | (funct3 << 5) | (funct7 << 8)


def OP(funct3, funct7):
    # type: (int, int) -> int
    assert funct3 <= 0b111
    assert funct7 <= 0b1111111
    return 0b01100 | (funct3 << 5) | (funct7 << 8)


def OP32(funct3, funct7):
    # type: (int, int) -> int
    assert funct3 <= 0b111
    assert funct7 <= 0b1111111
    return 0b01110 | (funct3 << 5) | (funct7 << 8)


def AIUPC():
    # type: () -> int
    return 0b00101


def LUI():
    # type: () -> int
    return 0b01101


# R-type 32-bit instructions: These are mostly binary arithmetic instructions.
# The encbits are `opcode[6:2] | (funct3 << 5) | (funct7 << 8)
R = EncRecipe('R', Binary, size=4, ins=(GPR, GPR), outs=GPR)

# R-type with an immediate shift amount instead of rs2.
Rshamt = EncRecipe('Rshamt', BinaryImm, size=4, ins=GPR, outs=GPR)

# R-type encoding of an integer comparison.
Ricmp = EncRecipe('Ricmp', IntCompare, size=4, ins=(GPR, GPR), outs=GPR)

I = EncRecipe(
        'I', BinaryImm, size=4, ins=GPR, outs=GPR,
        instp=IsSignedInt(BinaryImm.imm, 12))

# I-type encoding of an integer comparison.
Iicmp = EncRecipe(
        'Iicmp', IntCompareImm, size=4, ins=GPR, outs=GPR,
        instp=IsSignedInt(IntCompareImm.imm, 12))

# I-type encoding for `jalr` as a return instruction. We won't use the
# immediate offset.
# The variable return values are not encoded.
Iret = EncRecipe('Iret', MultiAry, size=4, ins=(), outs=())

# I-type encoding for `jalr` as an indirect call.
Icall = EncRecipe('Icall', IndirectCall, size=4, ins=GPR, outs=())

# Copy of a GPR is implemented as addi x, 0.
Icopy = EncRecipe('Icopy', Unary, size=4, ins=GPR, outs=GPR)

# U-type instructions have a 20-bit immediate that targets bits 12-31.
U = EncRecipe(
        'U', UnaryImm, size=4, ins=(), outs=GPR,
        instp=IsSignedInt(UnaryImm.imm, 32, 12))

# UJ-type unconditional branch instructions.
UJ = EncRecipe('UJ', Jump, size=4, ins=(), outs=(), branch_range=(0, 21))
UJcall = EncRecipe('UJcall', Call, size=4, ins=(), outs=())

# SB-type branch instructions.
# TODO: These instructions have a +/- 4 KB branch range. How to encode that
# constraint?
SB = EncRecipe(
        'SB', BranchIcmp, size=4,
        ins=(GPR, GPR), outs=(),
        branch_range=(0, 13))

# SB-type branch instruction with rs2 fixed to zero.
SBzero = EncRecipe(
        'SBzero', Branch, size=4,
        ins=(GPR), outs=(),
        branch_range=(0, 13))

# Spill of a GPR.
GPsp = EncRecipe(
        'GPsp', Unary, size=4,
        ins=GPR, outs=Stack(GPR))

# Fill of a GPR.
GPfi = EncRecipe(
        'GPfi', Unary, size=4,
        ins=Stack(GPR), outs=GPR)

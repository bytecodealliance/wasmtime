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
from base.formats import Call, CallIndirect, RegMove
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
R = EncRecipe(
        'R', Binary, base_size=4, ins=(GPR, GPR), outs=GPR,
        emit='put_r(bits, in_reg0, in_reg1, out_reg0, sink);')

# R-type with an immediate shift amount instead of rs2.
Rshamt = EncRecipe(
        'Rshamt', BinaryImm, base_size=4, ins=GPR, outs=GPR,
        emit='put_rshamt(bits, in_reg0, imm.into(), out_reg0, sink);')

# R-type encoding of an integer comparison.
Ricmp = EncRecipe(
        'Ricmp', IntCompare, base_size=4, ins=(GPR, GPR), outs=GPR,
        emit='put_r(bits, in_reg0, in_reg1, out_reg0, sink);')

Ii = EncRecipe(
        'Ii', BinaryImm, base_size=4, ins=GPR, outs=GPR,
        instp=IsSignedInt(BinaryImm.imm, 12),
        emit='put_i(bits, in_reg0, imm.into(), out_reg0, sink);')

# I-type instruction with a hardcoded %x0 rs1.
Iz = EncRecipe(
        'Iz', UnaryImm, base_size=4, ins=(), outs=GPR,
        instp=IsSignedInt(UnaryImm.imm, 12),
        emit='put_i(bits, 0, imm.into(), out_reg0, sink);')

# I-type encoding of an integer comparison.
Iicmp = EncRecipe(
        'Iicmp', IntCompareImm, base_size=4, ins=GPR, outs=GPR,
        instp=IsSignedInt(IntCompareImm.imm, 12),
        emit='put_i(bits, in_reg0, imm.into(), out_reg0, sink);')

# I-type encoding for `jalr` as a return instruction. We won't use the
# immediate offset.
# The variable return values are not encoded.
Iret = EncRecipe(
        'Iret', MultiAry, base_size=4, ins=(), outs=(),
        emit='''
        // Return instructions are always a jalr to %x1.
        // The return address is provided as a special-purpose link argument.
        put_i(
            bits,
            1, // rs1 = %x1
            0, // no offset.
            0, // rd = %x0: no address written.
            sink,
        );
        ''')

# I-type encoding for `jalr` as a call_indirect.
Icall = EncRecipe(
        'Icall', CallIndirect, base_size=4, ins=GPR, outs=(),
        emit='''
        // call_indirect instructions are jalr with rd=%x1.
        put_i(
            bits,
            in_reg0,
            0, // no offset.
            1, // rd = %x1: link register.
            sink,
        );
        ''')


# Copy of a GPR is implemented as addi x, 0.
Icopy = EncRecipe(
        'Icopy', Unary, base_size=4, ins=GPR, outs=GPR,
        emit='put_i(bits, in_reg0, 0, out_reg0, sink);')

# Same for a GPR regmove.
Irmov = EncRecipe(
        'Irmov', RegMove, base_size=4, ins=GPR, outs=(),
        emit='put_i(bits, src, 0, dst, sink);')

# U-type instructions have a 20-bit immediate that targets bits 12-31.
U = EncRecipe(
        'U', UnaryImm, base_size=4, ins=(), outs=GPR,
        instp=IsSignedInt(UnaryImm.imm, 32, 12),
        emit='put_u(bits, imm.into(), out_reg0, sink);')

# UJ-type unconditional branch instructions.
UJ = EncRecipe(
        'UJ', Jump, base_size=4, ins=(), outs=(), branch_range=(0, 21),
        emit='''
        let dest = i64::from(func.offsets[destination]);
        let disp = dest - i64::from(sink.offset());
        put_uj(bits, disp, 0, sink);
        ''')

UJcall = EncRecipe(
        'UJcall', Call, base_size=4, ins=(), outs=(),
        emit='''
        sink.reloc_external(Reloc::RiscvCall,
                            &func.dfg.ext_funcs[func_ref].name,
                            0);
        // rd=%x1 is the standard link register.
        put_uj(bits, 0, 1, sink);
        ''')

# SB-type branch instructions.
SB = EncRecipe(
        'SB', BranchIcmp, base_size=4,
        ins=(GPR, GPR), outs=(),
        branch_range=(0, 13),
        emit='''
        let dest = i64::from(func.offsets[destination]);
        let disp = dest - i64::from(sink.offset());
        put_sb(bits, disp, in_reg0, in_reg1, sink);
        ''')

# SB-type branch instruction with rs2 fixed to zero.
SBzero = EncRecipe(
        'SBzero', Branch, base_size=4,
        ins=(GPR), outs=(),
        branch_range=(0, 13),
        emit='''
        let dest = i64::from(func.offsets[destination]);
        let disp = dest - i64::from(sink.offset());
        put_sb(bits, disp, in_reg0, 0, sink);
        ''')

# Spill of a GPR.
GPsp = EncRecipe(
        'GPsp', Unary, base_size=4,
        ins=GPR, outs=Stack(GPR),
        emit='unimplemented!();')

# Fill of a GPR.
GPfi = EncRecipe(
        'GPfi', Unary, base_size=4,
        ins=Stack(GPR), outs=GPR,
        emit='unimplemented!();')

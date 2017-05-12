"""
Intel Encoding recipes.
"""
from __future__ import absolute_import
from cdsl.isa import EncRecipe
from cdsl.predicates import IsSignedInt, IsEqual
from base.formats import Binary, BinaryImm, Store, Load
from .registers import GPR, ABCD

# Opcode representation.
#
# Cretonne requires each recipe to have a single encoding size in bytes, and
# Intel opcodes are variable length, so we use separate recipes for different
# styles of opcodes and prefixes. The opcode format is indicated by the recipe
# name prefix:
#
# <op>            Op1* OP(op)
# 0F <op>         Op2* OP0F(op)
# 0F 38 <op>      Op3* OP38(op)
# 0F 3A <op>      Op3* OP3A(op)
# 66 <op>         Mp1* MP66(op)
# 66 0F <op>      Mp2* MP660F(op)
# 66 0F 38 <op>   Mp3* MP660F38(op)
# 66 0F 3A <op>   Mp3* MP660F3A(op)
# F2 <op>         Mp1* MPF2(op)
# F2 0F <op>      Mp2* MPF20F(op)
# F2 0F 38 <op>   Mp3* MPF20F38(op)
# F2 0F 3A <op>   Mp3* MPF20F3A(op)
# F3 <op>         Mp1* MPF3(op)
# F3 0F <op>      Mp2* MP0FF3(op)
# F3 0F 38 <op>   Mp3* MPF30F38(op)
# F3 0F 3A <op>   Mp3* MPF30F3A(op)
#
# VEX/XOP and EVEX prefixes are not yet supported.
#
# The encoding bits are:
#
# 0-7:   The opcode byte <op>.
# 8-9:   pp, mandatory prefix:
#        00 none (Op*)
#        01 66   (Mp*)
#        10 F3   (Mp*)
#        11 F2   (Mp*)
# 10-11: mm, opcode map:
#        00 <op>        (Op1/Mp1)
#        01 0F <op>     (Op2/Mp2)
#        10 0F 38 <op>  (Op3/Mp3)
#        11 0F 3A <op>  (Op3/Mp3)
# 12-14  rrr, opcode bits for the ModR/M byte for certain opcodes.
# 15:    REX.W bit (or VEX.W/E)
#
# There is some redundancy between bits 8-11 and the recipe names, but we have
# enough bits, and the pp+mm format is ready for supporting VEX prefixes.


def OP(op, pp=0, mm=0, rrr=0, w=0):
    # type: (int, int, int, int, int) -> int
    assert op <= 0xff
    assert pp <= 0b11
    assert mm <= 0b11
    assert rrr <= 0b111
    assert w <= 1
    return op | (pp << 8) | (mm << 10) | (rrr << 12) | (w << 15)


def OP0F(op, rrr=0, w=0):
    # type: (int, int, int) -> int
    return OP(op, pp=0, mm=1, rrr=rrr, w=w)


def MP66(op, rrr=0, w=0):
    # type: (int, int, int) -> int
    return OP(op, pp=1, mm=0, rrr=rrr, w=w)


# XX /r
Op1rr = EncRecipe('Op1rr', Binary, size=2, ins=(GPR, GPR), outs=0)

# XX /n with one arg in %rcx, for shifts.
Op1rc = EncRecipe('Op1rc', Binary, size=2, ins=(GPR, GPR.rcx), outs=0)

# XX /n ib with 8-bit immediate sign-extended.
Op1rib = EncRecipe(
        'Op1rib', BinaryImm, size=3, ins=GPR, outs=0,
        instp=IsSignedInt(BinaryImm.imm, 8))

# XX /n id with 32-bit immediate sign-extended.
Op1rid = EncRecipe(
        'Op1rid', BinaryImm, size=6, ins=GPR, outs=0,
        instp=IsSignedInt(BinaryImm.imm, 32))

#
# Store recipes.
#

# XX /r register-indirect store with no offset.
Op1st = EncRecipe(
        'Op1st', Store, size=2, ins=(GPR, GPR), outs=(),
        instp=IsEqual(Store.offset, 0))

# XX /r register-indirect store with no offset.
# Only ABCD allowed for stored value. This is for byte stores.
Op1st_abcd = EncRecipe(
        'Op1st_abcd', Store, size=2, ins=(ABCD, GPR), outs=(),
        instp=IsEqual(Store.offset, 0))

# XX /r register-indirect store with 8-bit offset.
Op1stDisp8 = EncRecipe(
        'Op1stDisp8', Store, size=3, ins=(GPR, GPR), outs=(),
        instp=IsSignedInt(Store.offset, 8))
Op1stDisp8_abcd = EncRecipe(
        'Op1stDisp8_abcd', Store, size=3, ins=(ABCD, GPR), outs=(),
        instp=IsSignedInt(Store.offset, 8))

# XX /r register-indirect store with 32-bit offset.
Op1stDisp32 = EncRecipe('Op1stDisp32', Store, size=6, ins=(GPR, GPR), outs=())
Op1stDisp32_abcd = EncRecipe(
        'Op1stDisp32_abcd', Store, size=6, ins=(ABCD, GPR), outs=())

# PP WW /r register-indirect store with no offset.
Mp1st = EncRecipe(
        'Mp1st', Store, size=3, ins=(GPR, GPR), outs=(),
        instp=IsEqual(Store.offset, 0))

# PP XX /r register-indirect store with 8-bit offset.
Mp1stDisp8 = EncRecipe(
        'Mp1stDisp8', Store, size=4, ins=(GPR, GPR), outs=(),
        instp=IsSignedInt(Store.offset, 8))

# PP XX /r register-indirect store with 32-bit offset.
Mp1stDisp32 = EncRecipe('Mp1stDisp32', Store, size=7, ins=(GPR, GPR), outs=())

#
# Load recipes
#

# XX /r load with no offset.
Op1ld = EncRecipe(
        'Op1ld', Load, size=2, ins=(GPR), outs=(GPR),
        instp=IsEqual(Load.offset, 0))

# XX /r load with 8-bit offset.
Op1ldDisp8 = EncRecipe(
        'Op1ldDisp8', Load, size=3, ins=(GPR), outs=(GPR),
        instp=IsSignedInt(Load.offset, 8))

# XX /r load with 32-bit offset.
Op1ldDisp32 = EncRecipe(
        'Op1ldDisp32', Load, size=6, ins=(GPR), outs=(GPR),
        instp=IsSignedInt(Load.offset, 32))

# 0F XX /r load with no offset.
Op2ld = EncRecipe(
        'Op2ld', Load, size=3, ins=(GPR), outs=(GPR),
        instp=IsEqual(Load.offset, 0))

# XX /r load with 8-bit offset.
Op2ldDisp8 = EncRecipe(
        'Op2ldDisp8', Load, size=4, ins=(GPR), outs=(GPR),
        instp=IsSignedInt(Load.offset, 8))

# XX /r load with 32-bit offset.
Op2ldDisp32 = EncRecipe(
        'Op2ldDisp32', Load, size=7, ins=(GPR), outs=(GPR),
        instp=IsSignedInt(Load.offset, 32))

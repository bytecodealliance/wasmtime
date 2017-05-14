"""
Intel Encoding recipes.
"""
from __future__ import absolute_import
from cdsl.isa import EncRecipe
from cdsl.predicates import IsSignedInt, IsEqual
from base.formats import Binary, BinaryImm, Store, Load
from .registers import GPR, ABCD

try:
    from typing import Tuple, Dict  # noqa
    from cdsl.instructions import InstructionFormat  # noqa
    from cdsl.isa import ConstraintSeq, BranchRange, PredNode  # noqa
except ImportError:
    pass


# Opcode representation.
#
# Cretonne requires each recipe to have a single encoding size in bytes, and
# Intel opcodes are variable length, so we use separate recipes for different
# styles of opcodes and prefixes. The opcode format is indicated by the recipe
# name prefix:

OPCODE_PREFIX = {
        # Prefix bytes       Name     mmpp
        ():                 ('Op1', 0b0000),
        (0x66,):            ('Mp1', 0b0001),
        (0xf3,):            ('Mp1', 0b0010),
        (0xf2,):            ('Mp1', 0b0011),
        (0x0f,):            ('Op2', 0b0100),
        (0x66, 0x0f):       ('Mp2', 0b0101),
        (0xf3, 0x0f):       ('Mp2', 0b0110),
        (0xf2, 0x0f):       ('Mp2', 0b0111),
        (0x0f, 0x38):       ('Op3', 0b1000),
        (0x66, 0x0f, 0x38): ('Mp3', 0b1001),
        (0xf3, 0x0f, 0x38): ('Mp3', 0b1010),
        (0xf2, 0x0f, 0x38): ('Mp3', 0b1011),
        (0x0f, 0x3a):       ('Op3', 0b1100),
        (0x66, 0x0f, 0x3a): ('Mp3', 0b1101),
        (0xf3, 0x0f, 0x3a): ('Mp3', 0b1110),
        (0xf2, 0x0f, 0x3a): ('Mp3', 0b1111)
        }

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


def decode_ops(ops, rrr=0, w=0):
    # type: (Tuple[int, ...], int, int) -> Tuple[str, int]
    """
    Given a sequence of opcode bytes, compute the recipe name prefix and
    encoding bits.
    """
    assert rrr <= 0b111
    assert w <= 1
    name, mmpp = OPCODE_PREFIX[ops[:-1]]
    op = ops[-1]
    assert op <= 256
    return (name, op | (mmpp << 8) | (rrr << 12) | (w << 15))


class TailRecipe:
    """
    Generate encoding recipes on demand.

    Intel encodings are somewhat orthogonal with the opcode representation on
    one side and the ModR/M, SIB and immediate fields on the other side.

    A `TailRecipe` represents the part of an encoding that follow the opcode.
    It is used to generate full encoding recipes on demand when combined with
    an opcode.

    The arguments are the same as for an `EncRecipe`, except for `size` which
    does not include the size of the opcode.
    """

    def __init__(
            self,
            name,               # type: str
            format,             # type: InstructionFormat
            size,               # type: int
            ins,                # type: ConstraintSeq
            outs,               # type: ConstraintSeq
            branch_range=None,  # type: BranchRange
            instp=None,         # type: PredNode
            isap=None           # type: PredNode
            ):
        # type: (...) -> None
        self.name = name
        self.format = format
        self.size = size
        self.ins = ins
        self.outs = outs
        self.branch_range = branch_range
        self.instp = instp
        self.isap = isap

        # Cached recipes, keyed by name prefix.
        self.recipes = dict()  # type: Dict[str, EncRecipe]

    def __call__(self, *ops, **kwargs):
        # type: (*int, **int) -> Tuple[EncRecipe, int]
        """
        Create an encoding recipe and encoding bits for the opcode bytes in
        `ops`.
        """
        rrr = kwargs.get('rrr', 0)
        w = kwargs.get('w', 0)
        name, bits = decode_ops(ops, rrr, w)
        if name not in self.recipes:
            self.recipes[name] = EncRecipe(
                name + self.name,
                self.format,
                len(ops) + self.size,
                ins=self.ins,
                outs=self.outs,
                branch_range=self.branch_range,
                instp=self.instp,
                isap=self.isap)
        return (self.recipes[name], bits)


# XX /r
rr = TailRecipe('rr', Binary, size=1, ins=(GPR, GPR), outs=0)

# XX /n with one arg in %rcx, for shifts.
rc = TailRecipe('rc', Binary, size=1, ins=(GPR, GPR.rcx), outs=0)

# XX /n ib with 8-bit immediate sign-extended.
rib = TailRecipe(
        'rib', BinaryImm, size=2, ins=GPR, outs=0,
        instp=IsSignedInt(BinaryImm.imm, 8))

# XX /n id with 32-bit immediate sign-extended.
rid = TailRecipe(
        'rid', BinaryImm, size=5, ins=GPR, outs=0,
        instp=IsSignedInt(BinaryImm.imm, 32))

#
# Store recipes.
#

# XX /r register-indirect store with no offset.
st = TailRecipe(
        'st', Store, size=1, ins=(GPR, GPR), outs=(),
        instp=IsEqual(Store.offset, 0))

# XX /r register-indirect store with no offset.
# Only ABCD allowed for stored value. This is for byte stores.
st_abcd = TailRecipe(
        'st_abcd', Store, size=1, ins=(ABCD, GPR), outs=(),
        instp=IsEqual(Store.offset, 0))

# XX /r register-indirect store with 8-bit offset.
stDisp8 = TailRecipe(
        'stDisp8', Store, size=2, ins=(GPR, GPR), outs=(),
        instp=IsSignedInt(Store.offset, 8))
stDisp8_abcd = TailRecipe(
        'stDisp8_abcd', Store, size=2, ins=(ABCD, GPR), outs=(),
        instp=IsSignedInt(Store.offset, 8))

# XX /r register-indirect store with 32-bit offset.
stDisp32 = TailRecipe('stDisp32', Store, size=5, ins=(GPR, GPR), outs=())
stDisp32_abcd = TailRecipe(
        'stDisp32_abcd', Store, size=5, ins=(ABCD, GPR), outs=())

#
# Load recipes
#

# XX /r load with no offset.
ld = TailRecipe(
        'ld', Load, size=1, ins=(GPR), outs=(GPR),
        instp=IsEqual(Load.offset, 0))

# XX /r load with 8-bit offset.
ldDisp8 = TailRecipe(
        'ldDisp8', Load, size=2, ins=(GPR), outs=(GPR),
        instp=IsSignedInt(Load.offset, 8))

# XX /r load with 32-bit offset.
ldDisp32 = TailRecipe(
        'ldDisp32', Load, size=5, ins=(GPR), outs=(GPR),
        instp=IsSignedInt(Load.offset, 32))

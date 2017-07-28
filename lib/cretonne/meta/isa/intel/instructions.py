"""
Supplementary instruction definitions for Intel.

This module defines additional instructions that are useful only to the Intel
target ISA.
"""

from cdsl.operands import Operand
from cdsl.typevar import TypeVar
from cdsl.instructions import Instruction, InstructionGroup


GROUP = InstructionGroup("x86", "Intel-specific instruction set")

iWord = TypeVar('iWord', 'A scalar integer machine word', ints=(32, 64))

nlo = Operand('nlo', iWord, doc='Low part of numerator')
nhi = Operand('nhi', iWord, doc='High part of numerator')
d = Operand('d', iWord, doc='Denominator')
q = Operand('q', iWord, doc='Quotient')
r = Operand('r', iWord, doc='Remainder')

udivmodx = Instruction(
        'x86_udivmodx', r"""
        Extended unsigned division.

        Concatenate the bits in `nhi` and `nlo` to form the numerator.
        Interpret the bits as an unsigned number and divide by the unsigned
        denominator `d`. Trap when `d` is zero or if the quotient is larger
        than the range of the output.

        Return both quotient and remainder.
        """,
        ins=(nlo, nhi, d), outs=(q, r), can_trap=True)

sdivmodx = Instruction(
        'x86_sdivmodx', r"""
        Extended signed division.

        Concatenate the bits in `nhi` and `nlo` to form the numerator.
        Interpret the bits as a signed number and divide by the signed
        denominator `d`. Trap when `d` is zero or if the quotient is outside
        the range of the output.

        Return both quotient and remainder.
        """,
        ins=(nlo, nhi, d), outs=(q, r), can_trap=True)

GROUP.close()

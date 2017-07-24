"""
Supplementary instruction definitions for Intel.

This module defines additional instructions that are useful only to the Intel
target ISA.
"""

from cdsl.operands import Operand
from cdsl.instructions import Instruction, InstructionGroup
from base.instructions import iB


GROUP = InstructionGroup("x86", "Intel-specific instruction set")

nlo = Operand('nlo', iB, doc='Low part of numerator')
nhi = Operand('nhi', iB, doc='High part of numerator')
d = Operand('d', iB, doc='Denominator')
q = Operand('q', iB, doc='Quotient')
r = Operand('r', iB, doc='Remainder')

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

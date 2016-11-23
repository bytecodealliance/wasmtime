"""
ARM32 register banks.
"""
from __future__ import absolute_import
from cdsl.registers import RegBank, RegClass
from .defs import ISA


# Define the larger float bank first to avoid the alignment gap.
FloatRegs = RegBank(
        'FloatRegs', ISA, r"""
        Floating point registers.

        The floating point register units correspond to the S-registers, but
        extended as if there were 64 registers.

        - S registers are one unit each.
        - D registers are two units each, even D16 and above.
        - Q registers are 4 units each.
        """,
        units=64, prefix='s')

# Special register units:
# - r15 is the program counter.
# - r14 is the link register.
# - r13 is usually the stack pointer.
IntRegs = RegBank(
        'IntRegs', ISA,
        'General purpose registers',
        units=16, prefix='r')

GPR = RegClass('GPR', IntRegs)
S = RegClass('S', FloatRegs, count=32)
D = RegClass('D', FloatRegs, width=2)
Q = RegClass('Q', FloatRegs, width=4)

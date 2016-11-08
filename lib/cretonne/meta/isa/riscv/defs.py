"""
RISC-V definitions.

Commonly used definitions.
"""
from __future__ import absolute_import
from cdsl.isa import TargetISA, CPUMode
import base.instructions

isa = TargetISA('riscv', [base.instructions.GROUP])

# CPU modes for 32-bit and 64-bit operation.
RV32 = CPUMode('RV32', isa)
RV64 = CPUMode('RV64', isa)

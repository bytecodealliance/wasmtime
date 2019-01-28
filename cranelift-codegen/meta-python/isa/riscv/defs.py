"""
RISC-V definitions.

Commonly used definitions.
"""
from __future__ import absolute_import
from cdsl.isa import TargetISA, CPUMode
import base.instructions

ISA = TargetISA('riscv', [base.instructions.GROUP])  # type: TargetISA

# CPU modes for 32-bit and 64-bit operation.
RV32 = CPUMode('RV32', ISA)
RV64 = CPUMode('RV64', ISA)

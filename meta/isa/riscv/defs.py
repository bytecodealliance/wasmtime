"""
RISC-V definitions.

Commonly used definitions.
"""

from cretonne import TargetISA, CPUMode
import cretonne.base

isa = TargetISA('riscv', [cretonne.base.instructions])

# CPU modes for 32-bit and 64-bit operation.
RV32 = CPUMode('RV32', isa)
RV64 = CPUMode('RV64', isa)

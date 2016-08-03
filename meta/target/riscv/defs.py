"""
RISC-V definitions.

Commonly used definitions.
"""

from cretonne import Target, CPUMode
import cretonne.base

target = Target('riscv', [cretonne.base.instructions])

# CPU modes for 32-bit and 64-bit operation.
RV32 = CPUMode('RV32', target)
RV64 = CPUMode('RV64', target)

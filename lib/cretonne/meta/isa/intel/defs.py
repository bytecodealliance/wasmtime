"""
Intel definitions.

Commonly used definitions.
"""
from __future__ import absolute_import
from cdsl.isa import TargetISA, CPUMode
import base.instructions
from . import instructions as x86

ISA = TargetISA('intel', [base.instructions.GROUP, x86.GROUP])

# CPU modes for 32-bit and 64-bit operation.
I64 = CPUMode('I64', ISA)
I32 = CPUMode('I32', ISA)

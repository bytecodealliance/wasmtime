"""
ARM 32-bit definitions.

Commonly used definitions.
"""
from __future__ import absolute_import
from cdsl.isa import TargetISA, CPUMode
import base.instructions

ISA = TargetISA('arm32', [base.instructions.GROUP])

# CPU modes for 32-bit ARM and Thumb2.
A32 = CPUMode('A32', ISA)
T32 = CPUMode('T32', ISA)

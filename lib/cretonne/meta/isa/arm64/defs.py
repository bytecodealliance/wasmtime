"""
ARM64 definitions.

Commonly used definitions.
"""
from __future__ import absolute_import
from cdsl.isa import TargetISA, CPUMode
import base.instructions

ISA = TargetISA('arm64', [base.instructions.GROUP])
A64 = CPUMode('A64', ISA)

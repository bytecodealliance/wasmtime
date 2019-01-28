"""
ARM64 definitions.

Commonly used definitions.
"""
from __future__ import absolute_import
from cdsl.isa import TargetISA, CPUMode
import base.instructions
from base.legalize import narrow

ISA = TargetISA('arm64', [base.instructions.GROUP])  # type: TargetISA
A64 = CPUMode('A64', ISA)

# TODO: Refine these
A64.legalize_type(narrow)

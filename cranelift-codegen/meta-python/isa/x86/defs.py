"""
x86 definitions.

Commonly used definitions.
"""
from __future__ import absolute_import
from cdsl.isa import TargetISA, CPUMode
import base.instructions
from . import instructions as x86
from base.immediates import floatcc

ISA = TargetISA('x86', [base.instructions.GROUP, x86.GROUP])  # type: TargetISA

# CPU modes for 32-bit and 64-bit operation.
X86_64 = CPUMode('I64', ISA)
X86_32 = CPUMode('I32', ISA)

# The set of floating point condition codes that are directly supported.
# Other condition codes need to be reversed or expressed as two tests.
supported_floatccs = [
        floatcc.ord,
        floatcc.uno,
        floatcc.one,
        floatcc.ueq,
        floatcc.gt,
        floatcc.ge,
        floatcc.ult,
        floatcc.ule]

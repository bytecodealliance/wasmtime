"""
ARM 32-bit Architecture
-----------------------

This target ISA generates code for ARMv7 and ARMv8 CPUs in 32-bit mode
(AArch32). We support both ARM and Thumb2 instruction encodings.
"""

from __future__ import absolute_import
from . import defs
from . import settings, registers  # noqa
from cdsl.isa import TargetISA  # noqa

# Re-export the primary target ISA definition.
ISA = defs.ISA.finish()  # type: TargetISA

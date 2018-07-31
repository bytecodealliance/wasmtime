"""
x86 Target Architecture
-----------------------

This target ISA generates code for x86 CPUs with two separate CPU modes:

`I32`
    32-bit x86 architecture, also known as 'IA-32', also sometimes referred
    to as 'i386', however note that Cranelift depends on instructions not
    in the original `i386`, such as SSE2, CMOVcc, and UD2.

`I64`
    x86-64 architecture, also known as 'AMD64`, `Intel 64`, and 'x64'.
"""

from __future__ import absolute_import
from . import defs
from . import encodings, settings, registers  # noqa
from cdsl.isa import TargetISA  # noqa

# Re-export the primary target ISA definition.
ISA = defs.ISA.finish()  # type: TargetISA

"""
Intel Target Architecture
-------------------------

This target ISA generates code for Intel CPUs with two separate CPU modes:

`I32`
    IA-32 architecture, also known as 'x86'. Generates code for the Intel 386
    and later processors in 32-bit mode.
`I64`
    Intel 64 architecture, also known as 'x86-64, 'x64', and 'amd64'. Intel and
    AMD CPUs running in 64-bit mode.

Floating point is supported only on CPUs with support for SSE2 or later. There
is no x87 floating point support.
"""

from __future__ import absolute_import
from . import defs
from . import registers  # noqa

# Re-export the primary target ISA definition.
ISA = defs.ISA.finish()

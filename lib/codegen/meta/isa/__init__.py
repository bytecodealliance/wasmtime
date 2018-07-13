"""
Cranelift target ISA definitions
--------------------------------

The :py:mod:`isa` package contains sub-packages for each target instruction set
architecture supported by Cranelift.
"""
from __future__ import absolute_import
from cdsl.isa import TargetISA  # noqa
from . import riscv, x86, arm32, arm64

try:
    from typing import List  # noqa
except ImportError:
    pass


def all_isas():
    # type: () -> List[TargetISA]
    """
    Get a list of all the supported target ISAs. Each target ISA is represented
    as a :py:class:`cranelift.TargetISA` instance.
    """
    return [riscv.ISA, x86.ISA, arm32.ISA, arm64.ISA]

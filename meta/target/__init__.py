"""
Cretonne target definitions
---------------------------

The :py:mod:`target` package contains sub-packages for each target instruction
set architecture supported by Cretonne.
"""

from . import riscv


def all_targets():
    """
    Get a list of all the supported targets. Each target is represented as a
    :py:class:`cretonne.Target` instance.
    """
    return [riscv.target]

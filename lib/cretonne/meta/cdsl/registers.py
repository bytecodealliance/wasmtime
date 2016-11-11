"""
Register set definitions
------------------------

Each ISA defines a separate register set that is used by the register allocator
and the final binary encoding of machine code.

The CPU registers are first divided into disjoint register banks, represented
by a `RegBank` instance. Registers in different register banks never interfere
with each other. A typical CPU will have a general purpose and a floating point
register bank.

A register bank consists of a number of *register units* which are the smallest
indivisible units of allocation and interference. A register unit doesn't
necesarily correspond to a particular number of bits in a register, it is more
like a placeholder that can be used to determine of a register is taken or not.

The register allocator works with *register classes* which can allocate one or
more register units at a time. A register class allocates more than one
register unit at a time when its registers are composed of smaller alocatable
units. For example, the ARM double precision floating point registers are
composed of two single precision registers. """
from __future__ import absolute_import
from . import is_power_of_two, next_power_of_two


try:
    from typing import Sequence # noqa
    from .isa import TargetISA  # noqa
except ImportError:
    pass


class RegBank(object):
    """
    A register bank belonging to an ISA.

    A register bank controls a set of *register units* disjoint from all the
    other register banks in the ISA. The register units are numbered uniquely
    within the target ISA, and the units in a register bank form a contiguous
    sequence starting from a sufficiently aligned point that their low bits can
    be used directly when encoding machine code instructions.

    Register units can be given generated names like `r0`, `r1`, ..., or a
    tuple of special register unit names can be provided.

    :param name: Name of this register bank.
    :param doc: Documentation string.
    :param units: Number of register units.
    :param prefix: Prefix for generated unit names.
    :param names: Special names for the first units. May be shorter than
                  `units`, the remaining units are named using `prefix`.
    """

    def __init__(self, name, isa, doc, units, prefix='p', names=()):
        # type: (str, TargetISA, str, int, str, Sequence[str]) -> None
        self.name = name
        self.isa = isa
        self.first_unit = 0
        self.units = units
        self.prefix = prefix
        self.names = names

        assert len(names) <= units

        if isa.regbanks:
            # Get the next free unit number.
            last = isa.regbanks[-1]
            u = last.first_unit + last.units
            align = units
            if not is_power_of_two(align):
                align = next_power_of_two(align)
            self.first_unit = (u + align - 1) & -align

        isa.regbanks.append(self)

    def __repr__(self):
        # type: () -> str
        return ('RegBank({}, units={}, first_unit={})'
                .format(self.name, self.units, self.first_unit))

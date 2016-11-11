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
composed of two single precision registers.
"""
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


class RegClass(object):
    """
    A register class is a subset of register units in a RegBank along with a
    strategy for allocating registers.

    The *width* parameter determines how many register units are allocated at a
    time. Usually it that is one, but for example the ARM D registers are
    allocated two units at a time. When multiple units are allocated, it is
    always a contiguous set of unit numbers.

    :param name: Name of this register class.
    :param bank: The register bank we're allocating from.
    :param count: The maximum number of allocations in this register class. By
                  default, the whole register bank can be allocated.
    :param width: How many units to allocate at a time.
    :param start: The first unit to allocate, relative to `bank.first.unit`.
    :param stride: How many units to skip to get to the next allocation.
                   Default is `width`.
    """

    def __init__(self, name, bank, count=None, width=1, start=0, stride=None):
        # type: (str, RegBank, int, int, int, int) -> None
        self.name = name
        self.bank = bank
        self.start = start
        self.width = width

        assert width > 0
        assert start >= 0 and start < bank.units

        if stride is None:
            stride = width
        assert stride > 0
        self.stride = stride

        if count is None:
            count = bank.units / stride
        self.count = count

        # When the stride is 1, we can wrap around to the beginning of the
        # register bank, but with a larger stride, we wouldn't cover all the
        # possible allocations with a simple modulo stride. For example,
        # attempting to allocate the even registers before the odd ones
        # wouldn't work. Only if stride is coprime to bank.units would it work,
        # but that is unlikely since the bank size is almost always a power of
        # two.
        if start + count*stride > bank.units:
            assert stride == 1, 'Wrapping with stride not supported'

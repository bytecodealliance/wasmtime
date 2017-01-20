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


# The number of 32-bit elements in a register unit mask
MASK_LEN = 3

# The maximum total number of register units allowed.
# This limit can be raised by also adjusting the RegUnitMask type in
# src/isa/registers.rs.
MAX_UNITS = MASK_LEN * 32


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
        self.classes = list()  # type: List[RegClass]

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

    :param bank: The register bank we're allocating from.
    :param count: The maximum number of allocations in this register class. By
                  default, the whole register bank can be allocated.
    :param width: How many units to allocate at a time.
    :param start: The first unit to allocate, relative to `bank.first.unit`.
    """

    def __init__(self, bank, count=None, width=1, start=0):
        # type: (RegBank, int, int, int) -> None
        self.name = None  # type: str
        self.bank = bank
        self.start = start
        self.width = width

        assert width > 0
        assert start >= 0 and start < bank.units

        if count is None:
            count = bank.units // width
        self.count = count

        bank.classes.append(self)

    def __getitem__(self, sliced):
        """
        Create a sub-class of a register class using slice notation. The slice
        indexes refer to allocations in the parent register class, not register
        units.
        """
        assert isinstance(sliced, slice), "RegClass slicing can't be 1 reg"
        # We could add strided sub-classes if needed.
        assert sliced.step is None, 'Subclass striding not supported'

        w = self.width
        s = self.start + sliced.start * w
        c = sliced.stop - sliced.start
        assert c > 1, "Can't have single-register classes"

        return RegClass(self.bank, count=c, width=w, start=s)

    def mask(self):
        """
        Compute a bit-mask of the register units allocated by this register
        class.

        Return as a list of 32-bit integers.
        """
        mask = [0] * MASK_LEN

        start = self.bank.first_unit + self.start
        for a in range(self.count):
            u = start + a * self.width
            mask[u // 32] |= 1 << (u % 32)

        return mask

    @staticmethod
    def extract_names(globs):
        """
        Given a dict mapping name -> object as returned by `globals()`, find
        all the RegClass objects and set their name from the dict key.
        This is used to name a bunch of global variables in a module.
        """
        for name, obj in globs.items():
            if isinstance(obj, RegClass):
                assert obj.name is None
                obj.name = name

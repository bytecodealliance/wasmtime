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
    from typing import Sequence, Tuple  # noqa
    from .isa import TargetISA  # noqa
    # A tuple uniquely identifying a register class inside a register bank.
    # (count, width, start)
    RCTup = Tuple[int, int, int]
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

    def __init__(self, name, isa, doc, units, prefix='r', names=()):
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

    def finish_regclasses(self, first_index):
        # type: (int) -> None
        """
        Assign indexes to the register classes in this bank, starting from
        `first_index`.

        Verify that the set of register classes satisfies:

        1. Closed under intersection: The intersection of any two register
           classes in the set is either empty or identical to a member of the
           set.
        2. There are no identical classes under different names.
        3. Classes are sorted topologically such that all subclasses have a
           higher index that the superclass.

        We could reorder classes topologically here instead of just enforcing
        the order, but the ordering tends to fall out naturally anyway.
        """
        cmap = dict()  # type: Dict[RCTup, RegClass]

        for idx, rc in enumerate(self.classes):
            # All register classes must be given a name.
            assert rc.name, "Anonymous register class found"

            # Assign a unique index.
            assert rc.index is None
            rc.index = idx + first_index

            # Check for duplicates.
            tup = rc.rctup()
            if tup in cmap:
                raise AssertionError(
                        '{} and {} are identical register classes'
                        .format(rc, cmap[tup]))
            cmap[tup] = rc

        # Check intersections and topological order.
        for idx, rc1 in enumerate(self.classes):
            for rc2 in self.classes[0:idx]:
                itup = rc1.intersect(rc2)
                if itup is None:
                    continue
                if itup not in cmap:
                    raise AssertionError(
                        'intersection of {} and {} missing'
                        .format(rc1, rc2))
                irc = cmap[itup]
                # rc1 > rc2, so rc2 can't be the sub-class.
                if irc is rc2:
                    raise AssertionError(
                            'Bad topological order: {}/{}'
                            .format(rc1, rc2))
                if irc is rc1:
                    # The intersection of rc1 and rc2 is rc1, so it must be a
                    # sub-class.
                    rc2.subclasses.append(rc1)


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
        self.index = None  # type: int
        self.bank = bank
        self.start = start
        self.width = width

        # This is computed later in `finish_regclasses()`.
        self.subclasses = list()  # type: List[RegClass]

        assert width > 0
        assert start >= 0 and start < bank.units

        if count is None:
            count = bank.units // width
        self.count = count

        bank.classes.append(self)

    def __str__(self):
        return self.name

    def rctup(self):
        # type: () -> RCTup
        """
        Get a tuple that uniquely identifies the registers in this class.

        The tuple can be used as a dictionary key to ensure that there are no
        duplicate register classes.
        """
        return (self.count, self.width, self.start)

    def intersect(self, other):
        # type: (RegClass) -> RCTup
        """
        Get a tuple representing the intersction of two register classes.

        Returns `None` if the two classes are disjoint.
        """
        if self.width != other.width:
            return None
        s_end = self.start + self.count * self.width
        o_end = other.start + other.count * other.width
        if self.start >= o_end or other.start >= s_end:
            return None

        # We have an overlap.
        start = max(self.start, other.start)
        end = min(s_end, o_end)
        count = (end - start) // self.width
        assert count > 0
        return (count, self.width, start)

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
        # type: () -> List[int]
        """
        Compute a bit-mask of the register units allocated by this register
        class.

        Return as a list of 32-bit integers.
        """
        mask = [0] * MASK_LEN

        start = self.bank.first_unit + self.start
        for a in range(self.count):
            u = start + a * self.width
            b = u % 32
            # We need fancier masking code if a register can straddle mask
            # words. This will only happen with widths that are not powers of
            # two.
            assert b + self.width <= 32, 'Register straddles words'
            mask[u // 32] |= 1 << b

        return mask

    def subclass_mask(self):
        # type: () -> int
        """
        Compute a bit-mask of subclasses, including self.
        """
        m = 1 << self.index
        for rc in self.subclasses:
            m |= 1 << rc.index
        return m

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


class Register(object):
    """
    A specific register in a register class.

    A register is identified by the top-level register class it belongs to and
    its first register unit.

    Specific registers are used to describe constraints on instructions where
    some operands must use a fixed register.

    Register objects should be created using the indexing syntax on the
    register class.
    """
    def __init__(self, rc, unit):
        # type: (RegClass, int) -> None
        self.regclass = rc
        self.unit = unit

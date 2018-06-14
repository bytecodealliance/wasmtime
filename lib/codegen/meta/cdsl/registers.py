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
necessarily correspond to a particular number of bits in a register, it is more
like a placeholder that can be used to determine of a register is taken or not.

The register allocator works with *register classes* which can allocate one or
more register units at a time. A register class allocates more than one
register unit at a time when its registers are composed of smaller allocatable
units. For example, the ARM double precision floating point registers are
composed of two single precision registers.
"""
from __future__ import absolute_import
from . import is_power_of_two, next_power_of_two


try:
    from typing import Sequence, Tuple, List, Dict, Any, Optional, TYPE_CHECKING  # noqa
    if TYPE_CHECKING:
        from .isa import TargetISA  # noqa
        # A tuple uniquely identifying a register class inside a register bank.
        # (width, bitmask)
        RCTup = Tuple[int, int]
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
    :param pressure_tracking: Enable tracking of register pressure.
    :param prefix: Prefix for generated unit names.
    :param names: Special names for the first units. May be shorter than
                  `units`, the remaining units are named using `prefix`.
    """

    def __init__(
            self,
            name,                       # type: str
            isa,                        # type: TargetISA
            doc,                        # type: str
            units,                      # type: int
            pressure_tracking=True,     # type: bool
            prefix='r',                 # type: str
            names=()                    # type: Sequence[str]
            ):
        # type: (...) -> None
        self.name = name
        self.isa = isa
        self.first_unit = 0
        self.units = units
        self.pressure_tracking = pressure_tracking
        self.prefix = prefix
        self.names = names
        self.classes = list()  # type: List[RegClass]
        self.toprcs = list()  # type: List[RegClass]
        self.first_toprc_index = None  # type: int

        assert len(names) <= units

        if isa.regbanks:
            # Get the next free unit number.
            last = isa.regbanks[-1]
            u = last.first_unit + last.units
            align = units
            if not is_power_of_two(align):
                align = next_power_of_two(align)
            self.first_unit = (u + align - 1) & -align

        self.index = len(isa.regbanks)
        isa.regbanks.append(self)

    def __repr__(self):
        # type: () -> str
        return ('RegBank({}, units={}, first_unit={})'
                .format(self.name, self.units, self.first_unit))

    def finish_regclasses(self):
        # type: () -> None
        """
        Compute subclasses and the top-level register class.

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

        for rc in self.classes:
            # All register classes must be given a name.
            assert rc.name, "Anonymous register class found"

            # Check for duplicates.
            tup = rc.rctup()
            if tup in cmap:
                raise AssertionError(
                        '{} and {} are identical register classes'
                        .format(rc, cmap[tup]))
            cmap[tup] = rc

        # Check intersections and topological order.
        for idx, rc1 in enumerate(self.classes):
            rc1.toprc = rc1
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
                    rc1.toprc = rc2.toprc

            if rc1.is_toprc():
                self.toprcs.append(rc1)

    def unit_by_name(self, name):
        # type: (str) -> int
        """
        Get a register unit in this bank by name.
        """
        if name in self.names:
            r = self.names.index(name)
        elif name.startswith(self.prefix):
            r = int(name[len(self.prefix):])
        assert r < self.units, 'Invalid register name: ' + name
        return self.first_unit + r


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

    def __init__(self, bank, count=0, width=1, start=0, bitmask=None):
        # type: (RegBank, int, int, int, Optional[int]) -> None
        self.name = None  # type: str
        self.index = None  # type: int
        self.bank = bank
        self.width = width
        self.bitmask = 0

        # This is computed later in `finish_regclasses()`.
        self.subclasses = list()  # type: List[RegClass]
        self.toprc = None  # type: RegClass

        assert width > 0

        if bitmask:
            self.bitmask = bitmask
        else:
            assert start >= 0 and start < bank.units
            if count == 0:
                count = bank.units // width
            for a in range(count):
                u = start + a * self.width
                self.bitmask |= 1 << u

        bank.classes.append(self)

    def __str__(self):
        # type: () -> str
        return self.name

    def is_toprc(self):
        # type: () -> bool
        """
        Is this a top-level register class?

        A top-level register class has no sub-classes. This can only be
        answered aster running `finish_regclasses()`.
        """
        return self.toprc is self

    def rctup(self):
        # type: () -> RCTup
        """
        Get a tuple that uniquely identifies the registers in this class.

        The tuple can be used as a dictionary key to ensure that there are no
        duplicate register classes.
        """
        return (self.width, self.bitmask)

    def intersect(self, other):
        # type: (RegClass) -> RCTup
        """
        Get a tuple representing the intersction of two register classes.

        Returns `None` if the two classes are disjoint.
        """
        if self.width != other.width:
            return None
        intersection = self.bitmask & other.bitmask
        if intersection == 0:
            return None

        return (self.width, intersection)

    def __getitem__(self, sliced):
        # type: (slice) -> RegClass
        """
        Create a sub-class of a register class using slice notation. The slice
        indexes refer to allocations in the parent register class, not register
        units.
        """
        assert isinstance(sliced, slice), "RegClass slicing can't be 1 reg"
        # We could add strided sub-classes if needed.
        assert sliced.step is None, 'Subclass striding not supported'
        # Can't slice a non-contiguous class
        assert self.is_contiguous(), 'Cannot slice non-contiguous RegClass'

        w = self.width
        s = self.start() + sliced.start * w
        c = sliced.stop - sliced.start
        assert c > 1, "Can't have single-register classes"

        return RegClass(self.bank, count=c, width=w, start=s)

    def without(self, *registers):
        # type: (*Register) -> RegClass
        """
        Create a sub-class of a register class excluding a specific set of
        registers.

        For example: GPR.without(GPR.r9)
        """
        bm = self.bitmask
        w = self.width
        fmask = (1 << self.width) - 1
        for reg in registers:
            bm &= ~(fmask << (reg.unit * w))

        return RegClass(self.bank, bitmask=bm)

    def is_contiguous(self):
        # type: () -> bool
        """
        Returns boolean indicating whether a register class is a contiguous set
        of register units.
        """
        x = self.bitmask | (self.bitmask-1)
        return self.bitmask != 0 and ((x+1) & x) == 0

    def start(self):
        # type: () -> int
        """
        Returns the first valid register unit in this class.
        """
        start = 0
        bm = self.bitmask
        fmask = (1 << self.width) - 1
        while True:
            if bm & fmask > 0:
                break
            start += 1
            bm >>= self.width

        return start

    def __getattr__(self, attr):
        # type: (str) -> Register
        """
        Get a specific register in the class by name.

        For example: `GPR.r5`.
        """
        reg = Register(self, self.bank.unit_by_name(attr))
        # Save this register so we won't have to create it again.
        setattr(self, attr, reg)
        return reg

    def mask(self):
        # type: () -> List[int]
        """
        Compute a bit-mask of the register units allocated by this register
        class.

        Return as a list of 32-bit integers.
        """
        out_mask = []
        mask32 = (1 << 32) - 1
        bitmask = self.bitmask << self.bank.first_unit
        for i in range(MASK_LEN):
            out_mask.append((bitmask >> (i * 32)) & mask32)

        return out_mask

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
        # type: (Dict[str, Any]) -> None
        """
        Given a dict mapping name -> object as returned by `globals()`, find
        all the RegClass objects and set their name from the dict key.
        This is used to name a bunch of global values in a module.
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

    Register instances can be created with the constructor, or accessed as
    attributes on the register class: `GPR.rcx`.
    """
    def __init__(self, rc, unit):
        # type: (RegClass, int) -> None
        self.regclass = rc
        self.unit = unit


class Stack(object):
    """
    An operand that must be in a stack slot.

    A `Stack` object can be used to indicate an operand constraint for a value
    operand that must live in a stack slot.
    """
    def __init__(self, rc):
        # type: (RegClass) -> None
        self.regclass = rc

    def stack_base_mask(self):
        # type: () -> str
        """
        Get the StackBaseMask to use for this operand.

        This is a mask of base registers that can be supported by this operand.
        """
        # TODO: Make this configurable instead of just using the SP.
        return 'StackBaseMask(1)'

"""Cranelift ValueType hierarchy"""
from __future__ import absolute_import
import math

try:
    from typing import Dict, List, cast, TYPE_CHECKING # noqa
except ImportError:
    TYPE_CHECKING = False
    pass


# Numbering scheme for value types:
#
# 0: Void
# 0x01-0x6f: Special types
# 0x70-0x7f: Lane types
# 0x80-0xff: Vector types
#
# Vector types are encoded with the lane type in the low 4 bits and log2(lanes)
# in the high 4 bits, giving a range of 2-256 lanes.
LANE_BASE = 0x70


# ValueType instances (i8, i32, ...) are provided in the `base.types` module.
class ValueType(object):
    """
    A concrete SSA value type.

    All SSA values have a type that is described by an instance of `ValueType`
    or one of its subclasses.
    """

    # Map name -> ValueType.
    _registry = dict()  # type: Dict[str, ValueType]

    # List of all the lane types.
    all_lane_types = list()  # type: List[LaneType]

    # List of all the special types (neither lanes nor vectors).
    all_special_types = list()  # type: List[SpecialType]

    def __init__(self, name, membytes, doc):
        # type: (str, int, str) -> None
        self.name = name
        self.number = None  # type: int
        self.membytes = membytes
        self.__doc__ = doc
        assert name not in ValueType._registry
        ValueType._registry[name] = self

    def __str__(self):
        # type: () -> str
        return self.name

    def rust_name(self):
        # type: () -> str
        return 'ir::types::' + self.name.upper()

    @staticmethod
    def by_name(name):
        # type: (str) -> ValueType
        if name in ValueType._registry:
            return ValueType._registry[name]
        else:
            raise AttributeError("No type named '{}'".format(name))

    def lane_bits(self):
        # type: () -> int
        """Return the number of bits in a lane."""
        assert False, "Abstract"

    def lane_count(self):
        # type: () -> int
        """Return the number of lanes."""
        assert False, "Abstract"

    def width(self):
        # type: () -> int
        """Return the total number of bits of an instance of this type."""
        return self.lane_count() * self.lane_bits()

    def wider_or_equal(self, other):
        # type: (ValueType) -> bool
        """
        Return true iff:
            1. self and other have equal number of lanes
            2. each lane in self has at least as many bits as a lane in other
        """
        return (self.lane_count() == other.lane_count() and
                self.lane_bits() >= other.lane_bits())


class LaneType(ValueType):
    """
    A concrete scalar type that can appear as a vector lane too.

    Also tracks a unique set of :py:class:`VectorType` instances with this type
    as the lane type.
    """

    def __init__(self, name, membytes, doc):
        # type: (str, int, str) -> None
        super(LaneType, self).__init__(name, membytes, doc)
        self._vectors = dict()  # type: Dict[int, VectorType]
        # Assign numbers starting from LANE_BASE.
        n = len(ValueType.all_lane_types)
        ValueType.all_lane_types.append(self)
        assert n < 16, 'Too many lane types'
        self.number = LANE_BASE + n

    def __repr__(self):
        # type: () -> str
        return 'LaneType({})'.format(self.name)

    def by(self, lanes):
        # type: (int) -> VectorType
        """
        Get a vector type with this type as the lane type.

        For example, ``i32.by(4)`` returns the :obj:`i32x4` type.
        """
        if lanes in self._vectors:
            return self._vectors[lanes]
        else:
            v = VectorType(self, lanes)
            self._vectors[lanes] = v
            return v

    def lane_count(self):
        # type: () -> int
        """Return the number of lanes."""
        return 1


class VectorType(ValueType):
    """
    A concrete SIMD vector type.

    A vector type has a lane type which is an instance of :class:`LaneType`,
    and a positive number of lanes.
    """

    def __init__(self, base, lanes):
        # type: (LaneType, int) -> None
        super(VectorType, self).__init__(
                name='{}x{}'.format(base.name, lanes),
                membytes=lanes*base.membytes,
                doc="""
                A SIMD vector with {} lanes containing a `{}` each.
                """.format(lanes, base.name))
        assert lanes <= 256, "Too many lanes"
        self.base = base
        self.lanes = lanes
        self.number = 16*int(math.log(lanes, 2)) + base.number

    def __repr__(self):
        # type: () -> str
        return ('VectorType(base={}, lanes={})'
                .format(self.base.name, self.lanes))

    def lane_count(self):
        # type: () -> int
        """Return the number of lanes."""
        return self.lanes

    def lane_bits(self):
        # type: () -> int
        """Return the number of bits in a lane."""
        return self.base.lane_bits()


class SpecialType(ValueType):
    """
    A concrete scalar type that is neither a vector nor a lane type.

    Special types cannot be used to form vectors.
    """

    def __init__(self, name, membytes, doc):
        # type: (str, int, str) -> None
        super(SpecialType, self).__init__(name, membytes, doc)
        # Assign numbers starting from 1. (0 is VOID)
        ValueType.all_special_types.append(self)
        self.number = len(ValueType.all_special_types)
        assert self.number < LANE_BASE, 'Too many special types'

    def __repr__(self):
        # type: () -> str
        return 'SpecialType({})'.format(self.name)

    def lane_count(self):
        # type: () -> int
        """Return the number of lanes."""
        return 1


class IntType(LaneType):
    """A concrete scalar integer type."""

    def __init__(self, bits):
        # type: (int) -> None
        assert bits > 0, 'IntType must have positive number of bits'
        warning = ""
        if bits < 32:
            warning += "\nWARNING: "
            warning += "arithmetic on {}bit integers is incomplete".format(
                bits)
        super(IntType, self).__init__(
                name='i{:d}'.format(bits),
                membytes=bits // 8,
                doc="An integer type with {} bits.{}".format(bits, warning))
        self.bits = bits

    def __repr__(self):
        # type: () -> str
        return 'IntType(bits={})'.format(self.bits)

    @staticmethod
    def with_bits(bits):
        # type: (int) -> IntType
        typ = ValueType.by_name('i{:d}'.format(bits))
        if TYPE_CHECKING:
            return cast(IntType, typ)
        else:
            return typ

    def lane_bits(self):
        # type: () -> int
        """Return the number of bits in a lane."""
        return self.bits


class FloatType(LaneType):
    """A concrete scalar floating point type."""

    def __init__(self, bits, doc):
        # type: (int, str) -> None
        assert bits > 0, 'FloatType must have positive number of bits'
        super(FloatType, self).__init__(
                name='f{:d}'.format(bits),
                membytes=bits // 8,
                doc=doc)
        self.bits = bits

    def __repr__(self):
        # type: () -> str
        return 'FloatType(bits={})'.format(self.bits)

    @staticmethod
    def with_bits(bits):
        # type: (int) -> FloatType
        typ = ValueType.by_name('f{:d}'.format(bits))
        if TYPE_CHECKING:
            return cast(FloatType, typ)
        else:
            return typ

    def lane_bits(self):
        # type: () -> int
        """Return the number of bits in a lane."""
        return self.bits


class BoolType(LaneType):
    """A concrete scalar boolean type."""

    def __init__(self, bits):
        # type: (int) -> None
        assert bits > 0, 'BoolType must have positive number of bits'
        super(BoolType, self).__init__(
                name='b{:d}'.format(bits),
                membytes=bits // 8,
                doc="A boolean type with {} bits.".format(bits))
        self.bits = bits

    def __repr__(self):
        # type: () -> str
        return 'BoolType(bits={})'.format(self.bits)

    @staticmethod
    def with_bits(bits):
        # type: (int) -> BoolType
        typ = ValueType.by_name('b{:d}'.format(bits))
        if TYPE_CHECKING:
            return cast(BoolType, typ)
        else:
            return typ

    def lane_bits(self):
        # type: () -> int
        """Return the number of bits in a lane."""
        return self.bits


class FlagsType(SpecialType):
    """
    A type representing CPU flags.

    Flags can't be stored in memory.
    """

    def __init__(self, name, doc):
        # type: (str, str) -> None
        super(FlagsType, self).__init__(name, 0, doc)

    def __repr__(self):
        # type: () -> str
        return 'FlagsType({})'.format(self.name)


class BVType(ValueType):
    """A flat bitvector type. Used for semantics description only."""

    def __init__(self, bits):
        # type: (int) -> None
        assert bits > 0, 'Must have positive number of bits'
        super(BVType, self).__init__(
                name='bv{:d}'.format(bits),
                membytes=bits // 8,
                doc="A bitvector type with {} bits.".format(bits))
        self.bits = bits

    def __repr__(self):
        # type: () -> str
        return 'BVType(bits={})'.format(self.bits)

    @staticmethod
    def with_bits(bits):
        # type: (int) -> BVType
        name = 'bv{:d}'.format(bits)
        if name not in ValueType._registry:
            return BVType(bits)

        typ = ValueType.by_name(name)
        if TYPE_CHECKING:
            return cast(BVType, typ)
        else:
            return typ

    def lane_bits(self):
        # type: () -> int
        """Return the number of bits in a lane."""
        return self.bits

    def lane_count(self):
        # type: () -> int
        """Return the number of lane. For BVtypes always 1."""
        return 1

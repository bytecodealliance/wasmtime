"""Cretonne ValueType hierarchy"""
from __future__ import absolute_import
import math

try:
    from typing import Dict, List, cast, TYPE_CHECKING # noqa
except ImportError:
    TYPE_CHECKING = False
    pass


# ValueType instances (i8, i32, ...) are provided in the cretonne.types module.
class ValueType(object):
    """
    A concrete SSA value type.

    All SSA values have a type that is described by an instance of `ValueType`
    or one of its subclasses.
    """

    # Map name -> ValueType.
    _registry = dict()  # type: Dict[str, ValueType]

    # List of all the scalar types.
    all_scalars = list()  # type: List[ScalarType]

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
        return 'types::' + self.name.upper()

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


class ScalarType(ValueType):
    """
    A concrete scalar (not vector) type.

    Also tracks a unique set of :py:class:`VectorType` instances with this type
    as the lane type.
    """

    def __init__(self, name, membytes, doc):
        # type: (str, int, str) -> None
        super(ScalarType, self).__init__(name, membytes, doc)
        self._vectors = dict()  # type: Dict[int, VectorType]
        # Assign numbers starting from 1. (0 is VOID).
        ValueType.all_scalars.append(self)
        # Numbers are only valid for Cretone types that get emitted to Rust.
        # This excludes BVTypes
        self.number = len([x for x in ValueType.all_scalars
                           if not isinstance(x, BVType)])
        assert self.number < 16, 'Too many scalar types'

    def __repr__(self):
        # type: () -> str
        return 'ScalarType({})'.format(self.name)

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

    A vector type has a lane type which is an instance of :class:`ScalarType`,
    and a positive number of lanes.
    """

    def __init__(self, base, lanes):
        # type: (ScalarType, int) -> None
        assert isinstance(base, ScalarType), 'SIMD lanes must be scalar types'
        super(VectorType, self).__init__(
                name='{}x{}'.format(base.name, lanes),
                membytes=lanes*base.membytes,
                doc="""
                A SIMD vector with {} lanes containing a `{}` each.
                """.format(lanes, base.name))
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


class IntType(ScalarType):
    """A concrete scalar integer type."""

    def __init__(self, bits):
        # type: (int) -> None
        assert bits > 0, 'IntType must have positive number of bits'
        super(IntType, self).__init__(
                name='i{:d}'.format(bits),
                membytes=bits // 8,
                doc="An integer type with {} bits.".format(bits))
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


class FloatType(ScalarType):
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


class BoolType(ScalarType):
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


class BVType(ScalarType):
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
        typ = ValueType.by_name('bv{:d}'.format(bits))
        if TYPE_CHECKING:
            return cast(BVType, typ)
        else:
            return typ

    def lane_bits(self):
        # type: () -> int
        """Return the number of bits in a lane."""
        return self.bits

"""
Type variables for Parametric polymorphism.

Cranelift instructions and instruction transformations can be specified to be
polymorphic by using type variables.
"""
from __future__ import absolute_import
import math
from . import types, is_power_of_two
from copy import copy

try:
    from typing import Tuple, Union, Iterable, Any, Set, TYPE_CHECKING # noqa
    if TYPE_CHECKING:
        from srcgen import Formatter  # noqa
        Interval = Tuple[int, int]
        # An Interval where `True` means 'everything'
        BoolInterval = Union[bool, Interval]
        # Set of special types: None, False, True, or iterable.
        SpecialSpec = Union[bool, Iterable[types.SpecialType]]
except ImportError:
    pass

MAX_LANES = 256
MAX_BITS = 64
MAX_BITVEC = MAX_BITS * MAX_LANES


def int_log2(x):
    # type: (int) -> int
    return int(math.log(x, 2))


def intersect(a, b):
    # type: (Interval, Interval) -> Interval
    """
    Given two `(min, max)` inclusive intervals, compute their intersection.

    Use `(None, None)` to represent the empty interval on input and output.
    """
    if a[0] is None or b[0] is None:
        return (None, None)
    lo = max(a[0], b[0])
    assert lo is not None
    hi = min(a[1], b[1])
    assert hi is not None
    if lo <= hi:
        return (lo, hi)
    else:
        return (None, None)


def is_empty(intv):
    # type: (Interval) -> bool
    return intv is None or intv is False or intv == (None, None)


def encode_bitset(vals, size):
    # type: (Iterable[int], int) -> int
    """
    Encode a set of values (each between 0 and size) as a bitset of width size.
    """
    res = 0
    assert is_power_of_two(size) and size <= 64
    for v in vals:
        assert 0 <= v and v < size
        res |= 1 << v
    return res


def pp_set(s):
    # type: (Iterable[Any]) -> str
    """
    Return a consistent string representation of a set (ordering is fixed)
    """
    return '{' + ', '.join([repr(x) for x in sorted(s)]) + '}'


def decode_interval(intv, full_range, default=None):
    # type: (BoolInterval, Interval, int) -> Interval
    """
    Decode an interval specification which can take the following values:

    True
        Use the `full_range`.
    `False` or `None`
        An empty interval
    (lo, hi)
        An explicit interval
    """
    if isinstance(intv, tuple):
        # mypy bug here: 'builtins.None' object is not iterable
        lo, hi = intv
        assert is_power_of_two(lo)
        assert is_power_of_two(hi)
        assert lo <= hi
        assert lo >= full_range[0]
        assert hi <= full_range[1]
        return intv

    if intv:
        return full_range
    else:
        return (default, default)


def interval_to_set(intv):
    # type: (Interval) -> Set
    if is_empty(intv):
        return set()

    (lo, hi) = intv
    assert is_power_of_two(lo)
    assert is_power_of_two(hi)
    assert lo <= hi
    return set([2**i for i in range(int_log2(lo), int_log2(hi)+1)])


def legal_bool(bits):
    # type: (int) -> bool
    """
    True iff bits is a legal bit width for a bool type.
    bits == 1 || bits \in { 8, 16, .. MAX_BITS }
    """
    return bits == 1 or \
        (bits >= 8 and bits <= MAX_BITS and is_power_of_two(bits))


class TypeSet(object):
    """
    A set of types.

    We don't allow arbitrary subsets of types, but use a parametrized approach
    instead.

    Objects of this class can be used as dictionary keys.

    Parametrized type sets are specified in terms of ranges:

    - The permitted range of vector lanes, where 1 indicates a scalar type.
    - The permitted range of integer types.
    - The permitted range of floating point types, and
    - The permitted range of boolean types.

    The ranges are inclusive from smallest bit-width to largest bit-width.

    A typeset representing scalar integer types `i8` through `i32`:

    >>> TypeSet(ints=(8, 32))
    TypeSet(lanes={1}, ints={8, 16, 32})

    Passing `True` instead of a range selects all available scalar types:

    >>> TypeSet(ints=True)
    TypeSet(lanes={1}, ints={8, 16, 32, 64})
    >>> TypeSet(floats=True)
    TypeSet(lanes={1}, floats={32, 64})
    >>> TypeSet(bools=True)
    TypeSet(lanes={1}, bools={1, 8, 16, 32, 64})

    Similarly, passing `True` for the lanes selects all possible scalar and
    vector types:

    >>> TypeSet(lanes=True, ints=True)
    TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={8, 16, 32, 64})

    Finally, a type set can contain special types (derived from `SpecialType`)
    which can't appear as lane types.

    :param lanes: `(min, max)` inclusive range of permitted vector lane counts.
    :param ints: `(min, max)` inclusive range of permitted scalar integer
                 widths.
    :param floats: `(min, max)` inclusive range of permitted scalar floating
                   point widths.
    :param bools: `(min, max)` inclusive range of permitted scalar boolean
                  widths.
    :param bitvecs : `(min, max)` inclusive range of permitted bitvector
                  widths.
    :param specials: Sequence of special types to appear in the set.
    """

    def __init__(
            self,
            lanes=None,     # type: BoolInterval
            ints=None,      # type: BoolInterval
            floats=None,    # type: BoolInterval
            bools=None,     # type: BoolInterval
            bitvecs=None,   # type: BoolInterval
            specials=None   # type: SpecialSpec
            ):
        # type: (...) -> None
        self.lanes = interval_to_set(decode_interval(lanes, (1, MAX_LANES), 1))
        self.ints = interval_to_set(decode_interval(ints, (8, MAX_BITS)))
        self.floats = interval_to_set(decode_interval(floats, (32, 64)))
        self.bools = interval_to_set(decode_interval(bools, (1, MAX_BITS)))
        self.bools = set(filter(legal_bool, self.bools))
        self.bitvecs = interval_to_set(decode_interval(bitvecs,
                                                       (1, MAX_BITVEC)))
        # Allow specials=None, specials=True, specials=(...)
        self.specials = set()  # type: Set[types.SpecialType]
        if isinstance(specials, bool):
            if specials:
                self.specials = set(types.ValueType.all_special_types)
        elif specials:
            self.specials = set(specials)

    def copy(self):
        # type: (TypeSet) -> TypeSet
        """
        Return a copy of our self.
        """
        n = TypeSet()
        n.lanes = copy(self.lanes)
        n.ints = copy(self.ints)
        n.floats = copy(self.floats)
        n.bools = copy(self.bools)
        n.bitvecs = copy(self.bitvecs)
        n.specials = copy(self.specials)
        return n

    def typeset_key(self):
        # type: () -> Tuple[Tuple, Tuple, Tuple, Tuple, Tuple, Tuple]
        """Key tuple used for hashing and equality."""
        return (tuple(sorted(list(self.lanes))),
                tuple(sorted(list(self.ints))),
                tuple(sorted(list(self.floats))),
                tuple(sorted(list(self.bools))),
                tuple(sorted(list(self.bitvecs))),
                tuple(sorted(s.name for s in self.specials)))

    def __hash__(self):
        # type: () -> int
        h = hash(self.typeset_key())
        assert h == getattr(self, 'prev_hash', h), "TypeSet changed"
        self.prev_hash = h
        return h

    def __eq__(self, other):
        # type: (object) -> bool
        if isinstance(other, TypeSet):
            return self.typeset_key() == other.typeset_key()
        else:
            return False

    def __ne__(self, other):
        # type: (object) -> bool
        return not self.__eq__(other)

    def __repr__(self):
        # type: () -> str
        s = 'TypeSet(lanes={}'.format(pp_set(self.lanes))
        if len(self.ints) > 0:
            s += ', ints={}'.format(pp_set(self.ints))
        if len(self.floats) > 0:
            s += ', floats={}'.format(pp_set(self.floats))
        if len(self.bools) > 0:
            s += ', bools={}'.format(pp_set(self.bools))
        if len(self.bitvecs) > 0:
            s += ', bitvecs={}'.format(pp_set(self.bitvecs))
        if len(self.specials) > 0:
            s += ', specials=[{}]'.format(pp_set(self.specials))
        return s + ')'

    def emit_fields(self, fmt):
        # type: (Formatter) -> None
        """Emit field initializers for this typeset."""
        assert len(self.bitvecs) == 0, "Bitvector types are not emitable."
        fmt.comment(repr(self))

        fields = (('lanes', 16),
                  ('ints', 8),
                  ('floats', 8),
                  ('bools', 8))

        for (field, bits) in fields:
            vals = [int_log2(x) for x in getattr(self, field)]
            fmt.line('{}: BitSet::<u{}>({}),'
                     .format(field, bits, encode_bitset(vals, bits)))

    def __iand__(self, other):
        # type: (TypeSet) -> TypeSet
        """
        Intersect self with other type set.

        >>> a = TypeSet(lanes=True, ints=(16, 32))
        >>> a
        TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256}, ints={16, 32})
        >>> b = TypeSet(lanes=(4, 16), ints=True)
        >>> a &= b
        >>> a
        TypeSet(lanes={4, 8, 16}, ints={16, 32})

        >>> a = TypeSet(lanes=True, bools=(1, 8))
        >>> b = TypeSet(lanes=True, bools=(16, 32))
        >>> a &= b
        >>> a
        TypeSet(lanes={1, 2, 4, 8, 16, 32, 64, 128, 256})
        """
        self.lanes.intersection_update(other.lanes)
        self.ints.intersection_update(other.ints)
        self.floats.intersection_update(other.floats)
        self.bools.intersection_update(other.bools)
        self.bitvecs.intersection_update(other.bitvecs)
        self.specials.intersection_update(other.specials)

        return self

    def issubset(self, other):
        # type: (TypeSet) -> bool
        """
        Return true iff self is a subset of other
        """
        return self.lanes.issubset(other.lanes) and \
            self.ints.issubset(other.ints) and \
            self.floats.issubset(other.floats) and \
            self.bools.issubset(other.bools) and \
            self.bitvecs.issubset(other.bitvecs) and \
            self.specials.issubset(other.specials)

    def lane_of(self):
        # type: () -> TypeSet
        """
        Return a TypeSet describing the image of self across lane_of
        """
        new = self.copy()
        new.lanes = set([1])
        new.bitvecs = set()
        return new

    def as_bool(self):
        # type: () -> TypeSet
        """
        Return a TypeSet describing the image of self across as_bool
        """
        new = self.copy()
        new.ints = set()
        new.floats = set()
        new.bitvecs = set()

        if len(self.lanes.difference(set([1]))) > 0:
            new.bools = self.ints.union(self.floats).union(self.bools)

        if 1 in self.lanes:
            new.bools.add(1)
        return new

    def half_width(self):
        # type: () -> TypeSet
        """
        Return a TypeSet describing the image of self across halfwidth
        """
        new = self.copy()
        new.ints = set([x//2 for x in self.ints if x > 8])
        new.floats = set([x//2 for x in self.floats if x > 32])
        new.bools = set([x//2 for x in self.bools if x > 8])
        new.bitvecs = set([x//2 for x in self.bitvecs if x > 1])
        new.specials = set()

        return new

    def double_width(self):
        # type: () -> TypeSet
        """
        Return a TypeSet describing the image of self across doublewidth
        """
        new = self.copy()
        new.ints = set([x*2 for x in self.ints if x < MAX_BITS])
        new.floats = set([x*2 for x in self.floats if x < MAX_BITS])
        new.bools = set(filter(legal_bool,
                               set([x*2 for x in self.bools if x < MAX_BITS])))
        new.bitvecs = set([x*2 for x in self.bitvecs if x < MAX_BITVEC])
        new.specials = set()

        return new

    def half_vector(self):
        # type: () -> TypeSet
        """
        Return a TypeSet describing the image of self across halfvector
        """
        new = self.copy()
        new.bitvecs = set()
        new.lanes = set([x//2 for x in self.lanes if x > 1])
        new.specials = set()

        return new

    def double_vector(self):
        # type: () -> TypeSet
        """
        Return a TypeSet describing the image of self across doublevector
        """
        new = self.copy()
        new.bitvecs = set()
        new.lanes = set([x*2 for x in self.lanes if x < MAX_LANES])
        new.specials = set()

        return new

    def to_bitvec(self):
        # type: () -> TypeSet
        """
        Return a TypeSet describing the image of self across to_bitvec
        """
        assert len(self.bitvecs) == 0
        all_scalars = self.ints.union(self.floats.union(self.bools))

        new = self.copy()
        new.lanes = set([1])
        new.ints = set()
        new.bools = set()
        new.floats = set()
        new.bitvecs = set([lane_w * nlanes for lane_w in all_scalars
                           for nlanes in self.lanes])
        new.specials = set()

        return new

    def image(self, func):
        # type: (str) -> TypeSet
        """
        Return the image of self across the derived function func
        """
        if (func == TypeVar.LANEOF):
            return self.lane_of()
        elif (func == TypeVar.ASBOOL):
            return self.as_bool()
        elif (func == TypeVar.HALFWIDTH):
            return self.half_width()
        elif (func == TypeVar.DOUBLEWIDTH):
            return self.double_width()
        elif (func == TypeVar.HALFVECTOR):
            return self.half_vector()
        elif (func == TypeVar.DOUBLEVECTOR):
            return self.double_vector()
        elif (func == TypeVar.TOBITVEC):
            return self.to_bitvec()
        else:
            assert False, "Unknown derived function: " + func

    def preimage(self, func):
        # type: (str) -> TypeSet
        """
        Return the inverse image of self across the derived function func
        """
        # The inverse of the empty set is always empty
        if (self.size() == 0):
            return self

        if (func == TypeVar.LANEOF):
            new = self.copy()
            new.bitvecs = set()
            new.lanes = set([2**i for i in range(0, int_log2(MAX_LANES)+1)])
            return new
        elif (func == TypeVar.ASBOOL):
            new = self.copy()
            new.bitvecs = set()

            if 1 not in self.bools:
                new.ints = self.bools.difference(set([1]))
                new.floats = self.bools.intersection(set([32, 64]))
                # If b1 is not in our typeset, than lanes=1 cannot be in the
                # pre-image, as as_bool() of scalars is always b1.
                new.lanes = self.lanes.difference(set([1]))
            else:
                new.ints = set([2**x for x in range(3, 7)])
                new.floats = set([32, 64])

            return new
        elif (func == TypeVar.HALFWIDTH):
            return self.double_width()
        elif (func == TypeVar.DOUBLEWIDTH):
            return self.half_width()
        elif (func == TypeVar.HALFVECTOR):
            return self.double_vector()
        elif (func == TypeVar.DOUBLEVECTOR):
            return self.half_vector()
        elif (func == TypeVar.TOBITVEC):
            new = TypeSet()

            # Start with all possible lanes/ints/floats/bools
            lanes = interval_to_set(decode_interval(True, (1, MAX_LANES), 1))
            ints = interval_to_set(decode_interval(True, (8, MAX_BITS)))
            floats = interval_to_set(decode_interval(True, (32, 64)))
            bools = interval_to_set(decode_interval(True, (1, MAX_BITS)))

            # See which combinations have a size that appears in self.bitvecs
            has_t = set()  # type: Set[Tuple[str, int, int]]
            for l in lanes:
                for i in ints:
                    if i * l in self.bitvecs:
                        has_t.add(('i', i, l))
                for i in bools:
                    if i * l in self.bitvecs:
                        has_t.add(('b', i, l))
                for i in floats:
                    if i * l in self.bitvecs:
                        has_t.add(('f', i, l))

            for (t, width, lane) in has_t:
                new.lanes.add(lane)
                if (t == 'i'):
                    new.ints.add(width)
                elif (t == 'b'):
                    new.bools.add(width)
                else:
                    assert t == 'f'
                    new.floats.add(width)

            return new
        else:
            assert False, "Unknown derived function: " + func

    def size(self):
        # type: () -> int
        """
        Return the number of concrete types represented by this typeset
        """
        return (len(self.lanes) * (len(self.ints) + len(self.floats) +
                                   len(self.bools) + len(self.bitvecs)) +
                len(self.specials))

    def concrete_types(self):
        # type: () -> Iterable[types.ValueType]
        def by(scalar, lanes):
            # type: (types.LaneType, int) -> types.ValueType
            if (lanes == 1):
                return scalar
            else:
                return scalar.by(lanes)

        for nlanes in self.lanes:
            for bits in self.ints:
                yield by(types.IntType.with_bits(bits), nlanes)
            for bits in self.floats:
                yield by(types.FloatType.with_bits(bits), nlanes)
            for bits in self.bools:
                yield by(types.BoolType.with_bits(bits), nlanes)
            for bits in self.bitvecs:
                assert nlanes == 1
                yield types.BVType.with_bits(bits)

        for spec in self.specials:
            yield spec

    def get_singleton(self):
        # type: () -> types.ValueType
        """
        Return the singleton type represented by self. Can only call on
        typesets containing 1 type.
        """
        types = list(self.concrete_types())
        assert len(types) == 1
        return types[0]

    def widths(self):
        # type: () -> Set[int]
        """ Return a set of the widths of all possible types in self"""
        scalar_w = self.ints.union(self.floats.union(self.bools))
        scalar_w = scalar_w.union(self.bitvecs)
        return set(w * l for l in self.lanes for w in scalar_w)


class TypeVar(object):
    """
    Type variables can be used in place of concrete types when defining
    instructions. This makes the instructions *polymorphic*.

    A type variable is restricted to vary over a subset of the value types.
    This subset is specified by a set of flags that control the permitted base
    types and whether the type variable can assume scalar or vector types, or
    both.

    :param name: Short name of type variable used in instruction descriptions.
    :param doc: Documentation string.
    :param ints: Allow all integer base types, or `(min, max)` bit-range.
    :param floats: Allow all floating point base types, or `(min, max)`
                   bit-range.
    :param bools: Allow all boolean base types, or `(min, max)` bit-range.
    :param scalars: Allow type variable to assume scalar types.
    :param simd: Allow type variable to assume vector types, or `(min, max)`
                 lane count range.
    :param bitvecs: Allow all BitVec base types, or `(min, max)` bit-range.
    """

    def __init__(
            self,
            name,                   # type: str
            doc,                    # type: str
            ints=False,             # type: BoolInterval
            floats=False,           # type: BoolInterval
            bools=False,            # type: BoolInterval
            scalars=True,           # type: bool
            simd=False,             # type: BoolInterval
            bitvecs=False,          # type: BoolInterval
            base=None,              # type: TypeVar
            derived_func=None,      # type: str
            specials=None           # type: SpecialSpec
            ):
        # type: (...) -> None
        self.name = name
        self.__doc__ = doc
        self.is_derived = isinstance(base, TypeVar)
        if base:
            assert self.is_derived
            assert derived_func
            self.base = base
            self.derived_func = derived_func
            self.name = '{}({})'.format(derived_func, base.name)
        else:
            min_lanes = 1 if scalars else 2
            lanes = decode_interval(simd, (min_lanes, MAX_LANES), 1)
            self.type_set = TypeSet(
                    lanes=lanes,
                    ints=ints,
                    floats=floats,
                    bools=bools,
                    bitvecs=bitvecs,
                    specials=specials)

    @staticmethod
    def singleton(typ):
        # type: (types.ValueType) -> TypeVar
        """Create a type variable that can only assume a single type."""
        scalar = None  # type: types.ValueType
        if isinstance(typ, types.VectorType):
            scalar = typ.base
            lanes = (typ.lanes, typ.lanes)
        elif isinstance(typ, types.LaneType):
            scalar = typ
            lanes = (1, 1)
        elif isinstance(typ, types.SpecialType):
            return TypeVar(typ.name, typ.__doc__, specials=[typ])
        else:
            assert isinstance(typ, types.BVType)
            scalar = typ
            lanes = (1, 1)

        ints = None
        floats = None
        bools = None
        bitvecs = None

        if isinstance(scalar, types.IntType):
            ints = (scalar.bits, scalar.bits)
        elif isinstance(scalar, types.FloatType):
            floats = (scalar.bits, scalar.bits)
        elif isinstance(scalar, types.BoolType):
            bools = (scalar.bits, scalar.bits)
        elif isinstance(scalar, types.BVType):
            bitvecs = (scalar.bits, scalar.bits)

        tv = TypeVar(
                typ.name, typ.__doc__,
                ints=ints, floats=floats, bools=bools,
                bitvecs=bitvecs, simd=lanes)
        return tv

    def __str__(self):
        # type: () -> str
        return "`{}`".format(self.name)

    def __repr__(self):
        # type: () -> str
        if self.is_derived:
            return (
                    'TypeVar({}, base={}, derived_func={})'
                    .format(self.name, self.base, self.derived_func))
        else:
            return (
                    'TypeVar({}, {})'
                    .format(self.name, self.type_set))

    def __hash__(self):
        # type: () -> int
        if (not self.is_derived):
            return object.__hash__(self)

        return hash((self.derived_func, self.base))

    def __eq__(self, other):
        # type: (object) -> bool
        if not isinstance(other, TypeVar):
            return False
        if self.is_derived and other.is_derived:
            return (
                    self.derived_func == other.derived_func and
                    self.base == other.base)
        else:
            return self is other

    def __ne__(self, other):
        # type: (object) -> bool
        return not self.__eq__(other)

    # Supported functions for derived type variables.
    # The names here must match the method names on `ir::types::Type`.
    # The camel_case of the names must match `enum OperandConstraint` in
    # `instructions.rs`.
    LANEOF = 'lane_of'
    ASBOOL = 'as_bool'
    HALFWIDTH = 'half_width'
    DOUBLEWIDTH = 'double_width'
    HALFVECTOR = 'half_vector'
    DOUBLEVECTOR = 'double_vector'
    TOBITVEC = 'to_bitvec'

    @staticmethod
    def is_bijection(func):
        # type: (str) -> bool
        return func in [
            TypeVar.HALFWIDTH,
            TypeVar.DOUBLEWIDTH,
            TypeVar.HALFVECTOR,
            TypeVar.DOUBLEVECTOR]

    @staticmethod
    def inverse_func(func):
        # type: (str) -> str
        return {
            TypeVar.HALFWIDTH: TypeVar.DOUBLEWIDTH,
            TypeVar.DOUBLEWIDTH: TypeVar.HALFWIDTH,
            TypeVar.HALFVECTOR: TypeVar.DOUBLEVECTOR,
            TypeVar.DOUBLEVECTOR: TypeVar.HALFVECTOR
        }[func]

    @staticmethod
    def derived(base, derived_func):
        # type: (TypeVar, str) -> TypeVar
        """Create a type variable that is a function of another."""

        # Safety checks to avoid over/underflows.
        ts = base.get_typeset()

        assert len(ts.specials) == 0, "Can't derive from special types"

        if derived_func == TypeVar.HALFWIDTH:
            if len(ts.ints) > 0:
                assert min(ts.ints) > 8, "Can't halve all integer types"
            if len(ts.floats) > 0:
                assert min(ts.floats) > 32, "Can't halve all float types"
            if len(ts.bools) > 0:
                assert min(ts.bools) > 8, "Can't halve all boolean types"
        elif derived_func == TypeVar.DOUBLEWIDTH:
            if len(ts.ints) > 0:
                assert max(ts.ints) < MAX_BITS,\
                    "Can't double all integer types."
            if len(ts.floats) > 0:
                assert max(ts.floats) < MAX_BITS,\
                    "Can't double all float types."
            if len(ts.bools) > 0:
                assert max(ts.bools) < MAX_BITS, "Can't double all bool types."
        elif derived_func == TypeVar.HALFVECTOR:
            assert min(ts.lanes) > 1, "Can't halve a scalar type"
        elif derived_func == TypeVar.DOUBLEVECTOR:
            assert max(ts.lanes) < MAX_LANES, "Can't double 256 lanes."

        return TypeVar(None, None, base=base, derived_func=derived_func)

    @staticmethod
    def from_typeset(ts):
        # type: (TypeSet) -> TypeVar
        """ Create a type variable from a type set."""
        tv = TypeVar(None, None)
        tv.type_set = ts
        return tv

    def lane_of(self):
        # type: () -> TypeVar
        """
        Return a derived type variable that is the scalar lane type of this
        type variable.

        When this type variable assumes a scalar type, the derived type will be
        the same scalar type.
        """
        return TypeVar.derived(self, self.LANEOF)

    def as_bool(self):
        # type: () -> TypeVar
        """
        Return a derived type variable that has the same vector geometry as
        this type variable, but with boolean lanes. Scalar types map to `b1`.
        """
        return TypeVar.derived(self, self.ASBOOL)

    def half_width(self):
        # type: () -> TypeVar
        """
        Return a derived type variable that has the same number of vector lanes
        as this one, but the lanes are half the width.
        """
        return TypeVar.derived(self, self.HALFWIDTH)

    def double_width(self):
        # type: () -> TypeVar
        """
        Return a derived type variable that has the same number of vector lanes
        as this one, but the lanes are double the width.
        """
        return TypeVar.derived(self, self.DOUBLEWIDTH)

    def half_vector(self):
        # type: () -> TypeVar
        """
        Return a derived type variable that has half the number of vector lanes
        as this one, with the same lane type.
        """
        return TypeVar.derived(self, self.HALFVECTOR)

    def double_vector(self):
        # type: () -> TypeVar
        """
        Return a derived type variable that has twice the number of vector
        lanes as this one, with the same lane type.
        """
        return TypeVar.derived(self, self.DOUBLEVECTOR)

    def to_bitvec(self):
        # type: () -> TypeVar
        """
        Return a derived type variable that represent a flat bitvector with
        the same size as self
        """
        return TypeVar.derived(self, self.TOBITVEC)

    def singleton_type(self):
        # type: () -> types.ValueType
        """
        If the associated typeset has a single type return it. Otherwise return
        None
        """
        ts = self.get_typeset()
        if ts.size() != 1:
            return None

        return ts.get_singleton()

    def free_typevar(self):
        # type: () -> TypeVar
        """
        Get the free type variable controlling this one.
        """
        if self.is_derived:
            return self.base.free_typevar()
        elif self.singleton_type() is not None:
            # A singleton type variable is not a proper free variable.
            return None
        else:
            return self

    def rust_expr(self):
        # type: () -> str
        """
        Get a Rust expression that computes the type of this type variable.
        """
        if self.is_derived:
            return '{}.{}()'.format(
                    self.base.rust_expr(), self.derived_func)
        elif self.singleton_type():
            return self.singleton_type().rust_name()
        else:
            return self.name

    def constrain_types_by_ts(self, ts):
        # type: (TypeSet) -> None
        """
        Constrain the range of types this variable can assume to a subset of
        those in the typeset ts.
        """
        if not self.is_derived:
            self.type_set &= ts
        else:
            self.base.constrain_types_by_ts(ts.preimage(self.derived_func))

    def constrain_types(self, other):
        # type: (TypeVar) -> None
        """
        Constrain the range of types this variable can assume to a subset of
        those `other` can assume.
        """
        if self is other:
            return

        self.constrain_types_by_ts(other.get_typeset())

    def get_typeset(self):
        # type: () -> TypeSet
        """
        Returns the typeset for this TV. If the TV is derived, computes it
        recursively from the derived function and the base's typeset.
        """
        if not self.is_derived:
            return self.type_set
        else:
            return self.base.get_typeset().image(self.derived_func)

    def get_fresh_copy(self, name):
        # type: (str) -> TypeVar
        """
        Get a fresh copy of self. Can only be called on free typevars.
        """
        assert not self.is_derived
        tv = TypeVar.from_typeset(self.type_set.copy())
        tv.name = name
        return tv

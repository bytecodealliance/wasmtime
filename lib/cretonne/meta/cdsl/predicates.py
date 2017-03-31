"""
Cretonne predicates.

A *predicate* is a function that computes a boolean result. The inputs to the
function determine the kind of predicate:

- An *ISA predicate* is evaluated on the current ISA settings together with the
  shared settings defined in the :py:mod:`settings` module. Once a target ISA
  has been configured, the value of all ISA predicates is known.

- An *Instruction predicate* is evaluated on an instruction instance, so it can
  inspect all the immediate fields and type variables of the instruction.
  Instruction predicates can be evaluated before register allocation, so they
  can not depend on specific register assignments to the value operands or
  outputs.

Predicates can also be computed from other predicates using the `And`, `Or`,
and `Not` combinators defined in this module.

All predicates have a *context* which determines where they can be evaluated.
For an ISA predicate, the context is the ISA settings group. For an instruction
predicate, the context is the instruction format.
"""
from __future__ import absolute_import
from functools import reduce

try:
    from typing import Sequence, Tuple, Set, Any, Union, TYPE_CHECKING  # noqa
    if TYPE_CHECKING:
        from .formats import InstructionFormat, FormatField  # noqa
        from .settings import BoolSetting, SettingGroup  # noqa
        PredContext = Union[SettingGroup, InstructionFormat]
        PredLeaf = Union[BoolSetting, 'FieldPredicate']
        PredNode = Union[PredLeaf, 'Predicate']
except ImportError:
    pass


def _is_parent(a, b):
    # type: (PredContext, PredContext) -> bool
    """
    Return true if a is a parent of b, or equal to it.
    """
    while b and a is not b:
        b = getattr(b, 'parent', None)
    return a is b


def _descendant(a, b):
    # type: (PredContext, PredContext) -> PredContext
    """
    If a is a parent of b or b is a parent of a, return the descendant of the
    two.

    If neiher is a parent of the other, return None.
    """
    if _is_parent(a, b):
        return b
    if _is_parent(b, a):
        return a
    return None


class Predicate(object):
    """
    Superclass for all computed predicates.

    Leaf predicates can have other types, such as `Setting`.

    :param parts: Tuple of components in the predicate expression.
    """

    def __init__(self, parts):
        # type: (Sequence[PredNode]) -> None
        self.name = None  # type: str
        self.number = None  # type: int
        self.parts = parts
        self.context = reduce(
                _descendant,
                (p.predicate_context() for p in parts))
        assert self.context, "Incompatible predicate parts"

    def __str__(self):
        # type: () -> str
        if self.name:
            return '{}.{}'.format(self.context.name, self.name)
        else:
            return '{}({})'.format(
                    type(self).__name__,
                    ', '.join(map(str, self.parts)))

    def predicate_context(self):
        # type: () -> PredContext
        return self.context

    def predicate_leafs(self, leafs):
        # type: (Set[PredLeaf]) -> None
        """
        Collect all leaf predicates into the `leafs` set.
        """
        for part in self.parts:
            part.predicate_leafs(leafs)

    def rust_predicate(self, prec):
        # type: (int) -> str
        raise NotImplementedError("rust_predicate is an abstract method")


class And(Predicate):
    """
    Computed predicate that is true if all parts are true.
    """

    precedence = 2

    def __init__(self, *args):
        # type: (*PredNode) -> None
        super(And, self).__init__(args)

    def rust_predicate(self, prec):
        # type: (int) -> str
        """
        Return a Rust expression computing the value of this predicate.

        The surrounding precedence determines whether parentheses are needed:

        0. An `if` statement.
        1. An `||` expression.
        2. An `&&` expression.
        3. A `!` expression.
        """
        s = ' && '.join(p.rust_predicate(And.precedence) for p in self.parts)
        if prec > And.precedence:
            s = '({})'.format(s)
        return s

    @staticmethod
    def combine(*args):
        # type: (*PredNode) -> PredNode
        """
        Combine a sequence of predicates, allowing for `None` members.

        Return a predicate that is true when all non-`None` arguments are true,
        or `None` if all of the arguments are `None`.
        """
        args = tuple(p for p in args if p)
        if args == ():
            return None
        if len(args) == 1:
            return args[0]
        # We have multiple predicate args. Combine with `And`.
        return And(*args)


class Or(Predicate):
    """
    Computed predicate that is true if any parts are true.
    """

    precedence = 1

    def __init__(self, *args):
        # type: (*PredNode) -> None
        super(Or, self).__init__(args)

    def rust_predicate(self, prec):
        # type: (int) -> str
        s = ' || '.join(p.rust_predicate(Or.precedence) for p in self.parts)
        if prec > Or.precedence:
            s = '({})'.format(s)
        return s


class Not(Predicate):
    """
    Computed predicate that is true if its single part is false.
    """

    precedence = 3

    def __init__(self, part):
        # type: (PredNode) -> None
        super(Not, self).__init__((part,))

    def rust_predicate(self, prec):
        # type: (int) -> str
        return '!' + self.parts[0].rust_predicate(Not.precedence)


class FieldPredicate(object):
    """
    An instruction predicate that performs a test on a single `FormatField`.

    :param field: The `FormatField` to be tested.
    :param function: Boolean predicate function to call.
    :param args: Additional arguments for the predicate function.
    """

    def __init__(self, field, function, args):
        # type: (FormatField, str, Sequence[Any]) -> None
        self.number = None  # type: int
        self.field = field
        self.function = function
        self.args = args
        # All PredNode members must have a name field. This will never be set.
        self.name = None  # type: str

    def __str__(self):
        # type: () -> str
        args = (self.field.rust_name(),) + tuple(map(str, self.args))
        return '{}({})'.format(self.function, ', '.join(args))

    def predicate_context(self):
        # type: () -> PredContext
        """
        This predicate can be evaluated in the context of an instruction
        format.
        """
        return self.field.format

    def predicate_leafs(self, leafs):
        # type: (Set[PredLeaf]) -> None
        leafs.add(self)

    def rust_predicate(self, prec):
        # type: (int) -> str
        """
        Return a string of Rust code that evaluates this predicate.
        """
        # Prepend `field` to the predicate function arguments.
        args = (self.field.rust_name(),) + tuple(map(str, self.args))
        return 'predicates::{}({})'.format(self.function, ', '.join(args))


class IsEqual(FieldPredicate):
    """
    Instruction predicate that checks if an immediate instruction format field
    is equal to a constant value.

    :param field: `FormatField` to be checked.
    :param value: The constant value to compare against.
    """

    def __init__(self, field, value):
        # type: (FormatField, Any) -> None
        super(IsEqual, self).__init__(field, 'is_equal', (value,))
        self.value = value


class IsSignedInt(FieldPredicate):
    """
    Instruction predicate that checks if an immediate instruction format field
    is representable as an n-bit two's complement integer.

    :param field: `FormatField` to be checked.
    :param width: Number of bits in the allowed range.
    :param scale: Number of low bits that must be 0.

    The predicate is true if the field is in the range:
    `-2^(width-1) -- 2^(width-1)-1`
    and a multiple of `2^scale`.
    """

    def __init__(self, field, width, scale=0):
        # type: (FormatField, int, int) -> None
        super(IsSignedInt, self).__init__(
                field, 'is_signed_int', (width, scale))
        self.width = width
        self.scale = scale
        assert width >= 0 and width <= 64
        assert scale >= 0 and scale < width


class IsUnsignedInt(FieldPredicate):
    """
    Instruction predicate that checks if an immediate instruction format field
    is representable as an n-bit unsigned complement integer.

    :param field: `FormatField` to be checked.
    :param width: Number of bits in the allowed range.
    :param scale: Number of low bits that must be 0.

    The predicate is true if the field is in the range:
    `0 -- 2^width - 1` and a multiple of `2^scale`.
    """

    def __init__(self, field, width, scale=0):
        # type: (FormatField, int, int) -> None
        super(IsUnsignedInt, self).__init__(
                field, 'is_unsigned_int', (width, scale))
        self.width = width
        self.scale = scale
        assert width >= 0 and width <= 64
        assert scale >= 0 and scale < width

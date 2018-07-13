"""
Cranelift predicates.

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
from .formats import instruction_context

try:
    from typing import Sequence, Tuple, Set, Any, Union, TYPE_CHECKING  # noqa
    if TYPE_CHECKING:
        from .formats import InstructionFormat, InstructionContext, FormatField  # noqa
        from .instructions import Instruction  # noqa
        from .settings import BoolSetting, SettingGroup  # noqa
        from .types import ValueType  # noqa
        from .typevar import TypeVar  # noqa
        PredContext = Union[SettingGroup, InstructionFormat,
                            InstructionContext]
        PredLeaf = Union[BoolSetting, 'FieldPredicate', 'TypePredicate',
                         'CtrlTypePredicate']
        PredNode = Union[PredLeaf, 'Predicate']
        # A predicate key is a (recursive) tuple of primitive types that
        # uniquely describes a predicate. It is used for interning.
        PredKey = Tuple[Any, ...]
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

    If neither is a parent of the other, return None.
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
        self.parts = parts
        self.context = reduce(
                _descendant,
                (p.predicate_context() for p in parts))
        assert self.context, "Incompatible predicate parts"
        self.predkey = None  # type: PredKey

    def __str__(self):
        # type: () -> str
        return '{}({})'.format(type(self).__name__,
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

    def predicate_key(self):
        # type: () -> PredKey
        """Tuple uniquely identifying a predicate."""
        if not self.predkey:
            p = tuple(p.predicate_key() for p in self.parts)  # type: PredKey
            self.predkey = (type(self).__name__,) + p
        return self.predkey


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
        self.field = field
        self.function = function
        self.args = args

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
        iform = self.field.format  # type: InstructionFormat
        return iform

    def predicate_key(self):
        # type: () -> PredKey
        a = tuple(map(str, self.args))
        return (self.function, str(self.field)) + a

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


class IsZero32BitFloat(FieldPredicate):
    """
    Instruction predicate that checks if an immediate instruction format field
    is equal to zero.

    :param field: `FormatField` to be checked.
    :param value: The constant value to check.
    """

    def __init__(self, field):
        # type: (FormatField) -> None
        super(IsZero32BitFloat, self).__init__(field,
                                               'is_zero_32_bit_float',
                                               ())


class IsZero64BitFloat(FieldPredicate):
    """
    Instruction predicate that checks if an immediate instruction format field
    is equal to zero.

    :param field: `FormatField` to be checked.
    :param value: The constant value to check.
    """

    def __init__(self, field):
        # type: (FormatField) -> None
        super(IsZero64BitFloat, self).__init__(field,
                                               'is_zero_64_bit_float',
                                               ())


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


class TypePredicate(object):
    """
    An instruction predicate that checks the type of an SSA argument value.

    Type predicates are used to implement encodings for instructions with
    multiple type variables. The encoding tables are keyed by the controlling
    type variable, type predicates check any secondary type variables.

    A type predicate is not bound to any specific instruction format.

    :param value_arg: Index of the value argument to type check.
    :param value_type: The required value type.
    """

    def __init__(self, value_arg, value_type):
        # type: (int, ValueType) -> None
        assert value_arg >= 0
        assert value_type is not None
        self.value_arg = value_arg
        self.value_type = value_type

    def __str__(self):
        # type: () -> str
        return 'args[{}]:{}'.format(self.value_arg, self.value_type)

    def predicate_context(self):
        # type: () -> PredContext
        return instruction_context

    def predicate_key(self):
        # type: () -> PredKey
        return ('typecheck', self.value_arg, self.value_type.name)

    def predicate_leafs(self, leafs):
        # type: (Set[PredLeaf]) -> None
        leafs.add(self)

    @staticmethod
    def typevar_check(inst, typevar, value_type):
        # type: (Instruction, TypeVar, ValueType) -> TypePredicate
        """
        Return a type check predicate for the given type variable in `inst`.

        The type variable must appear directly as the type of one of the
        operands to `inst`, so this is only guaranteed to work for secondary
        type variables.

        Find an `inst` value operand whose type is determined by `typevar` and
        create a `TypePredicate` that checks that the type variable has the
        value `value_type`.
        """
        # Find the first value operand whose type is `typevar`.
        value_arg = next(i for i, opnum in enumerate(inst.value_opnums)
                         if inst.ins[opnum].typevar == typevar)
        return TypePredicate(value_arg, value_type)

    def rust_predicate(self, prec):
        # type: (int) -> str
        """
        Return Rust code for evaluating this predicate.

        It is assumed that the context has `func` and `args` variables.
        """
        return 'func.dfg.value_type(args[{}]) == {}'.format(
                self.value_arg, self.value_type.rust_name())


class CtrlTypePredicate(object):
    """
    An instruction predicate that checks the controlling type variable

    :param value_type: The required value type.
    """

    def __init__(self, value_type):
        # type: (ValueType) -> None
        assert value_type is not None
        self.value_type = value_type

    def __str__(self):
        # type: () -> str
        return 'ctrl_typevar:{}'.format(self.value_type)

    def predicate_context(self):
        # type: () -> PredContext
        return instruction_context

    def predicate_key(self):
        # type: () -> PredKey
        return ('ctrltypecheck', self.value_type.name)

    def predicate_leafs(self, leafs):
        # type: (Set[PredLeaf]) -> None
        leafs.add(self)

    def rust_predicate(self, prec):
        # type: (int) -> str
        """
        Return Rust code for evaluating this predicate.

        It is assumed that the context has `func` and `inst` variables.
        """
        return 'func.dfg.ctrl_typevar(inst) == {}'.format(
                self.value_type.rust_name())

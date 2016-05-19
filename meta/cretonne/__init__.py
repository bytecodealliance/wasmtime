"""
Cretonne meta language module.

This module provides classes and functions used to describe Cretonne
instructions.
"""

import re
import importlib


camel_re = re.compile('(^|_)([a-z])')


def camel_case(s):
    """Convert the string s to CamelCase"""
    return camel_re.sub(lambda m: m.group(2).upper(), s)


# Kinds of operands.
#
# Each instruction has an opcode and a number of operands. The opcode
# determines the instruction format, and the format determines the number of
# operands and the kind of each operand.
class OperandKind(object):
    """
    An instance of the `OperandKind` class corresponds to a kind of operand.
    Each operand kind has a corresponding type in the Rust representation of an
    instruction.
    """

    def __init__(self, name, doc):
        self.name = name
        self.__doc__ = doc
        # The camel-cased name of an operand kind is also the Rust type used to
        # represent it.
        self.camel_name = camel_case(name)

    def __str__(self):
        return self.name

    def __repr__(self):
        return 'OperandKind({})'.format(self.name)

    def operand_kind(self):
        """
        An `OperandKind` instance can be used directly as the type of an
        `Operand` when defining an instruction.
        """
        return self

#: An SSA value operand. This is a value defined by another instruction.
value = OperandKind(
        'value', """
        An SSA value defined by another instruction.

        This kind of operand can represent any SSA value type, but the
        instruction format may restrict the valid value types for a given
        operand.
        """)

#: A variable-sized list of value operands. Use for Ebb and function call
#: arguments.
variable_args = OperandKind(
        'variable_args', """
        A variable size list of `value` operands.

        Use this to represent arguemtns passed to a function call, arguments
        passed to an extended basic block, or a variable number of results
        returned from an instruction.
        """)


# Instances of immediate operand types are provided in the
# `cretonne.immediates` module.
class ImmediateKind(OperandKind):
    """
    The kind of an immediate instruction operand.
    """

    def __init__(self, name, doc):
        self.name = name
        self.__doc__ = doc

    def __repr__(self):
        return 'ImmediateKind({})'.format(self.name)


# Instances of entity reference operand types are provided in the
# `cretonne.entities` module.
class EntityRefKind(OperandKind):
    """
    The kind of an entity reference instruction operand.
    """

    def __init__(self, name, doc):
        self.name = name
        self.__doc__ = doc

    def __repr__(self):
        return 'EntityRefKind({})'.format(self.name)


# ValueType instances (i8, i32, ...) are provided in the cretonne.types module.
class ValueType(object):
    """
    A concrete SSA value type.

    All SSA values have a type that is described by an instance of `ValueType`
    or one of its subclasses.
    """

    def __init__(self, name, membytes, doc):
        self.name = name
        self.membytes = membytes
        self.__doc__ = doc

    def __str__(self):
        return self.name

    def operand_kind(self):
        """
        When a `ValueType` object is used to describe the type of an `Operand`
        in an instruction definition, the kind of that operand is an SSA value.
        """
        return value


class ScalarType(ValueType):
    """
    A concrete scalar (not vector) type.

    Also tracks a unique set of :py:class:`VectorType` instances with this type
    as the lane type.
    """

    def __init__(self, name, membytes, doc):
        super(ScalarType, self).__init__(name, membytes, doc)
        self._vectors = dict()

    def __repr__(self):
        return 'ScalarType({})'.format(self.name)

    def by(self, lanes):
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


class VectorType(ValueType):
    """
    A concrete SIMD vector type.

    A vector type has a lane type which is an instance of :class:`ScalarType`,
    and a positive number of lanes.
    """

    def __init__(self, base, lanes):
        assert isinstance(base, ScalarType), 'SIMD lanes must be scalar types'
        super(VectorType, self).__init__(
                name='{}x{}'.format(base.name, lanes),
                membytes=lanes*base.membytes,
                doc="""
                A SIMD vector with {} lanes containing a {} each.
                """.format(lanes, base.name))
        self.base = base
        self.lanes = lanes

    def __repr__(self):
        return ('VectorType(base={}, lanes={})'
                .format(self.base.name, self.lanes))


class IntType(ScalarType):
    """A concrete scalar integer type."""

    def __init__(self, bits):
        assert bits > 0, 'IntType must have positive number of bits'
        super(IntType, self).__init__(
                name='i{:d}'.format(bits),
                membytes=bits/8,
                doc="An integer type with {} bits.".format(bits))
        self.bits = bits

    def __repr__(self):
        return 'IntType(bits={})'.format(self.bits)


class FloatType(ScalarType):
    """A concrete scalar floating point type."""

    def __init__(self, bits, doc):
        assert bits > 0, 'FloatType must have positive number of bits'
        super(FloatType, self).__init__(name='f{:d}'.format(bits),
                                        membytes=bits/8, doc=doc)
        self.bits = bits

    def __repr__(self):
        return 'FloatType(bits={})'.format(self.bits)


class BoolType(ScalarType):
    """A concrete scalar boolean type."""

    def __init__(self, bits):
        assert bits > 0, 'BoolType must have positive number of bits'
        super(BoolType, self).__init__(
                name='b{:d}'.format(bits),
                membytes=bits/8,
                doc="A boolean type with {} bits.".format(bits))
        self.bits = bits

    def __repr__(self):
        return 'BoolType(bits={})'.format(self.bits)


# Parametric polymorphism.


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
    :param base: Single base type or list of base types. Use this to specify an
        exact set of base types if the general categories below are not good
        enough.
    :param ints: Allow all integer base types.
    :param floats: Allow all floating point base types.
    :param bools: Allow all boolean base types.
    :param scalars: Allow type variable to assume scalar types.
    :param simd: Allow type variable to assume vector types.
    """

    def __init__(
            self, name, doc, base=None,
            ints=False, floats=False, bools=False,
            scalars=True, simd=False):
        self.name = name
        self.__doc__ = doc

    def lane(self):
        """
        Return a derived type variable that is the scalar lane type of this
        type variable.

        When this type variable assumes a scalar type, the derived type will be
        the same scalar type.
        """
        return TypeVar("Lane type of " + self.name, '', base=self, simd=False)

    def as_bool(self):
        """
        Return a derived type variable that has the same vector geometry as
        this type variable, but with boolean lanes. Scalar types map to `b1`.
        """
        return TypeVar(self.name + " as boolean", '', base=self, bools=True)

    def operand_kind(self):
        # When a `TypeVar` object is used to describe the type of an `Operand`
        # in an instruction definition, the kind of that operand is an SSA
        # value.
        return value


# Defining instructions.


class InstructionGroup(object):
    """
    Every instruction must belong to exactly one instruction group. A given
    target architecture can support instructions from multiple groups, and it
    does not necessarily support all instructions in a group.

    New instructions are automatically added to the currently open instruction
    group.
    """

    # The currently open instruction group.
    _current = None

    def open(self):
        """
        Open this instruction group such that future new instructions are
        added to this group.
        """
        assert InstructionGroup._current is None, (
                "Can't open {} since {} is already open"
                .format(self, InstructionGroup._current))
        InstructionGroup._current = self

    def close(self):
        """
        Close this instruction group. This function should be called before
        opening another instruction group.
        """
        assert InstructionGroup._current is self, (
                "Can't close {}, the open instuction group is {}"
                .format(self, InstructionGroup._current))
        InstructionGroup._current = None

    def __init__(self, name, doc):
        self.name = name
        self.__doc__ = doc
        self.instructions = []
        self.open()

    @staticmethod
    def append(inst):
        assert InstructionGroup._current, \
                "Open an instruction group before defining instructions."
        InstructionGroup._current.instructions.append(inst)


class Operand(object):
    """
    An instruction operand can be an *immediate*, an *SSA value*, or an *entity
    reference*. The type of the operand is one of:

    1. A :py:class:`ValueType` instance indicates an SSA value operand with a
       concrete type.

    2. A :py:class:`TypeVar` instance indicates an SSA value operand, and the
       instruction is polymorphic over the possible concrete types that the
       type variable can assume.

    3. An :py:class:`ImmediateKind` instance indicates an immediate operand
       whose value is encoded in the instruction itself rather than being
       passed as an SSA value.

    4. An :py:class:`EntityRefKind` instance indicates an operand that
       references another entity in the function, typically something declared
       in the function preamble.

    """
    def __init__(self, name, typ, doc=''):
        self.name = name
        self.typ = typ
        self.__doc__ = doc
        self.kind = typ.operand_kind()

    def get_doc(self):
        if self.__doc__:
            return self.__doc__
        else:
            return self.typ.__doc__


class InstructionFormat(object):
    """
    Every instruction opcode has a corresponding instruction format which
    determines the number of operands and their kinds. Instruction formats are
    identified structurally, i.e., the format of an instruction is derived from
    the kinds of operands used in its declaration.

    Most instruction formats produce a single result, or no result at all. If
    an instruction can produce more than one result, the `multiple_results`
    flag must be set on its format. All results are of the `value` kind, and
    the instruction format does not keep track of how many results are
    produced. Some instructions, like `call`, may have a variable number of
    results.

    All instruction formats must be predefined in the
    :py:mod:`cretonne.formats` module.

    :param kinds: List of `OperandKind` objects describing the operands.
    :param name: Instruction format name in CamelCase. This is used as a Rust
        variant name in both the `InstructionData` and `InstructionFormat`
        enums.
    :param multiple_results: Set to `True` if this instruction format allows
        more than one result to be produced.
    :param boxed_storage: Set to `True` is this instruction format requires a
        `data: Box<...>` pointer to additional storage in its `InstructionData`
        variant.
    """

    # Map (multiple_results, kind, kind, ...) -> InstructionFormat
    _registry = dict()

    # All existing formats.
    all_formats = list()

    def __init__(self, *kinds, **kwargs):
        self.name = kwargs.get('name', None)
        self.kinds = kinds
        self.multiple_results = kwargs.get('multiple_results', False)
        self.boxed_storage = kwargs.get('boxed_storage', False)
        # Compute a signature for the global registry.
        sig = (self.multiple_results,) + kinds
        if sig in InstructionFormat._registry:
            raise RuntimeError(
                "Format '{}' has the same signature as existing format '{}'"
                .format(self.name, InstructionFormat._registry[sig]))
        InstructionFormat._registry[sig] = self
        InstructionFormat.all_formats.append(self)

    @staticmethod
    def lookup(ins, outs):
        """
        Find an existing instruction format that matches the given lists of
        instruction inputs and outputs.

        The `ins` and `outs` arguments correspond to the
        :py:class:`Instruction` arguments of the same name, except they must be
        tuples of :py:`Operand` objects.
        """
        multiple_results = len(outs) > 1
        sig = (multiple_results,) + tuple(op.kind for op in ins)
        if sig not in InstructionFormat._registry:
            raise RuntimeError(
                    "No instruction format matches ins = ({}){}".format(
                        ", ".join(map(str, sig[1:])),
                        "[multiple results]" if multiple_results else ""))
        return InstructionFormat._registry[sig]

    @staticmethod
    def extract_names(globs):
        """
        Given a dict mapping name -> object as returned by `globals()`, find
        all the InstructionFormat objects and set their name from the dict key.
        This is used to name a bunch of global variables in a module.
        """
        for name, obj in globs.iteritems():
            if isinstance(obj, InstructionFormat):
                assert obj.name is None
                obj.name = name


class Instruction(object):
    """
    The operands to the instruction are specified as two tuples: ``ins`` and
    ``outs``. Since the Python singleton tuple syntax is a bit awkward, it is
    allowed to specify a singleton as just the operand itself, i.e., `ins=x`
    and `ins=(x,)` are both allowed and mean the same thing.

    :param name: Instruction mnemonic, also becomes opcode name.
    :param doc: Documentation string.
    :param ins: Tuple of input operands. This can be a mix of SSA value
                operands and other operand kinds.
    :param outs: Tuple of output operands. The output operands must be SSA
                values.
    :param is_terminator: This is a terminator instruction.
    :param is_branch: This is a branch instruction.
    """

    def __init__(self, name, doc, ins=(), outs=(), **kwargs):
        self.name = name
        self.camel_name = camel_case(name)
        self.__doc__ = doc
        self.ins = self._to_operand_tuple(ins)
        self.outs = self._to_operand_tuple(outs)
        self.format = InstructionFormat.lookup(self.ins, self.outs)
        InstructionGroup.append(self)

    @staticmethod
    def _to_operand_tuple(x):
        # Allow a single Operand instance instead of the awkward singleton
        # tuple syntax.
        if isinstance(x, Operand):
            x = (x,)
        else:
            x = tuple(x)
        for op in x:
            assert isinstance(op, Operand)
        return x


# Defining targets


class Target(object):
    """
    A target instruction set architecture.

    The `Target` class collects everything known about a target ISA.

    :param name: Short mnemonic name for the ISA.
    :param instruction_groups: List of `InstructionGroup` instances that are
        relevant for this ISA.
    """

    def __init__(self, name, instrution_groups):
        self.name = name
        self.instruction_groups = instrution_groups

# Import the fixed instruction formats now so they can be added to the
# registry.
importlib.import_module('cretonne.formats')

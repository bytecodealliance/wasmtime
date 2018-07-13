"""Classes for describing instruction formats."""
from __future__ import absolute_import
from .operands import OperandKind, VALUE, VARIABLE_ARGS
from .operands import Operand  # noqa

# The typing module is only required by mypy, and we don't use these imports
# outside type comments.
try:
    from typing import Dict, List, Tuple, Union, Any, Sequence, Iterable  # noqa
except ImportError:
    pass


class InstructionContext(object):
    """
    Most instruction predicates refer to immediate fields of a specific
    instruction format, so their `predicate_context()` method returns the
    specific instruction format.

    Predicates that only care about the types of SSA values are independent of
    the instruction format. They can be evaluated in the context of any
    instruction.

    The singleton `InstructionContext` class serves as the predicate context
    for these predicates.
    """

    def __init__(self):
        # type: () -> None
        self.name = 'inst'


# Singleton instance.
instruction_context = InstructionContext()


class InstructionFormat(object):
    """
    Every instruction opcode has a corresponding instruction format which
    determines the number of operands and their kinds. Instruction formats are
    identified structurally, i.e., the format of an instruction is derived from
    the kinds of operands used in its declaration.

    The instruction format stores two separate lists of operands: Immediates
    and values. Immediate operands (including entity references) are
    represented as explicit members in the `InstructionData` variants. The
    value operands are stored differently, depending on how many there are.
    Beyond a certain point, instruction formats switch to an external value
    list for storing value arguments. Value lists can hold an arbitrary number
    of values.

    All instruction formats must be predefined in the
    :py:mod:`cranelift.formats` module.

    :param kinds: List of `OperandKind` objects describing the operands.
    :param name: Instruction format name in CamelCase. This is used as a Rust
        variant name in both the `InstructionData` and `InstructionFormat`
        enums.
    :param typevar_operand: Index of the value input operand that is used to
        infer the controlling type variable. By default, this is `0`, the first
        `value` operand. The index is relative to the values only, ignoring
        immediate operands.
    """

    # Map (imm_kinds, num_value_operands) -> format
    _registry = dict()  # type: Dict[Tuple[Tuple[OperandKind, ...], int, bool], InstructionFormat]  # noqa

    # All existing formats.
    all_formats = list()  # type: List[InstructionFormat]

    def __init__(self, *kinds, **kwargs):
        # type: (*Union[OperandKind, Tuple[str, OperandKind]], **Any) -> None # noqa
        self.name = kwargs.get('name', None)  # type: str
        self.parent = instruction_context

        # The number of value operands stored in the format, or `None` when
        # `has_value_list` is set.
        self.num_value_operands = 0
        # Does this format use a value list for storing value operands?
        self.has_value_list = False
        # Operand fields for the immediate operands. All other instruction
        # operands are values or variable argument lists. They are all handled
        # specially.
        self.imm_fields = tuple(self._process_member_names(kinds))

        # The typevar_operand argument must point to a 'value' operand.
        self.typevar_operand = kwargs.get('typevar_operand', None)  # type: int
        if self.typevar_operand is not None:
            if not self.has_value_list:
                assert self.typevar_operand < self.num_value_operands, \
                        "typevar_operand must indicate a 'value' operand"
        elif self.has_value_list or self.num_value_operands > 0:
            # Default to the first 'value' operand, if there is one.
            self.typevar_operand = 0

        # Compute a signature for the global registry.
        imm_kinds = tuple(f.kind for f in self.imm_fields)
        sig = (imm_kinds, self.num_value_operands, self.has_value_list)
        if sig in InstructionFormat._registry:
            raise RuntimeError(
                "Format '{}' has the same signature as existing format '{}'"
                .format(self.name, InstructionFormat._registry[sig]))
        InstructionFormat._registry[sig] = self
        InstructionFormat.all_formats.append(self)

    def args(self):
        # type: () -> FormatField
        """
        Provides a ValueListField, which is derived from FormatField,
        corresponding to the full ValueList of the instruction format. This
        is useful for creating predicates for instructions which use variadic
        arguments.
        """

        if self.has_value_list:
            return ValueListField(self)
        return None

    def _process_member_names(self, kinds):
        # type: (Sequence[Union[OperandKind, Tuple[str, OperandKind]]]) -> Iterable[FormatField]  # noqa
        """
        Extract names of all the immediate operands in the kinds tuple.

        Each entry is either an `OperandKind` instance, or a `(member, kind)`
        pair. The member names correspond to members in the Rust
        `InstructionData` data structure.

        Updates the fields `self.num_value_operands` and `self.has_value_list`.

        Yields the immediate operand fields.
        """
        inum = 0
        for arg in kinds:
            if isinstance(arg, OperandKind):
                member = arg.default_member
                k = arg
            else:
                member, k = arg

            # We define 'immediate' as not a value or variable arguments.
            if k is VALUE:
                self.num_value_operands += 1
            elif k is VARIABLE_ARGS:
                self.has_value_list = True
            else:
                yield FormatField(self, inum, k, member)
                inum += 1

    def __str__(self):
        # type: () -> str
        args = ', '.join(
                '{}: {}'.format(f.member, f.kind) for f in self.imm_fields)
        return '{}(imms=({}), vals={})'.format(
                self.name, args, self.num_value_operands)

    def __getattr__(self, attr):
        # type: (str) -> FormatField
        """
        Make immediate instruction format members available as attributes.

        Each non-value format member becomes a corresponding `FormatField`
        attribute.
        """
        for f in self.imm_fields:
            if f.member == attr:
                # Cache this field attribute so we won't have to search again.
                setattr(self, attr, f)
                return f

        raise AttributeError(
                '{} is neither a {} member or a '
                .format(attr, self.name) +
                'normal InstructionFormat attribute')

    @staticmethod
    def lookup(ins, outs):
        # type: (Sequence[Operand], Sequence[Operand]) -> InstructionFormat
        """
        Find an existing instruction format that matches the given lists of
        instruction inputs and outputs.

        The `ins` and `outs` arguments correspond to the
        :py:class:`Instruction` arguments of the same name, except they must be
        tuples of :py:`Operand` objects.
        """
        # Construct a signature.
        imm_kinds = tuple(op.kind for op in ins if op.is_immediate())
        num_values = sum(1 for op in ins if op.is_value())
        has_varargs = (VARIABLE_ARGS in tuple(op.kind for op in ins))

        sig = (imm_kinds, num_values, has_varargs)
        if sig in InstructionFormat._registry:
            return InstructionFormat._registry[sig]

        # Try another value list format as an alternative.
        sig = (imm_kinds, 0, True)
        if sig in InstructionFormat._registry:
            return InstructionFormat._registry[sig]

        raise RuntimeError(
                'No instruction format matches '
                'imms={}, vals={}, varargs={}'.format(
                    imm_kinds, num_values, has_varargs))

    @staticmethod
    def extract_names(globs):
        # type: (Dict[str, Any]) -> None
        """
        Given a dict mapping name -> object as returned by `globals()`, find
        all the InstructionFormat objects and set their name from the dict key.
        This is used to name a bunch of global values in a module.
        """
        for name, obj in globs.items():
            if isinstance(obj, InstructionFormat):
                assert obj.name is None
                obj.name = name


class FormatField(object):
    """
    An immediate field in an instruction format.

    This corresponds to a single member of a variant of the `InstructionData`
    data type.

    :param iform: Parent `InstructionFormat`.
    :param immnum: Immediate operand number in parent.
    :param kind: Immediate Operand kind.
    :param member: Member name in `InstructionData` variant.
    """

    def __init__(self, iform, immnum, kind, member):
        # type: (InstructionFormat, int, OperandKind, str) -> None
        self.format = iform
        self.immnum = immnum
        self.kind = kind
        self.member = member

    def __str__(self):
        # type: () -> str
        return '{}.{}'.format(self.format.name, self.member)

    def rust_destructuring_name(self):
        # type: () -> str
        return self.member

    def rust_name(self):
        # type: () -> str
        return self.member


class ValueListField(FormatField):
    """
    The full value list field of an instruction format.

    This corresponds to all Value-type members of a variant of the
    `InstructionData` format, which contains a ValueList.

    :param iform: Parent `InstructionFormat`.
    """
    def __init__(self, iform):
        # type: (InstructionFormat) -> None
        self.format = iform
        self.member = "args"

    def rust_destructuring_name(self):
        # type: () -> str
        return 'ref {}'.format(self.member)

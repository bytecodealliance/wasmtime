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
    :param value_list: Set to `True` if this instruction format uses a
        `ValueList` member to store its value operands.
    :param boxed_storage: Set to `True` is this instruction format requires a
        `data: Box<...>` pointer to additional storage in its `InstructionData`
        variant.
    :param typevar_operand: Index of the input operand that is used to infer
        the controlling type variable. By default, this is the first `value`
        operand.
    """

    # Map (multiple_results, kind, kind, ...) -> InstructionFormat
    _registry = dict()  # type: Dict[Tuple[bool, Tuple[OperandKind, ...]], InstructionFormat]  # noqa

    # All existing formats.
    all_formats = list()  # type: List[InstructionFormat]

    def __init__(self, *kinds, **kwargs):
        # type: (*Union[OperandKind, Tuple[str, OperandKind]], **Any) -> None # noqa
        self.name = kwargs.get('name', None)  # type: str
        self.multiple_results = kwargs.get('multiple_results', False)
        self.has_value_list = kwargs.get('value_list', False)
        self.boxed_storage = kwargs.get('boxed_storage', False)
        self.members = list()  # type: List[str]
        self.kinds = tuple(self._process_member_names(kinds))

        # Which of self.kinds are `value`?
        self.value_operands = tuple(
                i for i, k in enumerate(self.kinds) if k is VALUE)

        # The typevar_operand argument must point to a 'value' operand.
        self.typevar_operand = kwargs.get('typevar_operand', None)  # type: int
        if self.typevar_operand is not None:
            assert self.kinds[self.typevar_operand] is VALUE, \
                    "typevar_operand must indicate a 'value' operand"
        elif len(self.value_operands) > 0:
            # Default to the first 'value' operand, if there is one.
            self.typevar_operand = self.value_operands[0]

        # Compute a signature for the global registry.
        sig = (self.multiple_results, self.kinds)
        if sig in InstructionFormat._registry:
            raise RuntimeError(
                "Format '{}' has the same signature as existing format '{}'"
                .format(self.name, InstructionFormat._registry[sig]))
        InstructionFormat._registry[sig] = self
        InstructionFormat.all_formats.append(self)

    def _process_member_names(self, kinds):
        # type: (Sequence[Union[OperandKind, Tuple[str, OperandKind]]]) -> Iterable[OperandKind] # noqa
        """
        Extract names of all the immediate operands in the kinds tuple.

        Each entry is either an `OperandKind` instance, or a `(member, kind)`
        pair. The member names correspond to members in the Rust
        `InstructionData` data structure.

        Yields the operand kinds.
        """
        for arg in kinds:
            if isinstance(arg, OperandKind):
                member = arg.default_member
                k = arg
            else:
                member, k = arg
            self.members.append(member)
            yield k

    def __str__(self):
        # type: () -> str
        args = ', '.join('{}: {}'.format(m, k) if m else str(k)
                         for m, k in zip(self.members, self.kinds))
        return '{}({})'.format(self.name, args)

    def __getattr__(self, attr):
        # type: (str) -> FormatField
        """
        Make instruction format members available as attributes.

        Each non-value format member becomes a corresponding `FormatField`
        attribute.
        """
        try:
            i = self.members.index(attr)
        except ValueError:
            raise AttributeError(
                    '{} is neither a {} member or a '
                    .format(attr, self.name) +
                    'normal InstructionFormat attribute')
        field = FormatField(self, i, attr)
        setattr(self, attr, field)
        return field

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
        if len(outs) == 1:
            multiple_results = outs[0].kind == VARIABLE_ARGS
        else:
            multiple_results = len(outs) > 1
        sig = (multiple_results, tuple(op.kind for op in ins))
        if sig not in InstructionFormat._registry:
            raise RuntimeError(
                    "No instruction format matches ins = ({}){}".format(
                        ", ".join(map(str, sig[1])),
                        "[multiple results]" if multiple_results else ""))
        return InstructionFormat._registry[sig]

    @staticmethod
    def extract_names(globs):
        """
        Given a dict mapping name -> object as returned by `globals()`, find
        all the InstructionFormat objects and set their name from the dict key.
        This is used to name a bunch of global variables in a module.
        """
        for name, obj in globs.items():
            if isinstance(obj, InstructionFormat):
                assert obj.name is None
                obj.name = name


class FormatField(object):
    """
    A field in an instruction format.

    This corresponds to a single member of a variant of the `InstructionData`
    data type.

    :param format: Parent `InstructionFormat`.
    :param operand: Operand number in parent.
    :param name: Member name in `InstructionData` variant.
    """

    def __init__(self, format, operand, name):
        # type: (InstructionFormat, int, str) -> None
        self.format = format
        self.operand = operand
        self.name = name

    def __str__(self):
        # type: () -> str
        return '{}.{}'.format(self.format.name, self.name)

    def rust_name(self):
        # type: () -> str
        if self.format.boxed_storage:
            return 'data.' + self.name
        else:
            return self.name

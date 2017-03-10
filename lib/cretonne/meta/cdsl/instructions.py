"""Classes for defining instructions."""
from __future__ import absolute_import
from . import camel_case
from .types import ValueType
from .operands import Operand
from .formats import InstructionFormat

try:
    from typing import Union, Sequence, List  # noqa
    # List of operands for ins/outs:
    OpList = Union[Sequence[Operand], Operand]
    MaybeBoundInst = Union['Instruction', 'BoundInstruction']
    from typing import Tuple, Any  # noqa
except ImportError:
    pass


class InstructionGroup(object):
    """
    Every instruction must belong to exactly one instruction group. A given
    target architecture can support instructions from multiple groups, and it
    does not necessarily support all instructions in a group.

    New instructions are automatically added to the currently open instruction
    group.
    """

    # The currently open instruction group.
    _current = None  # type: InstructionGroup

    def open(self):
        # type: () -> None
        """
        Open this instruction group such that future new instructions are
        added to this group.
        """
        assert InstructionGroup._current is None, (
                "Can't open {} since {} is already open"
                .format(self, InstructionGroup._current))
        InstructionGroup._current = self

    def close(self):
        # type: () -> None
        """
        Close this instruction group. This function should be called before
        opening another instruction group.
        """
        assert InstructionGroup._current is self, (
                "Can't close {}, the open instuction group is {}"
                .format(self, InstructionGroup._current))
        InstructionGroup._current = None

    def __init__(self, name, doc):
        # type: (str, str) -> None
        self.name = name
        self.__doc__ = doc
        self.instructions = []  # type: List[Instruction]
        self.open()

    @staticmethod
    def append(inst):
        # type: (Instruction) -> None
        assert InstructionGroup._current, \
                "Open an instruction group before defining instructions."
        InstructionGroup._current.instructions.append(inst)


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
                values or `variable_args`.
    :param is_terminator: This is a terminator instruction.
    :param is_branch: This is a branch instruction.
    :param is_call: This is a call instruction.
    :param is_return: This is a return instruction.
    :param can_trap: This instruction can trap.
    """

    # Boolean instruction attributes that can be passed as keyword arguments to
    # the constructor. Map attribute name to doc comment for generated Rust
    # code.
    ATTRIBS = {
            'is_terminator': 'True for instructions that terminate the EBB.',
            'is_branch': 'True for all branch or jump instructions.',
            'is_call': 'Is this a call instruction?',
            'is_return': 'Is this a return instruction?',
            'can_trap': 'Can this instruction cause a trap?',
            }

    def __init__(self, name, doc, ins=(), outs=(), **kwargs):
        # type: (str, str, OpList, OpList, **Any) -> None # noqa
        self.name = name
        self.camel_name = camel_case(name)
        self.__doc__ = doc
        self.ins = self._to_operand_tuple(ins)
        self.outs = self._to_operand_tuple(outs)
        self.format = InstructionFormat.lookup(self.ins, self.outs)

        # Indexes into `self.outs` for value results.
        # Other results are `variable_args`.
        self.value_results = tuple(
                i for i, o in enumerate(self.outs) if o.is_value())
        # Indexes into `self.ins` for value operands.
        self.value_opnums = tuple(
                i for i, o in enumerate(self.ins) if o.is_value())
        # Indexes into `self.ins` for non-value operands.
        self.imm_opnums = tuple(
                i for i, o in enumerate(self.ins) if o.is_immediate())

        self._verify_polymorphic()
        for attr in Instruction.ATTRIBS:
            setattr(self, attr, not not kwargs.get(attr, False))
        InstructionGroup.append(self)

    def __str__(self):
        prefix = ', '.join(o.name for o in self.outs)
        if prefix:
            prefix = prefix + ' = '
        suffix = ', '.join(o.name for o in self.ins)
        return '{}{} {}'.format(prefix, self.name, suffix)

    def snake_name(self):
        # type: () -> str
        """
        Get the snake_case name of this instruction.

        Keywords in Rust and Python are altered by appending a '_'
        """
        if self.name == 'return':
            return 'return_'
        else:
            return self.name

    def blurb(self):
        """Get the first line of the doc comment"""
        for line in self.__doc__.split('\n'):
            line = line.strip()
            if line:
                return line
        return ""

    def _verify_polymorphic(self):
        """
        Check if this instruction is polymorphic, and verify its use of type
        variables.
        """
        poly_ins = [
                i for i in self.value_opnums
                if self.ins[i].typevar.free_typevar()]
        poly_outs = [
                i for i, o in enumerate(self.outs)
                if o.is_value() and o.typevar.free_typevar()]
        self.is_polymorphic = len(poly_ins) > 0 or len(poly_outs) > 0
        if not self.is_polymorphic:
            return

        # Prefer to use the typevar_operand to infer the controlling typevar.
        self.use_typevar_operand = False
        typevar_error = None
        if self.format.typevar_operand is not None:
            try:
                opnum = self.value_opnums[self.format.typevar_operand]
                tv = self.ins[opnum].typevar
                if tv is tv.free_typevar():
                    self.other_typevars = self._verify_ctrl_typevar(tv)
                    self.ctrl_typevar = tv
                    self.use_typevar_operand = True
            except RuntimeError as e:
                typevar_error = e

        if not self.use_typevar_operand:
            # The typevar_operand argument doesn't work. Can we infer from the
            # first result instead?
            if len(self.outs) == 0:
                if typevar_error:
                    raise typevar_error
                else:
                    raise RuntimeError(
                            "typevar_operand must be a free type variable")
            tv = self.outs[0].typevar
            if tv is not tv.free_typevar():
                raise RuntimeError("first result must be a free type variable")
            self.other_typevars = self._verify_ctrl_typevar(tv)
            self.ctrl_typevar = tv

    def _verify_ctrl_typevar(self, ctrl_typevar):
        """
        Verify that the use of TypeVars is consistent with `ctrl_typevar` as
        the controlling type variable.

        All polymorhic inputs must either be derived from `ctrl_typevar` or be
        independent free type variables only used once.

        All polymorphic results must be derived from `ctrl_typevar`.

        Return list of other type variables used, or raise an error.
        """
        other_tvs = []
        # Check value inputs.
        for opnum in self.value_opnums:
            typ = self.ins[opnum].typevar
            tv = typ.free_typevar()
            # Non-polymorphic or derived form ctrl_typevar is OK.
            if tv is None or tv is ctrl_typevar:
                continue
            # No other derived typevars allowed.
            if typ is not tv:
                raise RuntimeError(
                        "{}: type variable {} must be derived from {}"
                        .format(self.ins[opnum], typ.name, ctrl_typevar))
            # Other free type variables can only be used once each.
            if tv in other_tvs:
                raise RuntimeError(
                        "type variable {} can't be used more than once"
                        .format(tv.name))
            other_tvs.append(tv)

        # Check outputs.
        for result in self.outs:
            if not result.is_value():
                continue
            typ = result.typevar
            tv = typ.free_typevar()
            # Non-polymorphic or derived from ctrl_typevar is OK.
            if tv is None or tv is ctrl_typevar:
                continue
            raise RuntimeError(
                    "type variable in output not derived from ctrl_typevar")

        return other_tvs

    @staticmethod
    def _to_operand_tuple(x):
        # type: (Union[Sequence[Operand], Operand]) -> Tuple[Operand, ...]
        # Allow a single Operand instance instead of the awkward singleton
        # tuple syntax.
        if isinstance(x, Operand):
            x = (x,)
        else:
            x = tuple(x)
        for op in x:
            assert isinstance(op, Operand)
        return x

    def bind(self, *args):
        # type: (*ValueType) -> BoundInstruction
        """
        Bind a polymorphic instruction to a concrete list of type variable
        values.
        """
        assert self.is_polymorphic
        return BoundInstruction(self, args)

    def __getattr__(self, name):
        # type: (str) -> BoundInstruction
        """
        Bind a polymorphic instruction to a single type variable with dot
        syntax:

        >>> iadd.i32
        """
        return self.bind(ValueType.by_name(name))

    def fully_bound(self):
        # type: () -> Tuple[Instruction, Tuple[ValueType, ...]]
        """
        Verify that all typevars have been bound, and return a
        `(inst, typevars)` pair.

        This version in `Instruction` itself allows non-polymorphic
        instructions to duck-type as `BoundInstruction`\s.
        """
        assert not self.is_polymorphic, self
        return (self, ())

    def __call__(self, *args):
        """
        Create an `ast.Apply` AST node representing the application of this
        instruction to the arguments.
        """
        from .ast import Apply
        return Apply(self, args)


class BoundInstruction(object):
    """
    A polymorphic `Instruction` bound to concrete type variables.
    """

    def __init__(self, inst, typevars):
        # type: (Instruction, Tuple[ValueType, ...]) -> None
        self.inst = inst
        self.typevars = typevars
        assert len(typevars) <= 1 + len(inst.other_typevars)

    def __str__(self):
        return '.'.join([self.inst.name, ] + list(map(str, self.typevars)))

    def bind(self, *args):
        # type: (*ValueType) -> BoundInstruction
        """
        Bind additional typevars.
        """
        return BoundInstruction(self.inst, self.typevars + args)

    def __getattr__(self, name):
        # type: (str) -> BoundInstruction
        """
        Bind an additional typevar dot syntax:

        >>> uext.i32.i8
        """
        return self.bind(ValueType.by_name(name))

    def fully_bound(self):
        # type: () -> Tuple[Instruction, Tuple[ValueType, ...]]
        """
        Verify that all typevars have been bound, and return a
        `(inst, typevars)` pair.
        """
        if len(self.typevars) < 1 + len(self.inst.other_typevars):
            unb = ', '.join(
                    str(tv) for tv in
                    self.inst.other_typevars[len(self.typevars) - 1:])
            raise AssertionError("Unbound typevar {} in {}".format(unb, self))
        assert len(self.typevars) == 1 + len(self.inst.other_typevars)
        return (self.inst, self.typevars)

    def __call__(self, *args):
        """
        Create an `ast.Apply` AST node representing the application of this
        instruction to the arguments.
        """
        from .ast import Apply
        return Apply(self, args)

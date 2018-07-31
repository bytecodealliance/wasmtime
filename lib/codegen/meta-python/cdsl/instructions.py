"""Classes for defining instructions."""
from __future__ import absolute_import
from . import camel_case
from .types import ValueType
from .operands import Operand
from .formats import InstructionFormat

try:
    from typing import Union, Sequence, List, Tuple, Any, TYPE_CHECKING  # noqa
    from typing import Dict # noqa
    if TYPE_CHECKING:
        from .ast import Expr, Apply, Var, Def, VarAtomMap  # noqa
        from .typevar import TypeVar  # noqa
        from .ti import TypeConstraint  # noqa
        from .xform import XForm, Rtl
        # List of operands for ins/outs:
        OpList = Union[Sequence[Operand], Operand]
        ConstrList = Union[Sequence[TypeConstraint], TypeConstraint]
        MaybeBoundInst = Union['Instruction', 'BoundInstruction']
        InstructionSemantics = Sequence[XForm]
        SemDefCase = Union[Rtl, Tuple[Rtl, Sequence[TypeConstraint]], XForm]
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
    :param constraints: Tuple of instruction-specific TypeConstraints.
    :param is_terminator: This is a terminator instruction.
    :param is_branch: This is a branch instruction.
    :param is_call: This is a call instruction.
    :param is_return: This is a return instruction.
    :param can_trap: This instruction can trap.
    :param can_load: This instruction can load from memory.
    :param can_store: This instruction can store to memory.
    :param other_side_effects: Instruction has other side effects.
    """

    # Boolean instruction attributes that can be passed as keyword arguments to
    # the constructor. Map attribute name to doc comment for generated Rust
    # code.
    ATTRIBS = {
            'is_terminator': 'True for instructions that terminate the EBB.',
            'is_branch': 'True for all branch or jump instructions.',
            'is_call': 'Is this a call instruction?',
            'is_return': 'Is this a return instruction?',
            'can_load': 'Can this instruction read from memory?',
            'can_store': 'Can this instruction write to memory?',
            'can_trap': 'Can this instruction cause a trap?',
            'other_side_effects':
            'Does this instruction have other side effects besides can_*',
            'writes_cpu_flags': 'Does this instruction write to CPU flags?',
            }

    def __init__(self, name, doc, ins=(), outs=(), constraints=(), **kwargs):
        # type: (str, str, OpList, OpList, ConstrList, **Any) -> None
        self.name = name
        self.camel_name = camel_case(name)
        self.__doc__ = doc
        self.ins = self._to_operand_tuple(ins)
        self.outs = self._to_operand_tuple(outs)
        self.constraints = self._to_constraint_tuple(constraints)
        self.format = InstructionFormat.lookup(self.ins, self.outs)
        self.semantics = None  # type: InstructionSemantics

        # Opcode number, assigned by gen_instr.py.
        self.number = None  # type: int

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
        for attr in kwargs:
            if attr not in Instruction.ATTRIBS:
                raise AssertionError(
                        "unknown instruction attribute '" + attr + "'")
        for attr in Instruction.ATTRIBS:
            setattr(self, attr, not not kwargs.get(attr, False))

        # Infer the 'writes_cpu_flags' field value.
        if 'writes_cpu_flags' not in kwargs:
            self.writes_cpu_flags = any(
                out.is_cpu_flags() for out in self.outs)

        InstructionGroup.append(self)

    def __str__(self):
        # type: () -> str
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
        # type: () -> str
        """Get the first line of the doc comment"""
        for line in self.__doc__.split('\n'):
            line = line.strip()
            if line:
                return line
        return ""

    def _verify_polymorphic(self):
        # type: () -> None
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
        tv_op = self.format.typevar_operand
        if tv_op is not None and tv_op < len(self.value_opnums):
            try:
                opnum = self.value_opnums[tv_op]
                tv = self.ins[opnum].typevar
                if tv is tv.free_typevar() or tv.singleton_type() is not None:
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
        # type: (TypeVar) -> List[TypeVar]
        """
        Verify that the use of TypeVars is consistent with `ctrl_typevar` as
        the controlling type variable.

        All polymorhic inputs must either be derived from `ctrl_typevar` or be
        independent free type variables only used once.

        All polymorphic results must be derived from `ctrl_typevar`.

        Return list of other type variables used, or raise an error.
        """
        other_tvs = []  # type: List[TypeVar]
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

    def all_typevars(self):
        # type: () -> List[TypeVar]
        """
        Get a list of all type variables in the instruction.
        """
        if self.is_polymorphic:
            return [self.ctrl_typevar] + self.other_typevars
        else:
            return []

    @staticmethod
    def _to_operand_tuple(x):
        # type: (Union[Sequence[Operand], Operand]) -> Tuple[Operand, ...]
        # Allow a single Operand instance instead of the awkward singleton
        # tuple syntax.
        if isinstance(x, Operand):
            y = (x,)  # type: Tuple[Operand, ...]
        else:
            y = tuple(x)
        for op in y:
            assert isinstance(op, Operand)
        return y

    @staticmethod
    def _to_constraint_tuple(x):
        # type: (ConstrList) -> Tuple[TypeConstraint, ...]
        """
        Allow a single TypeConstraint instance instead of the awkward singleton
        tuple syntax.
        """
        # import placed here to avoid circular dependency
        from .ti import TypeConstraint  # noqa
        if isinstance(x, TypeConstraint):
            y = (x,)  # type: Tuple[TypeConstraint, ...]
        else:
            y = tuple(x)
        for op in y:
            assert isinstance(op, TypeConstraint)
        return y

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
        assert name != 'any', 'Wildcard not allowed for ctrl_typevar'
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
        # type: (*Expr) -> Apply
        """
        Create an `ast.Apply` AST node representing the application of this
        instruction to the arguments.
        """
        from .ast import Apply  # noqa
        return Apply(self, args)

    def set_semantics(self, src, *dsts):
        # type: (Union[Def, Apply], *SemDefCase) -> None
        """Set our semantics."""
        from semantics import verify_semantics
        from .xform import XForm, Rtl

        sem = []  # type: List[XForm]
        for dst in dsts:
            if isinstance(dst, Rtl):
                sem.append(XForm(Rtl(src).copy({}), dst))
            elif isinstance(dst, XForm):
                sem.append(XForm(
                    dst.src.copy({}),
                    dst.dst.copy({}),
                    dst.constraints))
            else:
                assert isinstance(dst, tuple)
                sem.append(XForm(Rtl(src).copy({}), dst[0],
                                 constraints=dst[1]))

        verify_semantics(self, Rtl(src), sem)

        self.semantics = sem


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
        # type: () -> str
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
        if name == 'any':
            # This is a wild card bind represented as a None type variable.
            return self.bind(None)

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
        # type: (*Expr) -> Apply
        """
        Create an `ast.Apply` AST node representing the application of this
        instruction to the arguments.
        """
        from .ast import Apply  # noqa
        return Apply(self, args)

"""
Abstract syntax trees.

This module defines classes that can be used to create abstract syntax trees
for patern matching an rewriting of cranelift instructions.
"""
from __future__ import absolute_import
from . import instructions
from .typevar import TypeVar
from .predicates import IsEqual, And, TypePredicate, CtrlTypePredicate

try:
    from typing import Union, Tuple, Sequence, TYPE_CHECKING, Dict, List  # noqa
    from typing import Optional, Set, Any # noqa
    if TYPE_CHECKING:
        from .operands import ImmediateKind  # noqa
        from .predicates import PredNode  # noqa
        VarAtomMap = Dict["Var", "Atom"]
except ImportError:
    pass


def replace_var(arg, m):
    # type: (Expr, VarAtomMap) -> Expr
    """
    Given a var v return either m[v] or a new variable v' (and remember
    m[v]=v'). Otherwise return the argument unchanged
    """
    if isinstance(arg, Var):
        new_arg = m.get(arg, Var(arg.name))  # type: Atom
        m[arg] = new_arg
        return new_arg
    return arg


class Def(object):
    """
    An AST definition associates a set of variables with the values produced by
    an expression.

    Example:

    >>> from base.instructions import iadd_cout, iconst
    >>> x = Var('x')
    >>> y = Var('y')
    >>> x << iconst(4)
    (Var(x),) << Apply(iconst, (4,))
    >>> (x, y) << iadd_cout(4, 5)
    (Var(x), Var(y)) << Apply(iadd_cout, (4, 5))

    The `<<` operator is used to create variable definitions.

    :param defs: Single variable or tuple of variables to be defined.
    :param expr: Expression generating the values.
    """

    def __init__(self, defs, expr):
        # type: (Union[Var, Tuple[Var, ...]], Apply) -> None
        if not isinstance(defs, tuple):
            self.defs = (defs,)  # type: Tuple[Var, ...]
        else:
            self.defs = defs
        assert isinstance(expr, Apply)
        self.expr = expr

    def __repr__(self):
        # type: () -> str
        return "{} << {!r}".format(self.defs, self.expr)

    def __str__(self):
        # type: () -> str
        if len(self.defs) == 1:
            return "{!s} << {!s}".format(self.defs[0], self.expr)
        else:
            return "({}) << {!s}".format(
                    ', '.join(map(str, self.defs)), self.expr)

    def copy(self, m):
        # type: (VarAtomMap) -> Def
        """
        Return a copy of this Def with vars replaced with fresh variables,
        in accordance with the map m. Update m as neccessary.
        """
        new_expr = self.expr.copy(m)
        new_defs = []  # type: List[Var]
        for v in self.defs:
            new_v = replace_var(v, m)
            assert(isinstance(new_v, Var))
            new_defs.append(new_v)

        return Def(tuple(new_defs), new_expr)

    def definitions(self):
        # type: () -> Set[Var]
        """ Return the set of all Vars that are defined by self"""
        return set(self.defs)

    def uses(self):
        # type: () -> Set[Var]
        """ Return the set of all Vars that are used(read) by self"""
        return set(self.expr.vars())

    def vars(self):
        # type: () -> Set[Var]
        """Return the set of all Vars in self that correspond to SSA values"""
        return self.definitions().union(self.uses())

    def substitution(self, other, s):
        # type: (Def, VarAtomMap) -> Optional[VarAtomMap]
        """
        If the Defs self and other agree structurally, return a variable
        substitution to transform self to other. Otherwise return None. Two
        Defs agree structurally if there exists a Var substitution, that can
        transform one into the other. See Apply.substitution() for more
        details.
        """
        s = self.expr.substitution(other.expr, s)

        if (s is None):
            return s

        assert len(self.defs) == len(other.defs)
        for (self_d, other_d) in zip(self.defs, other.defs):
            assert self_d not in s  # Guaranteed by SSA form
            s[self_d] = other_d

        return s


class Expr(object):
    """
    An AST expression.
    """


class Atom(Expr):
    """
    An Atom in the DSL is either a literal or a Var
    """


class Var(Atom):
    """
    A free variable.

    When variables are used in `XForms` with source and destination patterns,
    they are classified as follows:

    Input values
        Uses in the source pattern with no preceding def. These may appear as
        inputs in the destination pattern too, but no new inputs can be
        introduced.
    Output values
        Variables that are defined in both the source and destination pattern.
        These values may have uses outside the source pattern, and the
        destination pattern must compute the same value.
    Intermediate values
        Values that are defined in the source pattern, but not in the
        destination pattern. These may have uses outside the source pattern, so
        the defining instruction can't be deleted immediately.
    Temporary values
        Values that are defined only in the destination pattern.
    """

    def __init__(self, name, typevar=None):
        # type: (str, TypeVar) -> None
        self.name = name
        # The `Def` defining this variable in a source pattern.
        self.src_def = None  # type: Def
        # The `Def` defining this variable in a destination pattern.
        self.dst_def = None  # type: Def
        # TypeVar representing the type of this variable.
        self.typevar = typevar  # type: TypeVar
        # The original 'typeof(x)' type variable that was created for this Var.
        # This one doesn't change. `self.typevar` above may be changed to
        # another typevar by type inference.
        self.original_typevar = self.typevar  # type: TypeVar

    def __str__(self):
        # type: () -> str
        return self.name

    def __repr__(self):
        # type: () -> str
        s = self.name
        if self.src_def:
            s += ", src"
        if self.dst_def:
            s += ", dst"
        return "Var({})".format(s)

    # Context bits for `set_def` indicating which pattern has defines of this
    # var.
    SRCCTX = 1
    DSTCTX = 2

    def set_def(self, context, d):
        # type: (int, Def) -> None
        """
        Set the `Def` that defines this variable in the given context.

        The `context` must be one of `SRCCTX` or `DSTCTX`
        """
        if context == self.SRCCTX:
            self.src_def = d
        else:
            self.dst_def = d

    def get_def(self, context):
        # type: (int) -> Def
        """
        Get the def of this variable in context.

        The `context` must be one of `SRCCTX` or `DSTCTX`
        """
        if context == self.SRCCTX:
            return self.src_def
        else:
            return self.dst_def

    def is_input(self):
        # type: () -> bool
        """Is this an input value to the src pattern?"""
        return self.src_def is None and self.dst_def is None

    def is_output(self):
        # type: () -> bool
        """Is this an output value, defined in both src and dst patterns?"""
        return self.src_def is not None and self.dst_def is not None

    def is_intermediate(self):
        # type: () -> bool
        """Is this an intermediate value, defined only in the src pattern?"""
        return self.src_def is not None and self.dst_def is None

    def is_temp(self):
        # type: () -> bool
        """Is this a temp value, defined only in the dst pattern?"""
        return self.src_def is None and self.dst_def is not None

    def get_typevar(self):
        # type: () -> TypeVar
        """Get the type variable representing the type of this variable."""
        if not self.typevar:
            # Create a TypeVar allowing all types.
            tv = TypeVar(
                    'typeof_{}'.format(self),
                    'Type of the pattern variable `{}`'.format(self),
                    ints=True, floats=True, bools=True,
                    scalars=True, simd=True, bitvecs=True,
                    specials=True)
            self.original_typevar = tv
            self.typevar = tv
        return self.typevar

    def set_typevar(self, tv):
        # type: (TypeVar) -> None
        self.typevar = tv

    def has_free_typevar(self):
        # type: () -> bool
        """
        Check if this variable has a free type variable.

        If not, the type of this variable is computed from the type of another
        variable.
        """
        if not self.typevar or self.typevar.is_derived:
            return False
        return self.typevar is self.original_typevar

    def rust_type(self):
        # type: () -> str
        """
        Get a Rust expression that computes the type of this variable.

        It is assumed that local variables exist corresponding to the free type
        variables.
        """
        return self.typevar.rust_expr()


class Apply(Expr):
    """
    Apply an instruction to arguments.

    An `Apply` AST expression is created by using function call syntax on
    instructions. This applies to both bound and unbound polymorphic
    instructions:

    >>> from base.instructions import jump, iadd
    >>> jump('next', ())
    Apply(jump, ('next', ()))
    >>> iadd.i32('x', 'y')
    Apply(iadd.i32, ('x', 'y'))

    :param inst: The instruction being applied, an `Instruction` or
                 `BoundInstruction` instance.
    :param args: Tuple of arguments.
    """

    def __init__(self, inst, args):
        # type: (instructions.MaybeBoundInst, Tuple[Expr, ...]) -> None  # noqa
        if isinstance(inst, instructions.BoundInstruction):
            self.inst = inst.inst
            self.typevars = inst.typevars
        else:
            assert isinstance(inst, instructions.Instruction)
            self.inst = inst
            self.typevars = ()
        self.args = args
        assert len(self.inst.ins) == len(args)

        # Check that the kinds of Literals arguments match the expected Operand
        for op_idx in self.inst.imm_opnums:
            arg = self.args[op_idx]
            op = self.inst.ins[op_idx]

            if isinstance(arg, Literal):
                assert arg.kind == op.kind, \
                    "Passing literal {} to field of wrong kind {}."\
                    .format(arg, op.kind)

    def __rlshift__(self, other):
        # type: (Union[Var, Tuple[Var, ...]]) -> Def
        """
        Define variables using `var << expr` or `(v1, v2) << expr`.
        """
        return Def(other, self)

    def instname(self):
        # type: () -> str
        i = self.inst.name
        for t in self.typevars:
            i += '.{}'.format(t)
        return i

    def __repr__(self):
        # type: () -> str
        return "Apply({}, {})".format(self.instname(), self.args)

    def __str__(self):
        # type: () -> str
        args = ', '.join(map(str, self.args))
        return '{}({})'.format(self.instname(), args)

    def rust_builder(self, defs=None):
        # type: (Sequence[Var]) -> str
        """
        Return a Rust Builder method call for instantiating this instruction
        application.

        The `defs` argument should be a list of variables defined by this
        instruction. It is used to construct a result type if necessary.
        """
        args = ', '.join(map(str, self.args))
        # Do we need to pass an explicit type argument?
        if self.inst.is_polymorphic and not self.inst.use_typevar_operand:
            args = defs[0].rust_type() + ', ' + args
        method = self.inst.snake_name()
        return '{}({})'.format(method, args)

    def inst_predicate(self):
        # type: () -> PredNode
        """
        Construct an instruction predicate that verifies the immediate operands
        on this instruction.

        Immediate operands in a source pattern can be either free variables or
        constants like `ConstantInt` and `Enumerator`. We don't currently
        support constraints on free variables, but we may in the future.
        """
        pred = None  # type: PredNode
        iform = self.inst.format

        # Examine all of the immediate operands.
        for ffield, opnum in zip(iform.imm_fields, self.inst.imm_opnums):
            arg = self.args[opnum]

            # Ignore free variables for now. We may add variable predicates
            # later.
            if isinstance(arg, Var):
                continue

            pred = And.combine(pred, IsEqual(ffield, arg))

        # Add checks for any bound secondary type variables.
        # We can't check the controlling type variable this way since it may
        # not appear as the type of an operand.
        if len(self.typevars) > 1:
            for bound_ty, tv in zip(self.typevars[1:],
                                    self.inst.other_typevars):
                if bound_ty is None:
                    continue
                type_chk = TypePredicate.typevar_check(self.inst, tv, bound_ty)
                pred = And.combine(pred, type_chk)

        return pred

    def inst_predicate_with_ctrl_typevar(self):
        # type: () -> PredNode
        """
        Same as `inst_predicate()`, but also check the controlling type
        variable.
        """
        pred = self.inst_predicate()

        if len(self.typevars) > 0:
            bound_ty = self.typevars[0]
            type_chk = None  # type: PredNode
            if bound_ty is not None:
                # Prefer to look at the types of input operands.
                if self.inst.use_typevar_operand:
                    type_chk = TypePredicate.typevar_check(
                            self.inst, self.inst.ctrl_typevar, bound_ty)
                else:
                    type_chk = CtrlTypePredicate(bound_ty)
                pred = And.combine(pred, type_chk)

        return pred

    def copy(self, m):
        # type: (VarAtomMap) -> Apply
        """
        Return a copy of this Expr with vars replaced with fresh variables,
        in accordance with the map m. Update m as neccessary.
        """
        return Apply(self.inst, tuple(map(lambda e: replace_var(e, m),
                                          self.args)))

    def vars(self):
        # type: () -> Set[Var]
        """Return the set of all Vars in self that correspond to SSA values"""
        res = set()
        for i in self.inst.value_opnums:
            arg = self.args[i]
            assert isinstance(arg, Var)
            res.add(arg)
        return res

    def substitution(self, other, s):
        # type: (Apply, VarAtomMap) -> Optional[VarAtomMap]
        """
        If there is a substituion from Var->Atom that converts self to other,
        return it, otherwise return None. Note that this is strictly weaker
        than unification (see TestXForm.test_subst_enum_bad_var_const for
        example).
        """
        if self.inst != other.inst:
            return None

        # Guaranteed by self.inst == other.inst
        assert (len(self.args) == len(other.args))

        for (self_a, other_a) in zip(self.args, other.args):
            assert isinstance(self_a, Atom) and isinstance(other_a, Atom)

            if (isinstance(self_a, Var)):
                if (self_a not in s):
                    s[self_a] = other_a
                else:
                    if (s[self_a] != other_a):
                        return None
            elif isinstance(other_a, Var):
                assert isinstance(self_a, Literal)
                if (other_a not in s):
                    s[other_a] = self_a
                else:
                    if s[other_a] != self_a:
                        return None
            else:
                assert (isinstance(self_a, Literal) and
                        isinstance(other_a, Literal))
                # Guaranteed by self.inst == other.inst
                assert self_a.kind == other_a.kind
                if (self_a.value != other_a.value):
                    return None

        return s


class Literal(Atom):
    """
    Base Class for all literal expressions in the DSL.
    """
    def __init__(self, kind, value):
        # type: (ImmediateKind, Any) -> None
        self.kind = kind
        self.value = value

    def __eq__(self, other):
        # type: (Any) -> bool
        if not isinstance(other, Literal):
            return False

        if self.kind != other.kind:
            return False

        # Can't just compare value here, as comparison Any <> Any returns Any
        return repr(self) == repr(other)

    def __ne__(self, other):
        # type: (Any) -> bool
        return not self.__eq__(other)

    def __repr__(self):
        # type: () -> str
        return '{}.{}'.format(self.kind, self.value)


class ConstantInt(Literal):
    """
    A value of an integer immediate operand.

    Immediate operands like `imm64` or `offset32` can be specified in AST
    expressions using the call syntax: `imm64(5)` which greates a `ConstantInt`
    node.
    """

    def __init__(self, kind, value):
        # type: (ImmediateKind, int) -> None
        super(ConstantInt, self).__init__(kind, value)

    def __str__(self):
        # type: () -> str
        """
        Get the Rust expression form of this constant.
        """
        return str(self.value)


class ConstantBits(Literal):
    """
    A bitwise value of an immediate operand.

    This is used to create bitwise exact floating point constants using
    `ieee32.bits(0x80000000)`.
    """

    def __init__(self, kind, bits):
        # type: (ImmediateKind, int) -> None
        v = '{}::with_bits({:#x})'.format(kind.rust_type, bits)
        super(ConstantBits, self).__init__(kind, v)

    def __str__(self):
        # type: () -> str
        """
        Get the Rust expression form of this constant.
        """
        return str(self.value)


class Enumerator(Literal):
    """
    A value of an enumerated immediate operand.

    Some immediate operand kinds like `intcc` and `floatcc` have an enumerated
    range of values corresponding to a Rust enum type. An `Enumerator` object
    is an AST leaf node representing one of the values.

    :param kind: The enumerated `ImmediateKind` containing the value.
    :param value: The textual IR representation of the value.

    `Enumerator` nodes are not usually created directly. They are created by
    using the dot syntax on immediate kinds: `intcc.ult`.
    """

    def __init__(self, kind, value):
        # type: (ImmediateKind, str) -> None
        super(Enumerator, self).__init__(kind, value)

    def __str__(self):
        # type: () -> str
        """
        Get the Rust expression form of this enumerator.
        """
        return self.kind.rust_enumerator(self.value)

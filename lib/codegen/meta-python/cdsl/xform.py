"""
Instruction transformations.
"""
from __future__ import absolute_import
from .ast import Def, Var, Apply
from .ti import ti_xform, TypeEnv, get_type_env, TypeConstraint
from collections import OrderedDict
from functools import reduce

try:
    from typing import Union, Iterator, Sequence, Iterable, List, Dict  # noqa
    from typing import Optional, Set # noqa
    from .ast import Expr, VarAtomMap  # noqa
    from .isa import TargetISA  # noqa
    from .typevar import TypeVar  # noqa
    from .instructions import ConstrList, Instruction # noqa
    DefApply = Union[Def, Apply]
except ImportError:
    pass


def canonicalize_defapply(node):
    # type: (DefApply) -> Def
    """
    Canonicalize a `Def` or `Apply` node into a `Def`.

    An `Apply` becomes a `Def` with an empty list of defs.
    """
    if isinstance(node, Apply):
        return Def((), node)
    else:
        return node


class Rtl(object):
    """
    Register Transfer Language list.

    An RTL object contains a list of register assignments in the form of `Def`
    objects.

    An RTL list can represent both a source pattern to be matched, or a
    destination pattern to be inserted.
    """

    def __init__(self, *args):
        # type: (*DefApply) -> None
        self.rtl = tuple(map(canonicalize_defapply, args))

    def copy(self, m):
        # type: (VarAtomMap) -> Rtl
        """
        Return a copy of this rtl with all Vars substituted with copies or
        according to m. Update m as neccessary.
        """
        return Rtl(*[d.copy(m) for d in self.rtl])

    def vars(self):
        # type: () -> Set[Var]
        """Return the set of all Vars in self that correspond to SSA values"""
        return reduce(lambda x, y:  x.union(y),
                      [d.vars() for d in self.rtl],
                      set([]))

    def definitions(self):
        # type: () -> Set[Var]
        """ Return the set of all Vars defined in self"""
        return reduce(lambda x, y:  x.union(y),
                      [d.definitions() for d in self.rtl],
                      set([]))

    def free_vars(self):
        # type: () -> Set[Var]
        """Return the set of free Vars corresp. to SSA vals used in self"""
        def flow_f(s, d):
            # type: (Set[Var], Def) -> Set[Var]
            """Compute the change in the set of free vars across a Def"""
            s = s.difference(set(d.defs))
            uses = set(d.expr.args[i] for i in d.expr.inst.value_opnums)
            for v in uses:
                assert isinstance(v, Var)
                s.add(v)

            return s

        return reduce(flow_f, reversed(self.rtl), set([]))

    def substitution(self, other, s):
        # type: (Rtl, VarAtomMap) -> Optional[VarAtomMap]
        """
        If the Rtl self agrees structurally with the Rtl other, return a
        substitution to transform self to other. Two Rtls agree structurally if
        they have the same sequence of Defs, that agree structurally.
        """
        if len(self.rtl) != len(other.rtl):
            return None

        for i in range(len(self.rtl)):
            s = self.rtl[i].substitution(other.rtl[i], s)

            if s is None:
                return None

        return s

    def is_concrete(self):
        # type: (Rtl) -> bool
        """Return True iff every Var in the self has a singleton type."""
        return all(v.get_typevar().singleton_type() is not None
                   for v in self.vars())

    def cleanup_concrete_rtl(self):
        # type: (Rtl) -> None
        """
        Given that there is only 1 possible concrete typing T for self, assign
        a singleton TV with type t=T[v] for each Var v \\in self. Its an error
        to call this on an Rtl with more than 1 possible typing. This modifies
        the Rtl in-place.
        """
        from .ti import ti_rtl, TypeEnv
        # 1) Infer the types of all vars in res
        typenv = get_type_env(ti_rtl(self, TypeEnv()))
        typenv.normalize()
        typenv = typenv.extract()

        # 2) Make sure there is only one possible type assignment
        typings = list(typenv.concrete_typings())
        assert len(typings) == 1
        typing = typings[0]

        # 3) Assign the only possible type to each variable.
        for v in typenv.vars:
            assert typing[v].singleton_type() is not None
            v.set_typevar(typing[v])

    def __str__(self):
        # type: () -> str
        return "\n".join(map(str, self.rtl))


class XForm(object):
    """
    An instruction transformation consists of a source and destination pattern.

    Patterns are expressed in *register transfer language* as tuples of
    `ast.Def` or `ast.Expr` nodes. A pattern may optionally have a sequence of
    TypeConstraints, that additionally limit the set of cases when it applies.

    A legalization pattern must have a source pattern containing only a single
    instruction.

    >>> from base.instructions import iconst, iadd, iadd_imm
    >>> a = Var('a')
    >>> c = Var('c')
    >>> v = Var('v')
    >>> x = Var('x')
    >>> XForm(
    ...     Rtl(c << iconst(v),
    ...         a << iadd(x, c)),
    ...     Rtl(a << iadd_imm(x, v)))
    XForm(inputs=[Var(v), Var(x)], defs=[Var(c, src), Var(a, src, dst)],
      c << iconst(v)
      a << iadd(x, c)
    =>
      a << iadd_imm(x, v)
    )
    """

    def __init__(self, src, dst, constraints=None):
        # type: (Rtl, Rtl, Optional[ConstrList]) -> None
        self.src = src
        self.dst = dst
        # Variables that are inputs to the source pattern.
        self.inputs = list()  # type: List[Var]
        # Variables defined in either src or dst.
        self.defs = list()  # type: List[Var]

        # Rewrite variables in src and dst RTL lists to our own copies.
        # Map name -> private Var.
        symtab = dict()  # type: Dict[str, Var]
        self._rewrite_rtl(src, symtab, Var.SRCCTX)
        num_src_inputs = len(self.inputs)
        self._rewrite_rtl(dst, symtab, Var.DSTCTX)
        # Needed for testing type inference on XForms
        self.symtab = symtab

        # Check for inconsistently used inputs.
        for i in self.inputs:
            if not i.is_input():
                raise AssertionError(
                        "'{}' used as both input and def".format(i))

        # Check for spurious inputs in dst.
        if len(self.inputs) > num_src_inputs:
            raise AssertionError(
                    "extra inputs in dst RTL: {}".format(
                        self.inputs[num_src_inputs:]))

        # Perform type inference and cleanup
        raw_ti = get_type_env(ti_xform(self, TypeEnv()))
        raw_ti.normalize()
        self.ti = raw_ti.extract()

        def interp_tv(tv):
            # type: (TypeVar) -> TypeVar
            """ Convert typevars according to symtab """
            if not tv.name.startswith("typeof_"):
                return tv
            return symtab[tv.name[len("typeof_"):]].get_typevar()

        self.constraints = []  # type: List[TypeConstraint]
        if constraints is not None:
            if isinstance(constraints, TypeConstraint):
                constr_list = [constraints]  # type: Sequence[TypeConstraint]
            else:
                constr_list = constraints

            for c in constr_list:
                type_m = {tv: interp_tv(tv) for tv in c.tvs()}
                inner_c = c.translate(type_m)
                self.constraints.append(inner_c)
                self.ti.add_constraint(inner_c)

        # Sanity: The set of inferred free typevars should be a subset of the
        # TVs corresponding to Vars appearing in src
        free_typevars = set(self.ti.free_typevars())
        src_vars = set(self.inputs).union(
            [x for x in self.defs if not x.is_temp()])
        src_tvs = set([v.get_typevar() for v in src_vars])
        if (not free_typevars.issubset(src_tvs)):
            raise AssertionError(
                "Some free vars don't appear in src - {}"
                .format(free_typevars.difference(src_tvs)))

        # Update the type vars for each Var to their inferred values
        for v in self.inputs + self.defs:
            v.set_typevar(self.ti[v.get_typevar()])

    def __repr__(self):
        # type: () -> str
        s = "XForm(inputs={}, defs={},\n  ".format(self.inputs, self.defs)
        s += '\n  '.join(str(n) for n in self.src.rtl)
        s += '\n=>\n  '
        s += '\n  '.join(str(n) for n in self.dst.rtl)
        s += '\n)'
        return s

    def _rewrite_rtl(self, rtl, symtab, context):
        # type: (Rtl, Dict[str, Var], int) -> None
        for line in rtl.rtl:
            if isinstance(line, Def):
                line.defs = tuple(
                        self._rewrite_defs(line, symtab, context))
                expr = line.expr
            else:
                expr = line
            self._rewrite_expr(expr, symtab, context)

    def _rewrite_expr(self, expr, symtab, context):
        # type: (Apply, Dict[str, Var], int) -> None
        """
        Find all uses of variables in `expr` and replace them with our own
        local symbols.
        """

        # Accept a whole expression tree.
        stack = [expr]
        while len(stack) > 0:
            expr = stack.pop()
            expr.args = tuple(
                    self._rewrite_uses(expr, stack, symtab, context))

    def _rewrite_defs(self, line, symtab, context):
        # type: (Def, Dict[str, Var], int) -> Iterable[Var]
        """
        Given a tuple of symbols defined in a Def, rewrite them to local
        symbols. Yield the new locals.
        """
        for sym in line.defs:
            name = str(sym)
            if name in symtab:
                var = symtab[name]
                if var.get_def(context):
                    raise AssertionError("'{}' multiply defined".format(name))
            else:
                var = Var(name)
                symtab[name] = var
                self.defs.append(var)
            var.set_def(context, line)
            yield var

    def _rewrite_uses(self, expr, stack, symtab, context):
        # type: (Apply, List[Apply], Dict[str, Var], int) -> Iterable[Expr]
        """
        Given an `Apply` expr, rewrite all uses in its arguments to local
        variables. Yield a sequence of new arguments.

        Append any `Apply` arguments to `stack`.
        """
        for arg, operand in zip(expr.args, expr.inst.ins):
            # Nested instructions are allowed. Visit recursively.
            if isinstance(arg, Apply):
                stack.append(arg)
                yield arg
                continue
            if not isinstance(arg, Var):
                assert not operand.is_value(), "Value arg must be `Var`"
                yield arg
                continue
            # This is supposed to be a symbolic value reference.
            name = str(arg)
            if name in symtab:
                var = symtab[name]
                # The variable must be used consistently as a def or input.
                if not var.is_input() and not var.get_def(context):
                    raise AssertionError(
                            "'{}' used as both input and def"
                            .format(name))
            else:
                # First time use of variable.
                var = Var(name)
                symtab[name] = var
                self.inputs.append(var)
            yield var

    def verify_legalize(self):
        # type: () -> None
        """
        Verify that this is a valid legalization XForm.

        - The source pattern must describe a single instruction.
        - All values defined in the output pattern must be defined in the
          destination pattern.
        """
        assert len(self.src.rtl) == 1, "Legalize needs single instruction."
        for d in self.src.rtl[0].defs:
            if not d.is_output():
                raise AssertionError(
                        '{} not defined in dest pattern'.format(d))

    def apply(self, r, suffix=None):
        # type: (Rtl, str) -> Rtl
        """
        Given a concrete Rtl r s.t. r matches self.src, return the
        corresponding concrete self.dst. If suffix is provided, any temporary
        defs are renamed with '.suffix' appended to their old name.
        """
        assert r.is_concrete()
        s = self.src.substitution(r, {})  # type: VarAtomMap
        assert s is not None

        if (suffix is not None):
            for v in self.dst.vars():
                if v.is_temp():
                    assert v not in s
                    s[v] = Var(v.name + '.' + suffix)

        dst = self.dst.copy(s)
        dst.cleanup_concrete_rtl()
        return dst


class XFormGroup(object):
    """
    A group of related transformations.

    :param isa: A target ISA whose instructions are allowed.
    :param chain: A next level group to try if this one doesn't match.
    """

    def __init__(self, name, doc, isa=None, chain=None):
        # type: (str, str, TargetISA, XFormGroup) -> None
        self.xforms = list()  # type: List[XForm]
        self.custom = OrderedDict()  # type: OrderedDict[Instruction, str]
        self.name = name
        self.__doc__ = doc
        self.isa = isa
        self.chain = chain

    def __str__(self):
        # type: () -> str
        if self.isa:
            return '{}.{}'.format(self.isa.name, self.name)
        else:
            return self.name

    def rust_name(self):
        # type: () -> str
        """
        Get the Rust name of this function implementing this transform.
        """
        if self.isa:
            # This is a function in the same module as the LEGALIZE_ACTION
            # table referring to it.
            return self.name
        else:
            return '::legalizer::{}'.format(self.name)

    def legalize(self, src, dst):
        # type: (Union[Def, Apply], Rtl) -> None
        """
        Add a legalization pattern to this group.

        :param src: Single `Def` or `Apply` to be legalized.
        :param dst: `Rtl` list of replacement instructions.
        """
        xform = XForm(Rtl(src), dst)
        xform.verify_legalize()
        self.xforms.append(xform)

    def custom_legalize(self, inst, funcname):
        # type: (Instruction, str) -> None
        """
        Add a custom legalization action for `inst`.

        The `funcname` parameter is the fully qualified name of a Rust function
        which takes the same arguments as the `isa::Legalize` actions.

        The custom function will be called to legalize `inst` and any return
        value is ignored.
        """
        assert inst not in self.custom, "Duplicate custom_legalize"
        self.custom[inst] = funcname

"""
Type Inference
"""
from .typevar import TypeVar
from .ast import Def, Var
from copy import copy
from itertools import product

try:
    from typing import Dict, TYPE_CHECKING, Union, Tuple, Optional, Set # noqa
    from typing import Iterable # noqa
    from typing import cast, List
    from .xform import Rtl, XForm # noqa
    from .ast import Expr # noqa
    if TYPE_CHECKING:
        Constraint = Tuple[TypeVar, TypeVar]
        ConstraintList = List[Constraint]
        TypeMap = Dict[TypeVar, TypeVar]
        VarMap = Dict[Var, TypeVar]
except ImportError:
    TYPE_CHECKING = False
    pass


class TypeEnv(object):
    """
    Class encapsulating the neccessary book keeping for type inference.
        :attribute type_map: dict holding the equivalence relations between tvs
        :attribute constraints: a list of accumulated constraints - tuples
                            (tv1, tv2)) where tv1 and tv2 are equal
        :attribute ranks: dictionary recording the (optional) ranks for tvs.
                          'rank' is a partial ordering on TVs based on their
                          origin. See comments in rank() and register().
        :attribute vars: a set containing all known Vars
        :attribute idx: counter used to get fresh ids
    """

    RANK_DERIVED = 5
    RANK_INPUT = 4
    RANK_INTERMEDIATE = 3
    RANK_OUTPUT = 2
    RANK_TEMP = 1
    RANK_INTERNAL = 0

    def __init__(self, arg=None):
        # type: (Optional[Tuple[TypeMap, ConstraintList]]) -> None
        self.ranks = {}  # type: Dict[TypeVar, int]
        self.vars = set()  # type: Set[Var]

        if arg is None:
            self.type_map = {}  # type: TypeMap
            self.constraints = []  # type: ConstraintList
        else:
            self.type_map, self.constraints = arg

        self.idx = 0

    def __getitem__(self, arg):
        # type: (Union[TypeVar, Var]) -> TypeVar
        """
        Lookup the canonical representative for a Var/TypeVar.
        """
        if (isinstance(arg, Var)):
            tv = arg.get_typevar()
        else:
            assert (isinstance(arg, TypeVar))
            tv = arg

        while tv in self.type_map:
            tv = self.type_map[tv]

        if tv.is_derived:
            tv = TypeVar.derived(self[tv.base], tv.derived_func)
        return tv

    def equivalent(self, tv1, tv2):
        # type: (TypeVar, TypeVar) -> None
        """
        Record a that the free tv1 is part of the same equivalence class as
        tv2.  The canonical representative of the merged class is tv2's
        cannonical representative.
        """
        assert not tv1.is_derived
        assert self[tv1] == tv1

        # Make sure we don't create cycles
        if tv2.is_derived:
            assert self[tv2.base] != tv1

        self.type_map[tv1] = tv2

    def add_constraint(self, tv1, tv2):
        # type: (TypeVar, TypeVar) -> None
        """
        Add a new equivalence constraint between tv1 and tv2
        """
        self.constraints.append((tv1, tv2))

    def get_uid(self):
        # type: () -> str
        r = str(self.idx)
        self.idx += 1
        return r

    def __repr__(self):
        # type: () -> str
        return self.dot()

    def rank(self, tv):
        # type: (TypeVar) -> int
        """
        Get the rank of tv in the partial order. TVs directly associated with a
        Var get their rank from the Var (see register()).
        Internally generated non-derived TVs implicitly get the lowest rank (0)
        Derived variables get the highest rank.
        """
        default_rank = TypeEnv.RANK_DERIVED if tv.is_derived else\
            TypeEnv.RANK_INTERNAL
        return self.ranks.get(tv, default_rank)

    def register(self, v):
        # type: (Var) -> None
        """
        Register a new Var v.  This computes a rank for the associated TypeVar
        for v, which is used to impose a partial order on type variables.
        """
        self.vars.add(v)

        if v.is_input():
            r = TypeEnv.RANK_INPUT
        elif v.is_intermediate():
            r = TypeEnv.RANK_INTERMEDIATE
        elif v.is_output():
            r = TypeEnv.RANK_OUTPUT
        else:
            assert(v.is_temp())
            r = TypeEnv.RANK_TEMP

        self.ranks[v.get_typevar()] = r

    def free_typevars(self):
        # type: () -> List[TypeVar]
        """
        Get the free typevars in the current type env.
        """
        tvs = set([self[tv].free_typevar() for tv in self.type_map.keys()])
        # Filter out None here due to singleton type vars
        return sorted(filter(lambda x: x is not None, tvs),
                      key=lambda x:   x.name)

    def normalize(self):
        # type: () -> None
        """
        Normalize by:
            - collapsing any roots that don't correspond to a concrete TV AND
              have a single TV derived from them or equivalent to them

        E.g. if we have a root of the tree that looks like:

          typeof_a   typeof_b
                 \  /
              typeof_x
                  |
                half_width(1)
                  |
                  1

        we want to collapse the linear path between 1 and typeof_x. The
        resulting graph is:

          typeof_a   typeof_b
                 \  /
              typeof_x
        """
        source_tvs = set([v.get_typevar() for v in self.vars])
        children = {}  # type: Dict[TypeVar, Set[TypeVar]]
        for v in self.type_map.values():
            if not v.is_derived:
                continue

            t = v.free_typevar()
            s = children.get(t, set())
            s.add(v)
            children[t] = s

        for (a, b) in self.type_map.items():
            s = children.get(b, set())
            s.add(a)
            children[b] = s

        for r in self.free_typevars():
            while (r not in source_tvs and r in children and
                   len(children[r]) == 1):
                child = list(children[r])[0]
                if child in self.type_map:
                    assert self.type_map[child] == r
                    del self.type_map[child]

                r = child

    def extract(self):
        # type: () -> TypeEnv
        """
        Extract a clean type environment from self, that only mentions
        TVs associated with real variables
        """
        vars_tvs = set([v.get_typevar() for v in self.vars])
        new_type_map = {tv: self[tv] for tv in vars_tvs if tv != self[tv]}
        new_constraints = [(self[tv1], self[tv2])
                           for (tv1, tv2) in self.constraints]

        # Sanity: new constraints and the new type_map should only contain
        # tvs associated with real vars
        for (a, b) in new_constraints:
            assert a.free_typevar() in vars_tvs and\
                   b.free_typevar() in vars_tvs

        for (k, v) in new_type_map.items():
            assert k in vars_tvs
            assert v.free_typevar() is None or v.free_typevar() in vars_tvs

        t = TypeEnv()
        t.type_map = new_type_map
        t.constraints = new_constraints
        # ranks and vars contain only TVs associated with real vars
        t.ranks = copy(self.ranks)
        t.vars = copy(self.vars)
        return t

    def concrete_typings(self):
        # type: () -> Iterable[VarMap]
        """
        Return an iterable over all possible concrete typings permitted by this
        TypeEnv.
        """
        free_tvs = self.free_typevars()
        free_tv_iters = [tv.get_typeset().concrete_types() for tv in free_tvs]
        for concrete_types in product(*free_tv_iters):
            # Build type substitutions for all free vars
            m = {tv: TypeVar.singleton(typ)
                 for (tv, typ) in zip(free_tvs, concrete_types)}

            concrete_var_map = {v: subst(self[v.get_typevar()], m)
                                for v in self.vars}

            # Check if constraints are satisfied for this typing
            failed = None
            for (tv1, tv2) in self.constraints:
                tv1 = subst(tv1, m)
                tv2 = subst(tv2, m)
                assert tv1.get_typeset().size() == 1 and\
                    tv2.get_typeset().size() == 1
                if (tv1.get_typeset() != tv2.get_typeset()):
                    failed = (tv1, tv2)
                    break

            if (failed is not None):
                continue

            yield concrete_var_map

    def dot(self):
        # type: () -> str
        """
        Return a representation of self as a graph in dot format.
            Nodes correspond to TypeVariables.
            Dotted edges correspond to equivalences between TVS
            Solid edges correspond to derivation relations between TVs.
            Dashed edges correspond to equivalence constraints.
        """
        def label(s):
            # type: (TypeVar) -> str
            return "\"" + str(s) + "\""

        # Add all registered TVs (as some of them may be singleton nodes not
        # appearing in the graph
        nodes = set([v.get_typevar() for v in self.vars])  # type: Set[TypeVar]
        edges = set()  # type: Set[Tuple[TypeVar, TypeVar, str, Optional[str]]]

        for (k, v) in self.type_map.items():
            # Add all intermediate TVs appearing in edges
            nodes.add(k)
            nodes.add(v)
            edges.add((k, v, "dotted", None))
            while (v.is_derived):
                nodes.add(v.base)
                edges.add((v, v.base, "solid", v.derived_func))
                v = v.base

        for (a, b) in self.constraints:
            assert a in nodes and b in nodes
            edges.add((a, b, "dashed", None))

        root_nodes = set([x for x in nodes
                          if x not in self.type_map and not x.is_derived])

        r = "digraph {\n"
        for n in nodes:
            r += label(n)
            if n in root_nodes:
                r += "[xlabel=\"{}\"]".format(self[n].get_typeset())
            r += ";\n"

        for (n1, n2, style, elabel) in edges:
            e = label(n1)
            if style == "dashed":
                e += '--'
            else:
                e += '->'
            e += label(n2)
            e += "[style={}".format(style)

            if elabel is not None:
                e += ",label={}".format(elabel)
            e += "];\n"

            r += e
        r += "}"

        return r


if TYPE_CHECKING:
    TypingError = str
    TypingOrError = Union[TypeEnv, TypingError]


def get_error(typing_or_err):
    # type: (TypingOrError) -> Optional[TypingError]
    """
    Helper function to appease mypy when checking the result of typing.
    """
    if isinstance(typing_or_err, str):
        if (TYPE_CHECKING):
            return cast(TypingError, typing_or_err)
        else:
            return typing_or_err
    else:
        return None


def get_type_env(typing_or_err):
    # type: (TypingOrError) -> TypeEnv
    """
    Helper function to appease mypy when checking the result of typing.
    """
    assert isinstance(typing_or_err, TypeEnv)
    if (TYPE_CHECKING):
        return cast(TypeEnv, typing_or_err)
    else:
        return typing_or_err


def subst(tv, tv_map):
    # type: (TypeVar, TypeMap) -> TypeVar
    """
    Perform substition on the input tv using the TypeMap tv_map.
    """
    if tv in tv_map:
        return tv_map[tv]

    if tv.is_derived:
        return TypeVar.derived(subst(tv.base, tv_map), tv.derived_func)

    return tv


def normalize_tv(tv):
    # type: (TypeVar) -> TypeVar
    """
    Normalize a (potentially derived) TV using the following rules:
        - vector and width derived functions commute
        {HALF,DOUBLE}VECTOR({HALF,DOUBLE}WIDTH(base)) ->
            {HALF,DOUBLE}WIDTH({HALF,DOUBLE}VECTOR(base))

        - half/double pairs collapse
        {HALF,DOUBLE}WIDTH({DOUBLE,HALF}WIDTH(base)) -> base
        {HALF,DOUBLE}VECTOR({DOUBLE,HALF}VECTOR(base)) -> base
    """
    vector_derives = [TypeVar.HALFVECTOR, TypeVar.DOUBLEVECTOR]
    width_derives = [TypeVar.HALFWIDTH, TypeVar.DOUBLEWIDTH]

    if not tv.is_derived:
        return tv

    df = tv.derived_func

    if (tv.base.is_derived):
        base_df = tv.base.derived_func

        # Reordering: {HALFWIDTH, DOUBLEWIDTH} commute with {HALFVECTOR,
        # DOUBLEVECTOR}. Arbitrarily pick WIDTH < VECTOR
        if df in vector_derives and base_df in width_derives:
            return normalize_tv(
                    TypeVar.derived(
                        TypeVar.derived(tv.base.base, df), base_df))

        # Cancelling: HALFWIDTH, DOUBLEWIDTH and HALFVECTOR, DOUBLEVECTOR
        # cancel each other. Note: This doesn't hide any over/underflows,
        # since we 1) assert the safety of each TV in the chain upon its
        # creation, and 2) the base typeset is only allowed to shrink.

        if (df, base_df) in \
                [(TypeVar.HALFVECTOR, TypeVar.DOUBLEVECTOR),
                 (TypeVar.DOUBLEVECTOR, TypeVar.HALFVECTOR),
                 (TypeVar.HALFWIDTH, TypeVar.DOUBLEWIDTH),
                 (TypeVar.DOUBLEWIDTH, TypeVar.HALFWIDTH)]:
            return normalize_tv(tv.base.base)

    return TypeVar.derived(normalize_tv(tv.base), df)


def constrain_fixpoint(tv1, tv2):
    # type: (TypeVar, TypeVar) -> None
    """
    Given typevars tv1 and tv2 (which could be derived from one another)
    constrain their typesets to be the same. When one is derived from the
    other, repeat the constrain process until fixpoint.
    """
    # Constrain tv2's typeset as long as tv1's typeset is changing.
    while True:
        old_tv1_ts = tv1.get_typeset().copy()
        tv2.constrain_types(tv1)
        if tv1.get_typeset() == old_tv1_ts:
            break

    old_tv2_ts = tv2.get_typeset().copy()
    tv1.constrain_types(tv2)
    assert old_tv2_ts == tv2.get_typeset()


def unify(tv1, tv2, typ):
    # type: (TypeVar, TypeVar, TypeEnv) -> TypingOrError
    """
    Unify tv1 and tv2 in the current type environment typ, and return an
    updated type environment or error.
    """
    tv1 = normalize_tv(typ[tv1])
    tv2 = normalize_tv(typ[tv2])

    # Already unified
    if tv1 == tv2:
        return typ

    if typ.rank(tv2) < typ.rank(tv1):
        return unify(tv2, tv1, typ)

    constrain_fixpoint(tv1, tv2)

    if (tv1.get_typeset().size() == 0 or tv2.get_typeset().size() == 0):
        return "Error: empty type created when unifying {} and {}"\
               .format(tv1, tv2)

    # Free -> Derived(Free)
    if not tv1.is_derived:
        typ.equivalent(tv1, tv2)
        return typ

    assert tv2.is_derived, "Ordering gives us !tv1.is_derived==>tv2.is_derived"

    if (tv1.is_derived and TypeVar.is_bijection(tv1.derived_func)):
        inv_f = TypeVar.inverse_func(tv1.derived_func)
        return unify(tv1.base, normalize_tv(TypeVar.derived(tv2, inv_f)), typ)

    typ.add_constraint(tv1, tv2)
    return typ


def ti_def(definition, typ):
    # type: (Def, TypeEnv) -> TypingOrError
    """
    Perform type inference on one Def in the current type environment typ and
    return an updated type environment or error.

    At a high level this works by creating fresh copies of each formal type var
    in the Def's instruction's signature, and unifying the formal tv with the
    corresponding actual tv.
    """
    expr = definition.expr
    inst = expr.inst

    # Create a map m mapping each free typevar in the signature of definition
    # to a fresh copy of itself
    all_formal_tvs = \
        [inst.outs[i].typevar for i in inst.value_results] +\
        [inst.ins[i].typevar for i in inst.value_opnums]
    free_formal_tvs = [tv for tv in all_formal_tvs if not tv.is_derived]
    m = {tv: tv.get_fresh_copy(str(typ.get_uid())) for tv in free_formal_tvs}

    # Get fresh copies for each typevar in the signature (both free and
    # derived)
    fresh_formal_tvs = \
        [subst(inst.outs[i].typevar, m) for i in inst.value_results] +\
        [subst(inst.ins[i].typevar, m) for i in inst.value_opnums]

    # Get the list of actual Vars
    actual_vars = []  # type: List[Expr]
    actual_vars += [definition.defs[i] for i in inst.value_results]
    actual_vars += [expr.args[i] for i in inst.value_opnums]

    # Get the list of the actual TypeVars
    actual_tvs = []
    for v in actual_vars:
        assert(isinstance(v, Var))
        # Register with TypeEnv that this typevar corresponds ot variable v,
        # and thus has a given rank
        typ.register(v)
        actual_tvs.append(v.get_typevar())

    # Unify each actual typevar with the correpsonding fresh formal tv
    for (actual_tv, formal_tv) in zip(actual_tvs, fresh_formal_tvs):
        typ_or_err = unify(actual_tv, formal_tv, typ)
        err = get_error(typ_or_err)
        if (err):
            return "fail ti on {} <: {}: ".format(actual_tv, formal_tv) + err

        typ = get_type_env(typ_or_err)

    return typ


def ti_rtl(rtl, typ):
    # type: (Rtl, TypeEnv) -> TypingOrError
    """
    Perform type inference on an Rtl in a starting type env typ.  Return an
    updated type environment or error.
    """
    for (i, d) in enumerate(rtl.rtl):
        assert (isinstance(d, Def))
        typ_or_err = ti_def(d, typ)
        err = get_error(typ_or_err)  # type: Optional[TypingError]
        if (err):
            return "On line {}: ".format(i) + err

        typ = get_type_env(typ_or_err)

    return typ


def ti_xform(xform, typ):
    # type: (XForm, TypeEnv) -> TypingOrError
    """
    Perform type inference on an Rtl in a starting type env typ.  Return an
    updated type environment or error.
    """
    typ_or_err = ti_rtl(xform.src, typ)
    err = get_error(typ_or_err)  # type: Optional[TypingError]
    if (err):
        return "In src pattern: " + err

    typ = get_type_env(typ_or_err)

    typ_or_err = ti_rtl(xform.dst, typ)
    err = get_error(typ_or_err)
    if (err):
        return "In dst pattern: " + err

    typ = get_type_env(typ_or_err)

    return get_type_env(typ_or_err)

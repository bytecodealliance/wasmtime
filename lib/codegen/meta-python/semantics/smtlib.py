"""
Tools to emit SMTLIB bitvector queries encoding concrete RTLs containing only
primitive instructions.
"""
from .primitives import GROUP as PRIMITIVES, prim_from_bv, prim_to_bv, bvadd,\
    bvult, bvzeroext, bvsplit, bvconcat, bvsignext
from cdsl.ast import Var
from cdsl.types import BVType
from .elaborate import elaborate
from z3 import BitVec, ZeroExt, SignExt, And, Extract, Concat, Not, Solver,\
    unsat, BoolRef, BitVecVal, If
from z3.z3core import Z3_mk_eq

try:
    from typing import TYPE_CHECKING, Tuple, Dict, List # noqa
    from cdsl.xform import Rtl, XForm # noqa
    from cdsl.ast import VarAtomMap, Atom # noqa
    from cdsl.ti import VarTyping # noqa
    if TYPE_CHECKING:
        from z3 import ExprRef, BitVecRef # noqa
        Z3VarMap = Dict[Var, BitVecRef]
except ImportError:
    TYPE_CHECKING = False


# Use this for constructing a == b instead of == since MyPy doesn't
# accept overloading of __eq__ that doesn't return bool
def mk_eq(e1, e2):
    # type: (ExprRef, ExprRef) -> ExprRef
    """Return a z3 expression equivalent to e1 == e2"""
    return BoolRef(Z3_mk_eq(e1.ctx_ref(), e1.as_ast(), e2.as_ast()), e1.ctx)


def to_smt(r):
    # type: (Rtl) -> Tuple[List[ExprRef], Z3VarMap]
    """
    Encode a concrete primitive Rtl r sa z3 query.
    Returns a tuple (query, var_m) where:
        - query is a list of z3 expressions
        - var_m is a map from Vars v with non-BVType to their correspodning z3
          bitvector variable.
    """
    assert r.is_concrete()
    # Should contain only primitives
    primitives = set(PRIMITIVES.instructions)
    assert set(d.expr.inst for d in r.rtl).issubset(primitives)

    q = []  # type: List[ExprRef]
    m = {}  # type: Z3VarMap

    # Build declarations for any bitvector Vars
    var_to_bv = {}  # type: Z3VarMap
    for v in r.vars():
        typ = v.get_typevar().singleton_type()
        if not isinstance(typ, BVType):
            continue

        var_to_bv[v] = BitVec(v.name, typ.bits)

    # Encode each instruction as a equality assertion
    for d in r.rtl:
        inst = d.expr.inst

        exp = None  # type: ExprRef
        # For prim_to_bv/prim_from_bv just update var_m. No assertion needed
        if inst == prim_to_bv:
            assert isinstance(d.expr.args[0], Var)
            m[d.expr.args[0]] = var_to_bv[d.defs[0]]
            continue

        if inst == prim_from_bv:
            assert isinstance(d.expr.args[0], Var)
            m[d.defs[0]] = var_to_bv[d.expr.args[0]]
            continue

        if inst in [bvadd, bvult]:  # Binary instructions
            assert len(d.expr.args) == 2 and len(d.defs) == 1
            lhs = d.expr.args[0]
            rhs = d.expr.args[1]
            df = d.defs[0]
            assert isinstance(lhs, Var) and isinstance(rhs, Var)

            if inst == bvadd:  # Normal binary - output type same as args
                exp = (var_to_bv[lhs] + var_to_bv[rhs])
            else:
                assert inst == bvult
                exp = (var_to_bv[lhs] < var_to_bv[rhs])
                # Comparison binary - need to convert bool to BitVec 1
                exp = If(exp, BitVecVal(1, 1), BitVecVal(0, 1))

            exp = mk_eq(var_to_bv[df], exp)
        elif inst == bvzeroext:
            arg = d.expr.args[0]
            df = d.defs[0]
            assert isinstance(arg, Var)
            fromW = arg.get_typevar().singleton_type().width()
            toW = df.get_typevar().singleton_type().width()

            exp = mk_eq(var_to_bv[df], ZeroExt(toW-fromW, var_to_bv[arg]))
        elif inst == bvsignext:
            arg = d.expr.args[0]
            df = d.defs[0]
            assert isinstance(arg, Var)
            fromW = arg.get_typevar().singleton_type().width()
            toW = df.get_typevar().singleton_type().width()

            exp = mk_eq(var_to_bv[df], SignExt(toW-fromW, var_to_bv[arg]))
        elif inst == bvsplit:
            arg = d.expr.args[0]
            assert isinstance(arg, Var)
            arg_typ = arg.get_typevar().singleton_type()
            width = arg_typ.width()
            assert (width % 2 == 0)

            lo = d.defs[0]
            hi = d.defs[1]

            exp = And(mk_eq(var_to_bv[lo],
                      Extract(width//2-1, 0, var_to_bv[arg])),
                      mk_eq(var_to_bv[hi],
                      Extract(width-1, width//2, var_to_bv[arg])))
        elif inst == bvconcat:
            assert isinstance(d.expr.args[0], Var) and \
                isinstance(d.expr.args[1], Var)
            lo = d.expr.args[0]
            hi = d.expr.args[1]
            df = d.defs[0]

            # Z3 Concat expects hi bits first, then lo bits
            exp = mk_eq(var_to_bv[df], Concat(var_to_bv[hi], var_to_bv[lo]))
        else:
            assert False, "Unknown primitive instruction {}".format(inst)

        q.append(exp)

    return (q, m)


def equivalent(r1, r2, inp_m, out_m):
    # type: (Rtl, Rtl, VarAtomMap, VarAtomMap) -> List[ExprRef]
    """
    Given:
        - concrete source Rtl r1
        - concrete dest Rtl r2
        - VarAtomMap inp_m mapping r1's non-bitvector inputs to r2
        - VarAtomMap out_m mapping r1's non-bitvector outputs to r2

    Build a query checking whether r1 and r2 are semantically equivalent.
    If the returned query is unsatisfiable, then r1 and r2 are equivalent.
    Otherwise, the satisfying example for the query gives us values
    for which the two Rtls disagree.
    """
    # Sanity - inp_m is a bijection from the set of inputs of r1 to the set of
    # inputs of r2
    assert set(r1.free_vars()) == set(inp_m.keys())
    assert set(r2.free_vars()) == set(inp_m.values())

    # Note that the same rule is not expected to hold for out_m due to
    # temporaries/intermediates. out_m specified which values are enough for
    # equivalence.

    # Rename the vars in r1 and r2 with unique suffixes to avoid conflicts
    src_m = {v: Var(v.name + ".a", v.get_typevar()) for v in r1.vars()}  # type: VarAtomMap # noqa
    dst_m = {v: Var(v.name + ".b", v.get_typevar()) for v in r2.vars()}  # type: VarAtomMap # noqa
    r1 = r1.copy(src_m)
    r2 = r2.copy(dst_m)

    def _translate(m, k_m, v_m):
        # type: (VarAtomMap, VarAtomMap, VarAtomMap) -> VarAtomMap
        """Obtain a new map from m, by mapping m's keys with k_m and m's values
        with v_m"""
        res = {}  # type: VarAtomMap
        for (k, v) in m1.items():
            new_k = k_m[k]
            new_v = v_m[v]
            assert isinstance(new_k, Var)
            res[new_k] = new_v

        return res

    # Convert inp_m, out_m in terms of variables with the .a/.b suffixes
    inp_m = _translate(inp_m, src_m, dst_m)
    out_m = _translate(out_m, src_m, dst_m)

    # Encode r1 and r2 as SMT queries
    (q1, m1) = to_smt(r1)
    (q2, m2) = to_smt(r2)

    # Build an expression for the equality of real Cranelift inputs of
    # r1 and r2
    args_eq_exp = []  # type: List[ExprRef]

    for (v1, v2) in inp_m.items():
        assert isinstance(v2, Var)
        args_eq_exp.append(mk_eq(m1[v1], m2[v2]))

    # Build an expression for the equality of real Cranelift outputs of
    # r1 and r2
    results_eq_exp = []  # type: List[ExprRef]
    for (v1, v2) in out_m.items():
        assert isinstance(v2, Var)
        results_eq_exp.append(mk_eq(m1[v1], m2[v2]))

    # Put the whole query toghether
    return q1 + q2 + args_eq_exp + [Not(And(*results_eq_exp))]


def xform_correct(x, typing):
    # type: (XForm, VarTyping) -> bool
    """
    Given an XForm x and a concrete variable typing for x check whether x is
    semantically preserving for the concrete typing.
    """
    assert x.ti.permits(typing)

    # Create copies of the x.src and x.dst with their concrete types
    src_m = {v: Var(v.name, typing[v]) for v in x.src.vars()}  # type: VarAtomMap # noqa
    src = x.src.copy(src_m)
    dst = x.apply(src)
    dst_m = x.dst.substitution(dst, {})

    # Build maps for the inputs/outputs for src->dst
    inp_m = {}  # type: VarAtomMap
    out_m = {}  # type: VarAtomMap

    for v in x.src.vars():
        src_v = src_m[v]
        assert isinstance(src_v, Var)
        if v.is_input():
            inp_m[src_v] = dst_m[v]
        elif v.is_output():
            out_m[src_v] = dst_m[v]

    # Get the primitive semantic Rtls for src and dst
    prim_src = elaborate(src)
    prim_dst = elaborate(dst)
    asserts = equivalent(prim_src, prim_dst, inp_m, out_m)

    s = Solver()
    s.add(*asserts)
    return s.check() == unsat

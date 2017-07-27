"""
Tools to emit SMTLIB bitvector queries encoding concrete RTLs containing only
primitive instructions.
"""
from .primitives import GROUP as PRIMITIVES, prim_from_bv, prim_to_bv, bvadd,\
    bvult, bvzeroext
from cdsl.ast import Var
from cdsl.types import BVType

try:
    from typing import TYPE_CHECKING, Tuple # noqa
    from cdsl.xform import Rtl # noqa
    from cdsl.ast import VarMap # noqa
except ImportError:
    TYPE_CHECKING = False


def bvtype_to_sort(typ):
    # type: (BVType) -> str
    """Return the BitVec sort corresponding to a BVType"""
    return "(_ BitVec {})".format(typ.bits)


def to_smt(r):
    # type: (Rtl) -> Tuple[str, VarMap]
    """
    Encode a concrete primitive Rtl r sa SMTLIB 2.0 query.
    Returns a tuple (query, var_m) where:
        - query is the resulting query.
        - var_m is a map from Vars v with non-BVType to their Vars v' with
          BVType s.t. v' holds the flattend bitvector value of v.
    """
    assert r.is_concrete()
    # Should contain only primitives
    primitives = set(PRIMITIVES.instructions)
    assert all(d.expr.inst in primitives for d in r.rtl)

    q = ""
    m = {}  # type: VarMap
    for v in r.vars():
        typ = v.get_typevar().singleton_type()
        if not isinstance(typ, BVType):
            continue

        q += "(declare-fun {} () {})\n".format(v.name, bvtype_to_sort(typ))

    for d in r.rtl:
        inst = d.expr.inst

        if inst == prim_to_bv:
            assert isinstance(d.expr.args[0], Var)
            m[d.expr.args[0]] = d.defs[0]
            continue

        if inst == prim_from_bv:
            assert isinstance(d.expr.args[0], Var)
            m[d.defs[0]] = d.expr.args[0]
            continue

        if inst in [bvadd, bvult]:  # Binary instructions
            assert len(d.expr.args) == 2 and len(d.defs) == 1
            lhs = d.expr.args[0]
            rhs = d.expr.args[1]
            df = d.defs[0]
            assert isinstance(lhs, Var) and isinstance(rhs, Var)

            if inst in [bvadd]:  # Normal binary - output type same as args
                exp = "(= {} ({} {} {}))".format(df, inst.name, lhs, rhs)
            else:
                # Comparison binary - need to convert bool to BitVec 1
                exp = "(= {} (ite ({} {} {}) #b1 #b0))"\
                      .format(df, inst.name, lhs, rhs)
        elif inst == bvzeroext:
            arg = d.expr.args[0]
            df = d.defs[0]
            assert isinstance(arg, Var)
            fromW = arg.get_typevar().singleton_type().width()
            toW = df.get_typevar().singleton_type().width()

            exp = "(= {} ((_ zero_extend {}) {}))"\
                  .format(df, toW-fromW, arg, df)
        else:
            assert False, "Unknown primitive instruction {}".format(inst)

        q += "(assert {})\n".format(exp)

    return (q, m)


def equivalent(r1, r2, m):
    # type: (Rtl, Rtl, VarMap) -> str
    """
    Given concrete primitive Rtls r1 and r2, and a VarMap m, mapping all
    non-primitive vars in r1 onto r2, return a query checking that the
    two Rtls are semantically equivalent.

    If the returned query is unsatisfiable, then r1 and r2 are equivalent.
    Otherwise, the satisfying example for the query gives us values
    for which the two Rtls disagree.
    """
    # Rename the vars in r1 and r2 to avoid conflicts
    src_m = {v: Var(v.name + ".a", v.get_typevar()) for v in r1.vars()}
    dst_m = {v: Var(v.name + ".b", v.get_typevar()) for v in r2.vars()}
    m = {src_m[k]: dst_m[v] for (k, v) in m.items()}

    r1 = r1.copy(src_m)
    r2 = r2.copy(dst_m)

    r1_nonprim_vars = set(
        [v for v in r1.vars()
         if not isinstance(v.get_typevar().singleton_type(), BVType)])

    r2_nonprim_vars = set(
        [v for v in r2.vars()
         if not isinstance(v.get_typevar().singleton_type(), BVType)])

    # Check that the map m maps all non real Cretone Vars from r1 onto r2
    assert r1_nonprim_vars == set(m.keys())
    assert r2_nonprim_vars == set(m.values())

    (q1, m1) = to_smt(r1)
    (q2, m2) = to_smt(r2)

    # Build an expression for the equality of real Cretone inputs
    args_eq_exp = "(and \n"

    for v in r1.free_vars():
        assert v in r1_nonprim_vars
        args_eq_exp += "(= {} {})\n".format(m1[v], m2[m[v]])
    args_eq_exp += ")"

    # Build an expression for the equality of real Cretone defs
    results_eq_exp = "(and \n"
    for v in r1.definitions():
        if (v not in r1_nonprim_vars):
            continue

        results_eq_exp += "(= {} {})\n".format(m1[v], m2[m[v]])
    results_eq_exp += ")"

    q = '; Rtl 1 declarations and assertions\n' + q1
    q += '; Rtl 2 declarations and assertions\n' + q2

    q += '; Assert that the inputs of Rtl1 and Rtl2 are equal\n' + \
         '(assert {})\n'.format(args_eq_exp)

    q += '; Assert that the outputs of Rtl1 and Rtl2 are not equal\n' + \
         '(assert (not {}))\n'.format(results_eq_exp)

    return q

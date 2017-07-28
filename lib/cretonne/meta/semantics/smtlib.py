"""
Tools to emit SMTLIB bitvector queries encoding concrete RTLs containing only
primitive instructions.
"""
from .primitives import GROUP as PRIMITIVES, prim_from_bv, prim_to_bv, bvadd,\
    bvult, bvzeroext, bvsplit, bvconcat
from cdsl.ast import Var
from cdsl.types import BVType
from .elaborate import elaborate

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
                  .format(df, toW-fromW, arg)
        elif inst == bvsplit:
            arg = d.expr.args[0]
            arg_typ = arg.get_typevar().singleton_type()
            width = arg_typ.width()
            assert (width % 2 == 0)

            lo = d.defs[0]
            hi = d.defs[1]
            assert isinstance(arg, Var)

            exp = "(and "
            exp += "(= {} ((_ extract {} {}) {})) "\
                   .format(lo, width//2-1, 0, arg)
            exp += "(= {} ((_ extract {} {}) {}))"\
                   .format(hi, width-1, width//2, arg)
            exp += ")"
        elif inst == bvconcat:
            lo = d.expr.args[0]
            hi = d.expr.args[1]
            assert isinstance(lo, Var) and isinstance(hi, Var)
            df = d.defs[0]

            # Z3 Concat expects hi bits first, then lo bits
            exp = "(= {} (concat {} {}))"\
                  .format(df, hi, lo)
        else:
            assert False, "Unknown primitive instruction {}".format(inst)

        q += "(assert {})\n".format(exp)

    return (q, m)


def equivalent(r1, r2, inp_m, out_m):
    # type: (Rtl, Rtl, VarMap, VarMap) -> str
    """
    Given:
        - concrete source Rtl r1
        - concrete dest Rtl r2
        - VarMap inp_m mapping r1's non-bitvector inputs to r2
        - VarMap out_m mapping r1's non-bitvector outputs to r2

    Build a query checking whether r1 and r2 are semantically equivalent.
    If the returned query is unsatisfiable, then r1 and r2 are equivalent.
    Otherwise, the satisfying example for the query gives us values
    for which the two Rtls disagree.
    """
    # Rename the vars in r1 and r2 with unique suffixes to avoid conflicts
    src_m = {v: Var(v.name + ".a", v.get_typevar()) for v in r1.vars()}
    dst_m = {v: Var(v.name + ".b", v.get_typevar()) for v in r2.vars()}
    r1 = r1.copy(src_m)
    r2 = r2.copy(dst_m)

    # Convert inp_m, out_m in terms of variables with the .a/.b suffixes
    inp_m = {src_m[k]: dst_m[v] for (k, v) in inp_m.items()}
    out_m = {src_m[k]: dst_m[v] for (k, v) in out_m.items()}

    # Encode r1 and r2 as SMT queries
    (q1, m1) = to_smt(r1)
    (q2, m2) = to_smt(r2)

    # Build an expression for the equality of real Cretone inputs of r1 and r2
    args_eq_exp = "(and \n"

    for v in r1.free_vars():
        assert v in inp_m
        args_eq_exp += "(= {} {})\n".format(m1[v], m2[inp_m[v]])
    args_eq_exp += ")"

    # Build an expression for the equality of real Cretone outputs of r1 and r2
    results_eq_exp = "(and \n"
    for (v1, v2) in out_m.items():
        results_eq_exp += "(= {} {})\n".format(m1[v1], m2[v2])
    results_eq_exp += ")"

    # Put the whole query toghether
    q = '; Rtl 1 declarations and assertions\n' + q1
    q += '; Rtl 2 declarations and assertions\n' + q2

    q += '; Assert that the inputs of Rtl1 and Rtl2 are equal\n' + \
         '(assert {})\n'.format(args_eq_exp)

    q += '; Assert that the outputs of Rtl1 and Rtl2 are not equal\n' + \
         '(assert (not {}))\n'.format(results_eq_exp)

    return q


def xform_correct(x, typing):
    # type: (XForm, VarTyping) -> str
    """
    Given an XForm x and a concrete variable typing for x typing, build the
    smtlib query asserting that x is correct for the given typing.
    """
    assert x.ti.permits(typing)

    # Create copies of the x.src and x.dst with the concrete types in typing.
    src_m = {v: Var(v.name, typing[v]) for v in x.src.vars()}
    src = x.src.copy(src_m)
    dst = x.apply(src)
    dst_m = x.dst.substitution(dst, {})

    # Build maps for the inputs/outputs for src->dst
    inp_m = {}
    out_m = {}

    for v in x.src.vars():
        if v.is_input():
            inp_m[src_m[v]] = dst_m[v]
        elif v.is_output():
            out_m[src_m[v]] = dst_m[v]
        else:
            assert False, "Haven't decided what to do with intermediates yet"

    # Get the primitive semantic Rtls for src and dst
    prim_src = elaborate(src)
    prim_dst = elaborate(dst)
    return equivalent(prim_src, prim_dst, inp_m, out_m)

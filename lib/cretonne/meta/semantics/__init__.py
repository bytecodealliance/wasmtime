"""Definitions for the semantics segment of the Cretonne language."""
from cdsl.ti import TypeEnv, ti_rtl, get_type_env

try:
    from typing import List, Dict, Tuple # noqa
    from cdsl.ast import Var # noqa
    from cdsl.xform import XForm # noqa
    from cdsl.ti import VarTyping # noqa
    from cdsl.instructions import Instruction, InstructionSemantics # noqa
except ImportError:
    pass


def verify_semantics(inst, sem):
    # type: (Instruction, InstructionSemantics) -> None
    """
    Verify that the semantics sem correctly describes the instruction inst.
    This involves checking that:
        1) For all XForms x \in sem, x.src consists of a single instance of
           inst
        2) For any possible concrete typing of inst there is exactly 1 XForm x
           in sem that applies.
    """
    # 1) The source rtl is always a single instance of inst.
    for xform in sem:
        assert len(xform.src.rtl) == 1 and\
            xform.src.rtl[0].expr.inst == inst,\
            "XForm {} doesn't describe instruction {}."\
            .format(xform, inst)

    # 2) Any possible typing for the instruction should be covered by
    #    exactly ONE semantic XForm
    inst_rtl = sem[0].src
    typenv = get_type_env(ti_rtl(inst_rtl, TypeEnv()))

    # This bit is awkward. Concrete typing is defined in terms of the vars
    # of one Rtl. We arbitrarily picked that Rtl to be sem[0].src. For any
    # other XForms in sem, we must build a substitution form
    # sem[0].src->sem[N].src, before we can check if sem[N] permits one of
    # the concrete typings of our Rtl.
    # TODO: Can this be made cleaner?
    subst = [inst_rtl.substitution(x.src, {}) for x in sem]
    assert not any(x is None for x in subst)
    sub_sem = list(zip(subst, sem))  # type: List[Tuple[Dict[Var, Var], XForm]] # noqa

    def subst_typing(typing, sub):
        # type: (VarTyping, Dict[Var, Var]) -> VarTyping
        return {sub[v]: tv for (v, tv) in typing.items()}

    for t in typenv.concrete_typings():
        matching_xforms = [x for (s, x) in sub_sem
                           if x.ti.permits(subst_typing(t, s))]
        assert len(matching_xforms) == 1,\
            ("Possible typing {} of {} not matched by exactly one case " +
             ": {}").format(t, inst, matching_xforms)

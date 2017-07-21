"""Definitions for the semantics segment of the Cretonne language."""
from cdsl.ti import TypeEnv, ti_rtl, get_type_env

try:
    from typing import List, Dict, Tuple # noqa
    from cdsl.ast import Var # noqa
    from cdsl.xform import XForm, Rtl # noqa
    from cdsl.ti import VarTyping # noqa
    from cdsl.instructions import Instruction, InstructionSemantics # noqa
except ImportError:
    pass


def verify_semantics(inst, src, xforms):
    # type: (Instruction, Rtl, InstructionSemantics) -> None
    """
    Verify that the semantics transforms in xforms correctly describe the
    instruction described by the src Rtl.  This involves checking that:
        1) For all XForms x \in xforms, there is a Var substitution form src to
           x.src
        2) For any possible concrete typing of src there is exactly 1 XForm x
           in xforms that applies.
    """
    # 0) The source rtl is always a single instruction
    assert len(src.rtl) == 1

    # 1) For all XForms x, x.src is structurally equivalent to src
    for x in xforms:
        assert src.substitution(x.src, {}) is not None,\
            "XForm {} doesn't describe instruction {}.".format(x, src)

    # 2) Any possible typing for the instruction should be covered by
    #    exactly ONE semantic XForm
    typenv = get_type_env(ti_rtl(src, TypeEnv()))
    typenv.normalize()
    typenv = typenv.extract()

    for t in typenv.concrete_typings():
        matching_xforms = []  # type: List[XForm]
        for x in xforms:
            # Translate t using x.symtab
            t = {x.symtab[str(v)]:  tv for (v, tv) in t.items()}
            if (x.ti.permits(t)):
                matching_xforms.append(x)

        assert len(matching_xforms) == 1,\
            ("Possible typing {} of {} not matched by exactly one case " +
             ": {}").format(t, inst, matching_xforms)

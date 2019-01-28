"""Definitions for the semantics segment of the Cranelift language."""
from cdsl.ti import TypeEnv, ti_rtl, get_type_env
from cdsl.operands import ImmediateKind
from cdsl.ast import Var

try:
    from typing import List, Dict, Tuple # noqa
    from cdsl.ast import VarAtomMap  # noqa
    from cdsl.xform import XForm, Rtl # noqa
    from cdsl.ti import VarTyping # noqa
    from cdsl.instructions import Instruction, InstructionSemantics # noqa
except ImportError:
    pass


def verify_semantics(inst, src, xforms):
    # type: (Instruction, Rtl, InstructionSemantics) -> None
    """
    Verify that the semantics transforms in xforms correctly describe the
    instruction described by the src Rtl. This involves checking that:
        0) src is a single instance of inst
        1) For all x \\in xforms x.src is a single instance of inst
        2) For any concrete values V of Literals in inst:
            For all concrete typing T of inst:
                Exists single x \\in xforms that applies to src conretazied to
                V and T
    """
    # 0) The source rtl is always a single instance of inst
    assert len(src.rtl) == 1 and src.rtl[0].expr.inst == inst

    # 1) For all XForms x, x.src is a single instance of inst
    for x in xforms:
        assert len(x.src.rtl) == 1 and x.src.rtl[0].expr.inst == inst

    variants = [src]  # type: List[Rtl]

    # 2) For all enumerated immediates, compute all the possible
    #    versions of src with the concrete value filled in.
    for i in inst.imm_opnums:
        op = inst.ins[i]
        if not (isinstance(op.kind, ImmediateKind) and
                op.kind.is_enumerable()):
            continue

        new_variants = []  # type: List[Rtl]
        for rtl_var in variants:
            s = {v: v for v in rtl_var.vars()}  # type: VarAtomMap
            arg = rtl_var.rtl[0].expr.args[i]
            assert isinstance(arg, Var)
            for val in op.kind.possible_values():
                    s[arg] = val
                    new_variants.append(rtl_var.copy(s))
        variants = new_variants

    # For any possible version of the src with concrete enumerated immediates
    for src in variants:
        # 2) Any possible typing should be covered by exactly ONE semantic
        # XForm
        src = src.copy({})
        typenv = get_type_env(ti_rtl(src, TypeEnv()))
        typenv.normalize()
        typenv = typenv.extract()

        for t in typenv.concrete_typings():
            matching_xforms = []  # type: List[XForm]
            for x in xforms:
                if src.substitution(x.src, {}) is None:
                        continue

                # Translate t using x.symtab
                t = {x.symtab[str(v)]:  tv for (v, tv) in t.items()}
                if (x.ti.permits(t)):
                    matching_xforms.append(x)

            assert len(matching_xforms) == 1,\
                ("Possible typing {} of {} not matched by exactly one case " +
                 ": {}").format(t, src.rtl[0], matching_xforms)

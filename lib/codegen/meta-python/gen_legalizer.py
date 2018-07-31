"""
Generate legalizer transformations.

The transformations defined in the `cranelift.legalize` module are all of the
macro-expansion form where the input pattern is a single instruction. We
generate a Rust function for each `XFormGroup` which takes a `Cursor` pointing
at the instruction to be legalized. The expanded destination pattern replaces
the input instruction.
"""
from __future__ import absolute_import
from srcgen import Formatter
from collections import defaultdict
from base import instructions
from cdsl.ast import Var
from cdsl.ti import ti_rtl, TypeEnv, get_type_env, TypesEqual,\
    InTypeset, WiderOrEq
from unique_table import UniqueTable
from gen_instr import gen_typesets_table
from cdsl.typevar import TypeVar

try:
    from typing import Sequence, List, Dict, Set, DefaultDict # noqa
    from cdsl.isa import TargetISA  # noqa
    from cdsl.ast import Def, VarAtomMap  # noqa
    from cdsl.xform import XForm, XFormGroup  # noqa
    from cdsl.typevar import TypeSet # noqa
    from cdsl.ti import TypeConstraint # noqa
except ImportError:
    pass


def get_runtime_typechecks(xform):
    # type: (XForm) -> List[TypeConstraint]
    """
    Given a XForm build a list of runtime type checks neccessary to determine
    if it applies. We have 2 types of runtime checks:
        1) typevar tv belongs to typeset T - needed for free tvs whose
               typeset is constrainted by their use in the dst pattern

        2) tv1 == tv2 where tv1 and tv2 are derived TVs - caused by unification
                of non-bijective functions
    """
    check_l = []  # type: List[TypeConstraint]

    # 1) Perform ti only on the source RTL. Accumulate any free tvs that have a
    #    different inferred type in src, compared to the type inferred for both
    #    src and dst.
    symtab = {}  # type: VarAtomMap
    src_copy = xform.src.copy(symtab)
    src_typenv = get_type_env(ti_rtl(src_copy, TypeEnv()))

    for v in xform.ti.vars:
        if not v.has_free_typevar():
            continue

        # In rust the local variable containing a free TV associated with var v
        # has name typeof_v. We rely on the python TVs having the same name.
        assert "typeof_{}".format(v) == xform.ti[v].name

        if v not in symtab:
            # We can have singleton vars defined only on dst. Ignore them
            assert v.get_typevar().singleton_type() is not None
            continue

        inner_v = symtab[v]
        assert isinstance(inner_v, Var)
        src_ts = src_typenv[inner_v].get_typeset()
        xform_ts = xform.ti[v].get_typeset()

        assert xform_ts.issubset(src_ts)
        if src_ts != xform_ts:
            check_l.append(InTypeset(xform.ti[v], xform_ts))

    # 2,3) Add any constraints that appear in xform.ti
    check_l.extend(xform.ti.constraints)

    return check_l


def emit_runtime_typecheck(check, fmt, type_sets):
    # type: (TypeConstraint, Formatter, UniqueTable) -> None
    """
    Emit rust code for the given check.

    The emitted code is a statement redefining the `predicate` variable like
    this:

        let predicate = predicate && ...
    """
    def build_derived_expr(tv):
        # type: (TypeVar) -> str
        """
        Build an expression of type Option<Type> corresponding to a concrete
        type transformed by the sequence of derivation functions in tv.

        We are using Option<Type>, as some constraints may cause an
        over/underflow on patterns that do not match them. We want to capture
        this without panicking at runtime.
        """
        if not tv.is_derived:
            assert tv.name.startswith('typeof_')
            return "Some({})".format(tv.name)

        base_exp = build_derived_expr(tv.base)
        if (tv.derived_func == TypeVar.LANEOF):
            return "{}.map(|t: ir::Type| t.lane_type())".format(base_exp)
        elif (tv.derived_func == TypeVar.ASBOOL):
            return "{}.map(|t: ir::Type| t.as_bool())".format(base_exp)
        elif (tv.derived_func == TypeVar.HALFWIDTH):
            return "{}.and_then(|t: ir::Type| t.half_width())".format(base_exp)
        elif (tv.derived_func == TypeVar.DOUBLEWIDTH):
            return "{}.and_then(|t: ir::Type| t.double_width())"\
                .format(base_exp)
        elif (tv.derived_func == TypeVar.HALFVECTOR):
            return "{}.and_then(|t: ir::Type| t.half_vector())"\
                .format(base_exp)
        elif (tv.derived_func == TypeVar.DOUBLEVECTOR):
            return "{}.and_then(|t: ir::Type| t.by(2))".format(base_exp)
        else:
            assert False, "Unknown derived function {}".format(tv.derived_func)

    if (isinstance(check, InTypeset)):
        assert not check.tv.is_derived
        tv = check.tv.name
        if check.ts not in type_sets.index:
            type_sets.add(check.ts)
        ts = type_sets.index[check.ts]
        fmt.comment("{} must belong to {}".format(tv, check.ts))
        fmt.format(
                'let predicate = predicate && TYPE_SETS[{}].contains({});',
                ts, tv)
    elif (isinstance(check, TypesEqual)):
        with fmt.indented(
            'let predicate = predicate && match ({}, {}) {{'
            .format(build_derived_expr(check.tv1),
                    build_derived_expr(check.tv2)), '};'):
            fmt.line('(Some(a), Some(b)) => a == b,')
            fmt.comment('On overflow, constraint doesn\'t appply')
            fmt.line('_ => false,')
    elif (isinstance(check, WiderOrEq)):
        with fmt.indented(
            'let predicate = predicate && match ({}, {}) {{'
            .format(build_derived_expr(check.tv1),
                    build_derived_expr(check.tv2)), '};'):
            fmt.line('(Some(a), Some(b)) => a.wider_or_equal(b),')
            fmt.comment('On overflow, constraint doesn\'t appply')
            fmt.line('_ => false,')
    else:
        assert False, "Unknown check {}".format(check)


def unwrap_inst(iref, node, fmt):
    # type: (str, Def, Formatter) -> bool
    """
    Given a `Def` node, emit code that extracts all the instruction fields from
    `pos.func.dfg[iref]`.

    Create local variables named after the `Var` instances in `node`.

    Also create a local variable named `predicate` with the value of the
    evaluated instruction predicate, or `true` if the node has no predicate.

    :param iref: Name of the `Inst` reference to unwrap.
    :param node: `Def` node providing variable names.
    :returns: True if the instruction arguments were not detached, expecting a
              replacement instruction to overwrite the original.
    """
    fmt.comment('Unwrap {}'.format(node))
    expr = node.expr
    iform = expr.inst.format
    nvops = iform.num_value_operands

    # The tuple of locals to extract is the `Var` instances in `expr.args`.
    arg_names = tuple(
            arg.name if isinstance(arg, Var) else '_' for arg in expr.args)
    with fmt.indented(
            'let ({}, predicate) = if let ir::InstructionData::{} {{'
            .format(', '.join(map(str, arg_names)), iform.name), '};'):
        # Fields are encoded directly.
        for f in iform.imm_fields:
            fmt.line('{},'.format(f.member))
        if nvops == 1:
            fmt.line('arg,')
        elif iform.has_value_list or nvops > 1:
            fmt.line('ref args,')
        fmt.line('..')
        fmt.outdented_line('} = pos.func.dfg[inst] {')
        fmt.line('let func = &pos.func;')
        if iform.has_value_list:
            fmt.line('let args = args.as_slice(&func.dfg.value_lists);')
        elif nvops == 1:
            fmt.line('let args = [arg];')
        # Generate the values for the tuple.
        with fmt.indented('(', ')'):
            for opnum, op in enumerate(expr.inst.ins):
                if op.is_immediate():
                    n = expr.inst.imm_opnums.index(opnum)
                    fmt.format('{},', iform.imm_fields[n].member)
                elif op.is_value():
                    n = expr.inst.value_opnums.index(opnum)
                    fmt.format('func.dfg.resolve_aliases(args[{}]),', n)
            # Evaluate the instruction predicate, if any.
            instp = expr.inst_predicate_with_ctrl_typevar()
            fmt.line(instp.rust_predicate(0) if instp else 'true')
        fmt.outdented_line('} else {')
        fmt.line('unreachable!("bad instruction format")')

    # Get the types of any variables where it is needed.
    for opnum in expr.inst.value_opnums:
        v = expr.args[opnum]
        if isinstance(v, Var) and v.has_free_typevar():
            fmt.format('let typeof_{0} = pos.func.dfg.value_type({0});', v)

    # If the node has results, detach the values.
    # Place the values in locals.
    replace_inst = False
    if len(node.defs) > 0:
        if node.defs == node.defs[0].dst_def.defs:
            # Special case: The instruction replacing node defines the exact
            # same values.
            fmt.comment(
                    'Results handled by {}.'
                    .format(node.defs[0].dst_def))
            replace_inst = True
        else:
            # Boring case: Detach the result values, capture them in locals.
            for d in node.defs:
                fmt.line('let {};'.format(d))
            with fmt.indented('{', '}'):
                fmt.line('let r = pos.func.dfg.inst_results(inst);')
                for i in range(len(node.defs)):
                    fmt.line('{} = r[{}];'.format(node.defs[i], i))
            for d in node.defs:
                if d.has_free_typevar():
                    fmt.line(
                            'let typeof_{0} = pos.func.dfg.value_type({0});'
                            .format(d))

    return replace_inst


def wrap_tup(seq):
    # type: (Sequence[object]) -> str
    tup = tuple(map(str, seq))
    if len(tup) == 1:
        return tup[0]
    else:
        return '({})'.format(', '.join(tup))


def is_value_split(node):
    # type: (Def) -> bool
    """
    Determine if `node` represents one of the value splitting instructions:
    `isplit` or `vsplit. These instructions are lowered specially by the
    `legalize::split` module.
    """
    if len(node.defs) != 2:
        return False
    return node.expr.inst in (instructions.isplit, instructions.vsplit)


def emit_dst_inst(node, fmt):
    # type: (Def, Formatter) -> None
    replaced_inst = None  # type: str

    if is_value_split(node):
        # Split instructions are not emitted with the builder, but by calling
        # special functions in the `legalizer::split` module. These functions
        # will eliminate concat-split patterns.
        fmt.line('let curpos = pos.position();')
        fmt.line('let srcloc = pos.srcloc();')
        fmt.format(
                'let {} = split::{}(pos.func, cfg, curpos, srcloc, {});',
                wrap_tup(node.defs),
                node.expr.inst.snake_name(),
                node.expr.args[0])
    else:
        if len(node.defs) == 0:
            # This node doesn't define any values, so just insert the new
            # instruction.
            builder = 'pos.ins()'
        else:
            src_def0 = node.defs[0].src_def
            if src_def0 and node.defs == src_def0.defs:
                # The replacement instruction defines the exact same values as
                # the source pattern. Unwrapping would have left the results
                # intact.
                # Replace the whole instruction.
                builder = 'let {} = pos.func.dfg.replace(inst)'.format(
                        wrap_tup(node.defs))
                replaced_inst = 'inst'
            else:
                # Insert a new instruction.
                builder = 'let {} = pos.ins()'.format(wrap_tup(node.defs))
                # We may want to reuse some of the detached output values.
                if len(node.defs) == 1 and node.defs[0].is_output():
                    # Reuse the single source result value.
                    builder += '.with_result({})'.format(node.defs[0])
                elif any(d.is_output() for d in node.defs):
                    # We have some output values to be reused.
                    array = ', '.join(
                            ('Some({})'.format(d) if d.is_output()
                                else 'None')
                            for d in node.defs)
                    builder += '.with_results([{}])'.format(array)

        fmt.line('{}.{};'.format(builder, node.expr.rust_builder(node.defs)))

    # If we just replaced an instruction, we need to bump the cursor so
    # following instructions are inserted *after* the replaced instruction.
    if replaced_inst:
        with fmt.indented(
                'if pos.current_inst() == Some({}) {{'
                .format(replaced_inst), '}'):
            fmt.line('pos.next_inst();')


def gen_xform(xform, fmt, type_sets):
    # type: (XForm, Formatter, UniqueTable) -> None
    """
    Emit code for `xform`, assuming that the opcode of xform's root instruction
    has already been matched.

    `inst: Inst` is the variable to be replaced. It is pointed to by `pos:
    Cursor`.
    `dfg: DataFlowGraph` is available and mutable.
    """
    # Unwrap the source instruction, create local variables for the input
    # variables.
    replace_inst = unwrap_inst('inst', xform.src.rtl[0], fmt)

    # Emit any runtime checks.
    # These will rebind `predicate` emitted by unwrap_inst().
    for check in get_runtime_typechecks(xform):
        emit_runtime_typecheck(check, fmt, type_sets)

    # Guard the actual expansion by `predicate`.
    with fmt.indented('if predicate {', '}'):
        # If we're going to delete `inst`, we need to detach its results first
        # so they can be reattached during pattern expansion.
        if not replace_inst:
            fmt.line('pos.func.dfg.clear_results(inst);')

        # Emit the destination pattern.
        for dst in xform.dst.rtl:
            emit_dst_inst(dst, fmt)

        # Delete the original instruction if we didn't have an opportunity to
        # replace it.
        if not replace_inst:
            fmt.line('let removed = pos.remove_inst();')
            fmt.line('debug_assert_eq!(removed, inst);')
        fmt.line('return true;')


def gen_xform_group(xgrp, fmt, type_sets):
    # type: (XFormGroup, Formatter, UniqueTable) -> None
    fmt.doc_comment("Legalize `inst`.")
    fmt.line('#[allow(unused_variables,unused_assignments,non_snake_case)]')
    with fmt.indented('pub fn {}('.format(xgrp.name)):
        fmt.line('inst: ir::Inst,')
        fmt.line('func: &mut ir::Function,')
        fmt.line('cfg: &mut ::flowgraph::ControlFlowGraph,')
        fmt.line('isa: &::isa::TargetIsa,')
    with fmt.indented(') -> bool {', '}'):
        fmt.line('use ir::InstBuilder;')
        fmt.line('use cursor::{Cursor, FuncCursor};')
        fmt.line('let mut pos = FuncCursor::new(func).at_inst(inst);')
        fmt.line('pos.use_srcloc(inst);')

        # Group the xforms by opcode so we can generate a big switch.
        # Preserve ordering.
        xforms = defaultdict(list)  # type: DefaultDict[str, List[XForm]]
        for xform in xgrp.xforms:
            inst = xform.src.rtl[0].expr.inst
            xforms[inst.camel_name].append(xform)

        with fmt.indented('{', '}'):
            with fmt.indented('match pos.func.dfg[inst].opcode() {', '}'):
                for camel_name in sorted(xforms.keys()):
                    with fmt.indented(
                            'ir::Opcode::{} => {{'.format(camel_name), '}'):
                        for xform in xforms[camel_name]:
                            gen_xform(xform, fmt, type_sets)

                # Emit the custom transforms. The Rust compiler will complain
                # about any overlap with the normal xforms.
                for inst, funcname in xgrp.custom.items():
                    with fmt.indented(
                            'ir::Opcode::{} => {{'
                            .format(inst.camel_name), '}'):
                        fmt.format('{}(inst, pos.func, cfg, isa);', funcname)
                        fmt.line('return true;')

                # We'll assume there are uncovered opcodes.
                fmt.line('_ => {},')

        # If we fall through, nothing was expanded. Call the chain if any.
        if xgrp.chain:
            fmt.format('{}(inst, pos.func, cfg, isa)', xgrp.chain.rust_name())
        else:
            fmt.line('false')


def gen_isa(isa, fmt, shared_groups):
    # type: (TargetISA, Formatter, Set[XFormGroup]) -> None
    """
    Generate legalization functions for `isa` and add any shared `XFormGroup`s
    encountered to `shared_groups`.

    Generate `TYPE_SETS` and `LEGALIZE_ACTION` tables.
    """
    type_sets = UniqueTable()
    for xgrp in isa.legalize_codes.keys():
        if xgrp.isa is None:
            shared_groups.add(xgrp)
        else:
            assert xgrp.isa == isa
            gen_xform_group(xgrp, fmt, type_sets)

    gen_typesets_table(fmt, type_sets)

    with fmt.indented(
            'pub static LEGALIZE_ACTIONS: [isa::Legalize; {}] = ['
            .format(len(isa.legalize_codes)), '];'):
        for xgrp in isa.legalize_codes.keys():
            fmt.format('{},', xgrp.rust_name())


def generate(isas, out_dir):
    # type: (Sequence[TargetISA], str) -> None
    shared_groups = set()  # type: Set[XFormGroup]

    for isa in isas:
        fmt = Formatter()
        gen_isa(isa, fmt, shared_groups)
        fmt.update_file('legalize-{}.rs'.format(isa.name), out_dir)

    # Shared xform groups.
    fmt = Formatter()
    type_sets = UniqueTable()
    for xgrp in sorted(shared_groups, key=lambda g: g.name):
        gen_xform_group(xgrp, fmt, type_sets)
    gen_typesets_table(fmt, type_sets)
    fmt.update_file('legalizer.rs', out_dir)

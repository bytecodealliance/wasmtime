"""
Generate legalizer transformations.

The transformations defined in the `cretonne.legalize` module are all of the
macro-expansion form where the input pattern is a single instruction. We
generate a Rust function for each `XFormGroup` which takes a `Cursor` pointing
at the instruction to be legalized. The expanded destination pattern replaces
the input instruction.
"""
from __future__ import absolute_import
from srcgen import Formatter
from base import legalize
from cdsl.ast import Var

try:
    from typing import Sequence  # noqa
    from cdsl.ast import Def  # noqa
    from cdsl.xform import XForm, XFormGroup  # noqa
except ImportError:
    pass


def unwrap_inst(iref, node, fmt):
    # type: (str, Def, Formatter) -> None
    """
    Given a `Def` node, emit code that extracts all the instruction fields from
    `dfg[iref]`.

    Create local variables named after the `Var` instances in `node`.

    :param iref: Name of the `Inst` reference to unwrap.
    :param node: `Def` node providing variable names.

    """
    fmt.comment('Unwrap {}'.format(node))
    expr = node.expr
    iform = expr.inst.format
    nvops = iform.num_value_operands

    # The tuple of locals we're extracting is `expr.args`.
    with fmt.indented(
            'let ({}) = if let InstructionData::{} {{'
            .format(', '.join(map(str, expr.args)), iform.name), '};'):
        if iform.boxed_storage:
            # This format indirects to a largish `data` struct.
            fmt.line('ref data,')
        else:
            # Fields are encoded directly.
            for m in iform.imm_members:
                fmt.line('{},'.format(m))
            if nvops == 1:
                fmt.line('arg,')
            elif nvops != 0:
                fmt.line('args,')
        fmt.line('..')
        fmt.outdented_line('} = dfg[inst] {')
        # Generate the values for the tuple.
        outs = list()
        prefix = 'data.' if iform.boxed_storage else ''
        for opnum, op in enumerate(expr.inst.ins):
            if op.is_immediate():
                n = expr.inst.imm_opnums.index(opnum)
                outs.append(prefix + iform.imm_members[n])
            elif op.is_value():
                if nvops == 1:
                    arg = prefix + 'arg'
                else:
                    n = expr.inst.value_opnums.index(opnum)
                    arg = '{}args[{}]'.format(prefix, n)
                outs.append('dfg.resolve_aliases({})'.format(arg))
        fmt.line('({})'.format(', '.join(outs)))
        fmt.outdented_line('} else {')
        fmt.line('unreachable!("bad instruction format")')

    # Get the types of any variables where it is needed.
    for opnum in expr.inst.value_opnums:
        v = expr.args[opnum]
        if isinstance(v, Var) and v.has_free_typevar():
            fmt.line('let typeof_{0} = dfg.value_type({0});'.format(v))

    # If the node has multiple results, detach the values.
    # Place the secondary values in 'src_{}' locals.
    if len(node.defs) > 1:
        if node.defs == node.defs[0].dst_def.defs:
            # Special case: The instruction replacing node defines the exact
            # same values.
            fmt.comment(
                    'Multiple results handled by {}.'
                    .format(node.defs[0].dst_def))
        else:
            fmt.comment('Detaching secondary results.')
            # Boring case: Detach the secondary values, capture them in locals.
            for d in node.defs[1:]:
                fmt.line('let src_{};'.format(d))
            with fmt.indented('{', '}'):
                fmt.line('let mut vals = dfg.detach_secondary_results(inst);')
                for d in node.defs[1:]:
                    fmt.line('src_{} = vals.next().unwrap();'.format(d))
                fmt.line('assert_eq!(vals.next(), None);')
            for d in node.defs[1:]:
                if d.has_free_typevar():
                    fmt.line(
                            'let typeof_{0} = dfg.value_type(src_{0});'
                            .format(d))


def wrap_tup(seq):
    # type: (Sequence[object]) -> str
    tup = tuple(map(str, seq))
    if len(tup) == 1:
        return tup[0]
    else:
        return '({})'.format(', '.join(tup))


def emit_dst_inst(node, fmt):
    # type: (Def, Formatter) -> None
    exact_replace = False
    replaced_inst = None  # type: str
    fixup_first_result = False
    if len(node.defs) == 0:
        # This node doesn't define any values, so just insert the new
        # instruction.
        builder = 'dfg.ins(pos)'
    else:
        src_def0 = node.defs[0].src_def
        if src_def0 and node.defs[0] == src_def0.defs[0]:
            # The primary result is replacing the primary result of the src
            # pattern.
            # Replace the whole instruction.
            builder = 'let {} = dfg.replace(inst)'.format(wrap_tup(node.defs))
            replaced_inst = 'inst'
            # Secondary values weren't replaced if this is an exact replacement
            # for all the src results.
            exact_replace = (node.defs == src_def0.defs)
        else:
            # Insert a new instruction since its primary def doesn't match the
            # src.
            builder = 'let {} = dfg.ins(pos)'.format(wrap_tup(node.defs))
            fixup_first_result = node.defs[0].is_output()

    fmt.line('{}.{};'.format(builder, node.expr.rust_builder(node.defs)))

    # If we just replaced an instruction, we need to bump the cursor so
    # following instructions are inserted *after* the replaced insruction.
    if replaced_inst:
        with fmt.indented(
                'if pos.current_inst() == Some({}) {{'
                .format(replaced_inst), '}'):
            fmt.line('pos.next_inst();')

    # Fix up any output vars.
    if fixup_first_result:
        # The first result of the instruction just inserted is an output var,
        # but it was not a primary result in the source pattern.
        # We need to change the original value to an alias of the primary one
        # we just inserted.
        fmt.line('dfg.change_to_alias(src_{0}, {0});'.format(node.defs[0]))

    if not exact_replace:
        # We don't support secondary values as outputs yet. Depending on the
        # source value, we would need to :
        # 1. For a primary source value, replace with a copy instruction.
        # 2. For a secondary source value, request that the builder reuses the
        #    value when making secondary result nodes.
        for d in node.defs[1:]:
            assert not d.is_output()


def gen_xform(xform, fmt):
    # type: (XForm, Formatter) -> None
    """
    Emit code for `xform`, assuming the the opcode of xform's root instruction
    has already been matched.

    `inst: Inst` is the variable to be replaced. It is pointed to by `pos:
    Cursor`.
    `dfg: DataFlowGraph` is available and mutable.
    """
    # Unwrap the source instruction, create local variables for the input
    # variables.
    unwrap_inst('inst', xform.src.rtl[0], fmt)

    # Emit the destination pattern.
    for dst in xform.dst.rtl:
        emit_dst_inst(dst, fmt)


def gen_xform_group(xgrp, fmt):
    # type: (XFormGroup, Formatter) -> None
    fmt.doc_comment("Legalize the instruction pointed to by `pos`.")
    fmt.line('#[allow(unused_variables,unused_assignments)]')
    with fmt.indented(
            'fn ' + xgrp.name +
            '(pos: &mut Cursor, dfg: &mut DataFlowGraph) -> bool {',
            '}'):
        # Gen the instruction to be legalized. The cursor we're passed must be
        # pointing at an instruction.
        fmt.line('let inst = pos.current_inst().expect("need instruction");')

        with fmt.indented('match dfg[inst].opcode() {', '}'):
            for xform in xgrp.xforms:
                inst = xform.src.rtl[0].expr.inst
                with fmt.indented(
                        'Opcode::{} => {{'.format(inst.camel_name), '}'):
                    gen_xform(xform, fmt)
            # We'll assume there are uncovered opcodes.
            fmt.line('_ => return false,')
        fmt.line('true')


def generate(isas, out_dir):
    fmt = Formatter()
    gen_xform_group(legalize.narrow, fmt)
    gen_xform_group(legalize.expand, fmt)
    fmt.update_file('legalizer.rs', out_dir)

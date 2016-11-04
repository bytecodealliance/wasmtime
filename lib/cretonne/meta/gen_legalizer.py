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
import cretonne.legalize as legalize
from cretonne.ast import Def  # noqa
from cretonne.xform import XForm, XFormGroup  # noqa

try:
    from typing import Sequence  # noqa
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
    nvops = len(iform.value_operands)

    # The tuple of locals we're extracting is `expr.args`.
    with fmt.indented(
            'let ({}) = if let InstructionData::{} {{'
            .format(', '.join(map(str, expr.args)), iform.name), '};'):
        if iform.boxed_storage:
            # This format indirects to a largish `data` struct.
            fmt.line('ref data,')
        else:
            # Fields are encoded directly.
            for m in iform.members:
                if m:
                    fmt.line('{},'.format(m))
            if nvops == 1:
                fmt.line('arg,')
            elif nvops > 1:
                fmt.line('args,')
        fmt.line('..')
        fmt.outdented_line('} = dfg[inst] {')
        # Generate the values for the tuple.
        outs = list()
        prefix = 'data.' if iform.boxed_storage else ''
        for i, m in enumerate(iform.members):
            if m:
                outs.append(prefix + m)
            else:
                # This is a value operand.
                if nvops == 1:
                    outs.append(prefix + 'arg')
                else:
                    outs.append(
                            '{}args[{}]'.format(
                                prefix, iform.value_operands.index(i)))
        fmt.line('({})'.format(', '.join(outs)))
        fmt.outdented_line('} else {')
        fmt.line('unreachable!("bad instruction format")')

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
            # Secondary values weren't replaced if this is an exact replacement
            # for all the src results.
            exact_replace = (node.defs == src_def0.defs)
        else:
            # Insert a new instruction since its primary def doesn't match the
            # src.
            builder = 'let {} = dfg.ins(pos)'.format(wrap_tup(node.defs))

    fmt.line('{}.{};'.format(builder, node.expr.rust_builder()))

    if exact_replace:
        fmt.comment('exactreplacement')


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
    fmt.doc_comment("""
        Legalize the instruction pointed to by `pos`.

        Return the first instruction in the expansion, and leave `pos` pointing
        at the last instruction in the expansion.
        """)
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

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
from base import legalize, instructions
from cdsl.ast import Var

try:
    from typing import Sequence  # noqa
    from cdsl.isa import TargetISA  # noqa
    from cdsl.ast import Def  # noqa
    from cdsl.xform import XForm, XFormGroup  # noqa
except ImportError:
    pass


def unwrap_inst(iref, node, fmt):
    # type: (str, Def, Formatter) -> bool
    """
    Given a `Def` node, emit code that extracts all the instruction fields from
    `dfg[iref]`.

    Create local variables named after the `Var` instances in `node`.

    :param iref: Name of the `Inst` reference to unwrap.
    :param node: `Def` node providing variable names.
    :returns: True if the instruction arguments were not detached, expecting a
              replacement instruction to overwrite the original.
    """
    fmt.comment('Unwrap {}'.format(node))
    expr = node.expr
    iform = expr.inst.format
    nvops = iform.num_value_operands

    # The tuple of locals we're extracting is `expr.args`.
    with fmt.indented(
            'let ({}) = if let InstructionData::{} {{'
            .format(', '.join(map(str, expr.args)), iform.name), '};'):
        # Fields are encoded directly.
        for f in iform.imm_fields:
            fmt.line('{},'.format(f.member))
        if nvops == 1:
            fmt.line('arg,')
        elif iform.has_value_list or nvops > 1:
            fmt.line('ref args,')
        fmt.line('..')
        fmt.outdented_line('} = dfg[inst] {')
        if iform.has_value_list:
            fmt.line('let args = args.as_slice(&dfg.value_lists);')
        # Generate the values for the tuple.
        outs = list()
        for opnum, op in enumerate(expr.inst.ins):
            if op.is_immediate():
                n = expr.inst.imm_opnums.index(opnum)
                outs.append(iform.imm_fields[n].member)
            elif op.is_value():
                if nvops == 1:
                    arg = 'arg'
                else:
                    n = expr.inst.value_opnums.index(opnum)
                    arg = 'args[{}]'.format(n)
                outs.append('dfg.resolve_aliases({})'.format(arg))
        fmt.line('({})'.format(', '.join(outs)))
        fmt.outdented_line('} else {')
        fmt.line('unreachable!("bad instruction format")')

    # Get the types of any variables where it is needed.
    for opnum in expr.inst.value_opnums:
        v = expr.args[opnum]
        if isinstance(v, Var) and v.has_free_typevar():
            fmt.line('let typeof_{0} = dfg.value_type({0});'.format(v))

    # If the node has results, detach the values.
    # Place the values in  locals.
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
            fmt.comment('Detaching results.')
            for d in node.defs:
                fmt.line('let {};'.format(d))
            with fmt.indented('{', '}'):
                fmt.line('let r = dfg.inst_results(inst);')
                for i in range(len(node.defs)):
                    fmt.line('{} = r[{}];'.format(node.defs[i], i))
            fmt.line('dfg.clear_results(inst);')
            for d in node.defs:
                if d.has_free_typevar():
                    fmt.line(
                            'let typeof_{0} = dfg.value_type({0});'
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
        fmt.line(
                'let {} = split::{}(dfg, cfg, pos, {});'
                .format(
                    wrap_tup(node.defs),
                    node.expr.inst.snake_name(),
                    node.expr.args[0]))
    else:
        if len(node.defs) == 0:
            # This node doesn't define any values, so just insert the new
            # instruction.
            builder = 'dfg.ins(pos)'
        else:
            src_def0 = node.defs[0].src_def
            if src_def0 and node.defs == src_def0.defs:
                # The replacement instruction defines the exact same values as
                # the source pattern. Unwrapping would have left the results
                # intact.
                # Replace the whole instruction.
                builder = 'let {} = dfg.replace(inst)'.format(
                        wrap_tup(node.defs))
                replaced_inst = 'inst'
            else:
                # Insert a new instruction.
                builder = 'let {} = dfg.ins(pos)'.format(wrap_tup(node.defs))
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
    replace_inst = unwrap_inst('inst', xform.src.rtl[0], fmt)

    # We could support instruction predicates, but not yet. Should we just
    # return false if it fails? What about multiple patterns with different
    # predicates for the same opcode?
    instp = xform.src.rtl[0].expr.inst_predicate()
    assert instp is None, "Instruction predicates not supported in legalizer"

    # Emit the destination pattern.
    for dst in xform.dst.rtl:
        emit_dst_inst(dst, fmt)

    # Delete the original instruction if we didn't have an opportunity to
    # replace it.
    if not replace_inst:
        fmt.line('assert_eq!(pos.remove_inst(), inst);')


def gen_xform_group(xgrp, fmt):
    # type: (XFormGroup, Formatter) -> None
    fmt.doc_comment("Legalize the instruction pointed to by `pos`.")
    fmt.line('#[allow(unused_variables,unused_assignments)]')
    with fmt.indented(
            'fn {}(dfg: &mut DataFlowGraph, '
            'cfg: &mut ControlFlowGraph, pos: &mut Cursor) -> '
            'bool {{'.format(xgrp.name), '}'):

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
    # type: (Sequence[TargetISA], str) -> None
    fmt = Formatter()
    gen_xform_group(legalize.narrow, fmt)
    gen_xform_group(legalize.expand, fmt)
    fmt.update_file('legalizer.rs', out_dir)

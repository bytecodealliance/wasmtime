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
from cretonne.ast import Def, Apply  # noqa
from cretonne.xform import XForm, XFormGroup  # noqa

try:
    from typing import Union
    DefApply = Union[Def, Apply]
except ImportError:
    pass


def unwrap_inst(iref, node, fmt):
    # type: (str, DefApply, Formatter) -> None
    """
    Given a `Def` or `Apply` node, emit code that extracts all the instruction
    fields from `dfg[iref]`.

    Create local variables named after the `Var` instances in `node`.

    :param iref: Name of the `Inst` reference to unwrap.
    :param node: `Def` or `Apply` node providing variable names.

    """
    fmt.comment('Unwrap {}'.format(node))
    defs, expr = node.defs_expr()
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
        fmt.line('unimplemented!("bad instruction format")')


def gen_xform(xform, fmt):
    # type: (XForm, Formatter) -> None
    """
    Emit code for `xform`, assuming the the opcode of xform's root instruction
    has already been matched.

    `inst: Inst` is the variable to be replaced. It is pointed to by `pos:
    Cursor`.
    `dfg: DataFlowGraph` is available and mutable.
    """
    unwrap_inst('inst', xform.src.rtl[0], fmt)


def gen_xform_group(xgrp, fmt):
    # type: (XFormGroup, Formatter) -> None
    fmt.doc_comment("""
        Legalize the instruction pointed to by `pos`.

        Return the first instruction in the expansion, and leave `pos` pointing
        at the last instruction in the expansion.
        """)
    with fmt.indented(
            'fn ' + xgrp.name +
            '(pos: &mut Cursor, dfg: &mut DataFlowGraph) -> ' +
            'Option<Inst> {{',
            '}'):
        # Gen the instruction to be legalized. The cursor we're passed must be
        # pointing at an instruction.
        fmt.line('let inst = pos.current_inst().expect("need instruction");')

        with fmt.indented('match dfg[inst].opcode() {', '}'):
            for xform in xgrp.xforms:
                inst = xform.src.rtl[0].root_inst()
                with fmt.indented(
                        'Opcode::{} => {{'.format(inst.camel_name), '}'):
                    gen_xform(xform, fmt)
            # We'll assume there are uncovered opcodes.
            fmt.line('_ => None,')


def generate(isas, out_dir):
    fmt = Formatter()
    gen_xform_group(legalize.narrow, fmt)
    gen_xform_group(legalize.expand, fmt)
    fmt.update_file('legalizer.rs', out_dir)

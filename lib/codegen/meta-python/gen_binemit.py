"""
Generate binary emission code for each ISA.
"""

from __future__ import absolute_import
from cdsl.registers import RegClass, Stack
import srcgen

try:
    from typing import Sequence, List  # noqa
    from cdsl.isa import TargetISA, EncRecipe, OperandConstraint  # noqa
except ImportError:
    pass


def gen_recipe(recipe, fmt):
    # type: (EncRecipe, srcgen.Formatter) -> None
    """
    Generate code to handle a single recipe.

    - Unpack the instruction data, knowing the format.
    - Determine register locations for operands with register constraints.
    - Determine stack slot locations for operands with stack constraints.
    - Call hand-written code for the actual emission.
    """
    iform = recipe.format
    nvops = iform.num_value_operands
    want_args = any(isinstance(i, RegClass) or isinstance(i, Stack)
                    for i in recipe.ins)
    assert not want_args or nvops > 0 or iform.has_value_list
    want_outs = any(isinstance(o, RegClass) or isinstance(o, Stack)
                    for o in recipe.outs)

    # Regmove instructions get special treatment.
    is_regmove = (recipe.format.name in ('RegMove', 'RegSpill', 'RegFill'))

    # First unpack the instruction.
    with fmt.indented(
            'if let InstructionData::{} {{'.format(iform.name),
            '}'):
        fmt.line('opcode,')
        for f in iform.imm_fields:
            fmt.line('{},'.format(f.member))
        if want_args:
            if iform.has_value_list or nvops > 1:
                fmt.line('ref args,')
            else:
                fmt.line('arg,')
        fmt.line('..')
        fmt.outdented_line('} = func.dfg[inst] {')

        # Pass recipe arguments in this order: inputs, imm_fields, outputs.
        args = ''

        # Normalize to an `args` array.
        if want_args and not is_regmove:
            if iform.has_value_list:
                fmt.line('let args = args.as_slice(&func.dfg.value_lists);')
            elif nvops == 1:
                fmt.line('let args = [arg];')
            args += unwrap_values(recipe.ins, 'in', 'args', fmt)

        for f in iform.imm_fields:
            args += ', ' + f.member

        # Unwrap interesting output arguments.
        if want_outs:
            if len(recipe.outs) == 1:
                fmt.line('let results = [func.dfg.first_result(inst)];')
            else:
                fmt.line('let results = func.dfg.inst_results(inst);')
            args += unwrap_values(recipe.outs, 'out', 'results', fmt)

        # Special handling for regmove instructions. Update the register
        # diversion tracker.
        if recipe.format.name == 'RegMove':
            fmt.line('divert.regmove(arg, src, dst);')
        elif recipe.format.name == 'RegSpill':
            fmt.line('divert.regspill(arg, src, dst);')
        elif recipe.format.name == 'RegFill':
            fmt.line('divert.regfill(arg, src, dst);')

        # Call hand-written code. If the recipe contains a code snippet, use
        # that. Otherwise cal a recipe function in the target ISA's binemit
        # module.
        if recipe.emit is None:
            fmt.format(
                    'return recipe_{}(func, inst, sink, bits{});',
                    recipe.name.lower(), args)
        else:
            fmt.multi_line(recipe.emit)
            fmt.line('return;')


def unwrap_values(args, prefix, values, fmt):
    # type: (Sequence[OperandConstraint], str, str, srcgen.Formatter) -> str  # noqa
    """
    Emit code that unwraps values living in registers or stack slots.

    :param args: Input or output constraints.
    :param prefix: Prefix to be used for the generated local variables.
    :param values: Name of slice containing the values to be unwrapped.
    :returns: Comma separated list of the generated variables
    """
    varlist = ''
    for i, cst in enumerate(args):
        if isinstance(cst, RegClass):
            v = '{}_reg{}'.format(prefix, i)
            varlist += ', ' + v
            fmt.format(
                    'let {} = divert.reg({}[{}], &func.locations);',
                    v, values, i)
        elif isinstance(cst, Stack):
            v = '{}_stk{}'.format(prefix, i)
            varlist += ', ' + v
            with fmt.indented(
                    'let {} = StackRef::masked('.format(v),
                    ').unwrap();'):
                fmt.format('divert.stack({}[{}], &func.locations),', values, i)
                fmt.format('{},', cst.stack_base_mask())
                fmt.line('&func.stack_slots,')
    return varlist


def gen_isa(isa, fmt):
    # type: (TargetISA, srcgen.Formatter) -> None
    """
    Generate Binary emission code for `isa`.
    """
    fmt.doc_comment(
            '''
            Emit binary machine code for `inst` for the {} ISA.
            '''.format(isa.name))
    if len(isa.all_recipes) == 0:
        # No encoding recipes: Emit a stub.
        with fmt.indented('pub fn emit_inst<CS: CodeSink + ?Sized>('):
            fmt.line('func: &Function,')
            fmt.line('inst: Inst,')
            fmt.line('_divert: &mut RegDiversions,')
            fmt.line('_sink: &mut CS,')
        with fmt.indented(') {', '}'):
            fmt.line('bad_encoding(func, inst)')
    else:
        fmt.line('#[allow(unused_variables, unreachable_code)]')
        with fmt.indented('pub fn emit_inst<CS: CodeSink + ?Sized>('):
            fmt.line('func: &Function,')
            fmt.line('inst: Inst,')
            fmt.line('divert: &mut RegDiversions,')
            fmt.line('sink: &mut CS,')
        with fmt.indented(') {', '}'):
            fmt.line('let encoding = func.encodings[inst];')
            fmt.line('let bits = encoding.bits();')
            with fmt.indented('match func.encodings[inst].recipe() {', '}'):
                for i, recipe in enumerate(isa.all_recipes):
                    fmt.comment('Recipe {}'.format(recipe.name))
                    with fmt.indented('{} => {{'.format(i), '}'):
                        gen_recipe(recipe, fmt)
                fmt.line('_ => {},')
            # Allow for un-encoded ghost instructions.
            # Verifier checks the details.
            with fmt.indented('if encoding.is_legal() {', '}'):
                fmt.line('bad_encoding(func, inst);')


def generate(isas, out_dir):
    # type: (Sequence[TargetISA], str) -> None
    for isa in isas:
        fmt = srcgen.Formatter()
        gen_isa(isa, fmt)
        fmt.update_file('binemit-{}.rs'.format(isa.name), out_dir)

"""
Generate binary emission code for each ISA.
"""

from __future__ import absolute_import
import srcgen

try:
    from typing import Sequence, List  # noqa
    from cdsl.isa import TargetISA  # noqa
except ImportError:
    pass


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
        with fmt.indented(
                'pub fn emit_inst<CS: CodeSink + ?Sized>'
                '(func: &Function, inst: Inst, _sink: &mut CS) {', '}'):
            fmt.line('bad_encoding(func, inst)')
    else:
        with fmt.indented(
                'pub fn emit_inst<CS: CodeSink + ?Sized>'
                '(func: &Function, inst: Inst, sink: &mut CS) {', '}'):
            with fmt.indented('match func.encodings[inst].recipe() {', '}'):
                for i, recipe in enumerate(isa.all_recipes):
                    fmt.line('{} => recipe_{}(func, inst, sink),'.format(
                        i, recipe.name.lower()))
                fmt.line('_ => bad_encoding(func, inst),')


def generate(isas, out_dir):
    # type: (Sequence[TargetISA], str) -> None
    for isa in isas:
        fmt = srcgen.Formatter()
        gen_isa(isa, fmt)
        fmt.update_file('binemit-{}.rs'.format(isa.name), out_dir)

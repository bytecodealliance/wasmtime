"""
Generate register bank descriptions for each ISA.
"""

from __future__ import absolute_import
import srcgen

try:
    from typing import Sequence  # noqa
    from cdsl.isa import TargetISA  # noqa
    from cdsl.registers import RegBank  # noqa
except ImportError:
    pass


def gen_regbank(regbank, fmt):
    # type: (RegBank, srcgen.Formatter) -> None
    """
    Emit a static data definition for regbank.
    """
    with fmt.indented(
            'RegBank {{'.format(regbank.name), '},'):
        fmt.line('name: "{}",'.format(regbank.name))
        fmt.line('first_unit: {},'.format(regbank.first_unit))
        fmt.line('units: {},'.format(regbank.units))
        fmt.line(
                'names: &[{}],'
                .format(', '.join('"{}"'.format(n) for n in regbank.names)))
        fmt.line('prefix: "{}",'.format(regbank.prefix))


def gen_isa(isa, fmt):
    # type: (TargetISA, srcgen.Formatter) -> None
    """
    Generate register tables for isa.
    """
    if not isa.regbanks:
        print('cargo:warning={} has no register banks'.format(isa.name))
    with fmt.indented('pub static INFO: RegInfo = RegInfo {', '};'):
        # Bank descriptors.
        with fmt.indented('banks: &[', '],'):
            for regbank in isa.regbanks:
                gen_regbank(regbank, fmt)


def generate(isas, out_dir):
    # type: (Sequence[TargetISA], str) -> None
    for isa in isas:
        fmt = srcgen.Formatter()
        gen_isa(isa, fmt)
        fmt.update_file('registers-{}.rs'.format(isa.name), out_dir)

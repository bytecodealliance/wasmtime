"""
Generate sources with settings.
"""

import srcgen
from cretonne import BoolSetting, NumSetting, settings


def layout_group(sgrp):
    """
    Layout the settings in sgrp, assigning byte and bit offsets.

    Return the next unused byte offset.
    """
    # Byte offset where booleans are allocated.
    bool_byte = -1
    # Next available bit number in bool_byte.
    bool_bit = 10
    # Next available whole byte.
    next_byte = 0

    for setting in sgrp.settings:
        if isinstance(setting, BoolSetting):
            # Allocate a bit from bool_byte.
            if bool_bit > 7:
                bool_byte = next_byte
                next_byte += 1
                bool_bit = 0
            setting.byte_offset = bool_byte
            setting.bit_offset = bool_bit
            bool_bit += 1
        else:
            # This is a numerical or enumerated setting. Allocate a single
            # byte.
            setting.byte_offset = next_byte
            next_byte += 1

    return next_byte


def gen_getter(setting, fmt):
    """
    Emit a getter function for `setting`.
    """
    fmt.doc_comment(setting.__doc__ + '.')

    if isinstance(setting, BoolSetting):
        proto = 'pub fn {}(&self) -> bool'.format(setting.name)
        with fmt.indented(proto + ' {', '}'):
            fmt.line('(self.bytes[{}] & (1 << {})) != 0'.format(
                setting.byte_offset,
                setting.bit_offset))
    elif isinstance(setting, NumSetting):
        proto = 'pub fn {}(&self) -> u8'.format(setting.name)
        with fmt.indented(proto + ' {', '}'):
            fmt.line('self.bytes[{}]'.format(setting.byte_offset))
    else:
        raise AssertionError("Unknown setting kind")


def gen_getters(sgrp, fmt):
    """
    Emit getter functions for all the settings in fmt.
    """
    fmt.doc_comment("User-defined settings.")
    with fmt.indented('impl Settings {', '}'):
        for setting in sgrp.settings:
            gen_getter(setting, fmt)


def gen_default(sgrp, byte_size, fmt):
    """
    Emit a Default impl for Settings.
    """
    v = [0] * byte_size
    for setting in sgrp.settings:
        v[setting.byte_offset] |= setting.default_byte()

    with fmt.indented('impl Default for Settings {', '}'):
        fmt.doc_comment('Return a `Settings` object with default values.')
        with fmt.indented('fn default() -> Settings {', '}'):
            with fmt.indented('Settings {', '}'):
                vs = ', '.join('{:#04x}'.format(x) for x in v)
                fmt.line('bytes: [ {} ],'.format(vs))


def gen_group(sgrp, fmt):
    """
    Generate a Settings struct representing `sgrp`.
    """
    byte_size = layout_group(sgrp)

    fmt.doc_comment('Settings group `{}`.'.format(sgrp.name))
    with fmt.indented('pub struct Settings {', '}'):
        fmt.line('bytes: [u8; {}],'.format(byte_size))

    gen_getters(sgrp, fmt)
    gen_default(sgrp, byte_size, fmt)


def generate(isas, out_dir):
    # Generate shared settings.
    fmt = srcgen.Formatter()
    gen_group(settings.group, fmt)
    fmt.update_file('settings.rs', out_dir)

    # Generate ISA-specific settings.
    for isa in isas:
        if isa.settings:
            fmt = srcgen.Formatter()
            gen_group(isa.settings, fmt)
            fmt.update_file('settings-{}.rs'.format(isa.name), out_dir)

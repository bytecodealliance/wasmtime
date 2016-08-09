"""
Generate sources with settings.
"""

import srcgen
from unique_table import UniqueSeqTable
import constant_hash
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


def gen_descriptors(sgrp, fmt):
    """
    Generate the DESCRIPTORS and ENUMERATORS tables.
    """

    enums = UniqueSeqTable()

    with fmt.indented(
            'const DESCRIPTORS: [Descriptor; {}] = ['
            .format(len(sgrp.settings)),
            '];'):
        for idx, setting in enumerate(sgrp.settings):
            setting.descriptor_index = idx
            with fmt.indented('Descriptor {', '},'):
                fmt.line('name: "{}",'.format(setting.name))
                fmt.line('offset: {},'.format(setting.byte_offset))
                if isinstance(setting, BoolSetting):
                    fmt.line(
                            'detail: Detail::Bool {{ bit: {} }},'
                            .format(setting.bit_offset))
                elif isinstance(setting, NumSetting):
                    fmt.line('detail: Detail::Num,')
                else:
                    raise AssertionError("Unknown setting kind")

    with fmt.indented(
            'const ENUMERATORS: [&\'static str; {}] = ['
            .format(len(enums.table)),
            '];'):
        for txt in enums.table:
            fmt.line('"{}",'.format(txt))

    def hash_setting(s):
        return constant_hash.simple_hash(s.name)

    hash_table = constant_hash.compute_quadratic(sgrp.settings, hash_setting)
    if len(sgrp.settings) > 0xffff:
        ty = 'u32'
    elif len(sgrp.settings) > 0xff:
        ty = 'u16'
    else:
        ty = 'u8'

    with fmt.indented(
            'const HASH_TABLE: [{}; {}] = ['
            .format(ty, len(hash_table)),
            '];'):
        for h in hash_table:
            if h is None:
                fmt.line('{},'.format(len(sgrp.settings)))
            else:
                fmt.line('{},'.format(h.descriptor_index))


def gen_stringwise(sgrp, fmt):
    """
    Generate the Stringwise implementation and supporting tables.
    """

    with fmt.indented('impl Stringwise for Settings {', '}'):
        with fmt.indented(
                'fn lookup_mut(&mut self, name: &str)' +
                '-> Result<(Detail, &mut u8)> {',
                '}'):
            fmt.line('use simple_hash::simple_hash;')
            fmt.line('let tlen = HASH_TABLE.len();')
            fmt.line('assert!(tlen.is_power_of_two());')
            fmt.line('let mut idx = simple_hash(name) as usize;')
            fmt.line('let mut step: usize = 0;')
            with fmt.indented('loop {', '}'):
                fmt.line('idx = idx % tlen;')
                fmt.line('let entry = HASH_TABLE[idx] as usize;')
                with fmt.indented('if entry >= DESCRIPTORS.len() {', '}'):
                    fmt.line('return Err(Error::BadName)')
                with fmt.indented('if DESCRIPTORS[entry].name == name {', '}'):
                    fmt.line(
                            'return Ok((DESCRIPTORS[entry].detail, ' +
                            '&mut self.bytes[DESCRIPTORS[entry].offset ' +
                            'as usize]))')
                fmt.line('step += 1;')
                fmt.line('assert!(step < tlen);')
                fmt.line('idx += step;')

        with fmt.indented(
                'fn enumerator(&self, enums: u16, value: u8)' +
                '-> &\'static str {',
                '}'):
            fmt.line('ENUMERATORS[enums as usize + value as usize]')


def gen_display(sgrp, fmt):
    """
    Generate the Display impl for Settings.
    """
    with fmt.indented('impl fmt::Display for Settings {', '}'):
        with fmt.indented(
                'fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {',
                '}'):
            fmt.line('try!(writeln!(f, "[{}]"));'.format(sgrp.name))
            with fmt.indented('for d in &DESCRIPTORS {', '}'):
                fmt.line('try!(write!(f, "{} = ", d.name));')
                fmt.line(
                        'try!(self.format_toml_value(d.detail,' +
                        'self.bytes[d.offset as usize], f));')
                fmt.line('try!(writeln!(f, ""));')
            fmt.line('Ok(())')


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
    gen_descriptors(sgrp, fmt)
    gen_stringwise(sgrp, fmt)
    gen_display(sgrp, fmt)


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

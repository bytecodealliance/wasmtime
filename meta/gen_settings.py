"""
Generate sources with settings.
"""
from __future__ import absolute_import
import srcgen
from unique_table import UniqueSeqTable
import constant_hash
from cretonne import camel_case, BoolSetting, NumSetting, EnumSetting, settings


def layout_group(sgrp):
    """
    Layout the settings in sgrp, assigning byte and bit offsets.

    Return the number of bytes needed for settings and the total number of
    bytes needed when including predicates.
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

    settings_size = next_byte

    # Allocate bits for all the precomputed predicates.
    for pred in sgrp.predicates:
        # Allocate a bit from bool_byte.
        if bool_bit > 7:
            bool_byte = next_byte
            next_byte += 1
            bool_bit = 0
        pred.byte_offset = bool_byte
        pred.bit_offset = bool_bit
        bool_bit += 1

    return (settings_size, next_byte)


def gen_enum_types(sgrp, fmt):
    """
    Emit enum types for any enum settings.
    """
    for setting in sgrp.settings:
        if not isinstance(setting, EnumSetting):
            continue
        ty = camel_case(setting.name)
        fmt.line('#[derive(Debug, PartialEq, Eq)]')
        fmt.line(
                'pub enum {} {{ {} }}'
                .format(ty, ", ".join(camel_case(v) for v in setting.values)))


def gen_getter(setting, fmt):
    """
    Emit a getter function for `setting`.
    """
    fmt.doc_comment(setting.__doc__)

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
    elif isinstance(setting, EnumSetting):
        ty = camel_case(setting.name)
        proto = 'pub fn {}(&self) -> {}'.format(setting.name, ty)
        with fmt.indented(proto + ' {', '}'):
            with fmt.indented(
                    'match self.bytes[{}] {{'
                    .format(setting.byte_offset), '}'):
                for i, v in enumerate(setting.values):
                    fmt.line('{} => {}::{},'.format(i, ty, camel_case(v)))
                fmt.line('_ => panic!("Invalid enum value")')
    else:
        raise AssertionError("Unknown setting kind")


def gen_pred_getter(pred, fmt):
    """
    Emit a getter for a pre-computed predicate.
    """
    fmt.doc_comment('Computed predicate `{}`.'.format(pred.rust_predicate(0)))
    proto = 'pub fn {}(&self) -> bool'.format(pred.name)
    with fmt.indented(proto + ' {', '}'):
        fmt.line('(self.bytes[{}] & (1 << {})) != 0'.format(
            pred.byte_offset,
            pred.bit_offset))


def gen_getters(sgrp, fmt):
    """
    Emit getter functions for all the settings in fmt.
    """
    fmt.doc_comment("User-defined settings.")
    with fmt.indented('impl Flags {', '}'):
        for setting in sgrp.settings:
            gen_getter(setting, fmt)
        for pred in sgrp.predicates:
            gen_pred_getter(pred, fmt)


def gen_descriptors(sgrp, fmt):
    """
    Generate the DESCRIPTORS and ENUMERATORS tables.
    """

    enums = UniqueSeqTable()

    with fmt.indented(
            'static DESCRIPTORS: [detail::Descriptor; {}] = ['
            .format(len(sgrp.settings)),
            '];'):
        for idx, setting in enumerate(sgrp.settings):
            setting.descriptor_index = idx
            with fmt.indented('detail::Descriptor {', '},'):
                fmt.line('name: "{}",'.format(setting.name))
                fmt.line('offset: {},'.format(setting.byte_offset))
                if isinstance(setting, BoolSetting):
                    fmt.line(
                            'detail: detail::Detail::Bool {{ bit: {} }},'
                            .format(setting.bit_offset))
                elif isinstance(setting, NumSetting):
                    fmt.line('detail: detail::Detail::Num,')
                elif isinstance(setting, EnumSetting):
                    offs = enums.add(setting.values)
                    fmt.line(
                            'detail: detail::Detail::Enum ' +
                            '{{ last: {}, enumerators: {} }},'
                            .format(len(setting.values)-1, offs))
                else:
                    raise AssertionError("Unknown setting kind")

    with fmt.indented(
            'static ENUMERATORS: [&\'static str; {}] = ['
            .format(len(enums.table)),
            '];'):
        for txt in enums.table:
            fmt.line('"{}",'.format(txt))

    def hash_setting(s):
        return constant_hash.simple_hash(s.name)

    hash_table = constant_hash.compute_quadratic(sgrp.settings, hash_setting)
    with fmt.indented(
            'static HASH_TABLE: [u16; {}] = ['
            .format(len(hash_table)),
            '];'):
        for h in hash_table:
            if h is None:
                fmt.line('0xffff,')
            else:
                fmt.line('{},'.format(h.descriptor_index))


def gen_template(sgrp, settings_size, fmt):
    """
    Emit a Template constant.
    """
    v = [0] * settings_size
    for setting in sgrp.settings:
        v[setting.byte_offset] |= setting.default_byte()

    with fmt.indented(
            'static TEMPLATE: detail::Template = detail::Template {', '};'):
        fmt.line('name: "{}",'.format(sgrp.name))
        fmt.line('descriptors: &DESCRIPTORS,')
        fmt.line('enumerators: &ENUMERATORS,')
        fmt.line('hash_table: &HASH_TABLE,')
        vs = ', '.join('{:#04x}'.format(x) for x in v)
        fmt.line('defaults: &[ {} ],'.format(vs))

    fmt.doc_comment(
            'Create a `settings::Builder` for the {} settings group.'
            .format(sgrp.name))
    with fmt.indented('pub fn builder() -> Builder {', '}'):
        fmt.line('Builder::new(&TEMPLATE)')


def gen_display(sgrp, fmt):
    """
    Generate the Display impl for Flags.
    """
    with fmt.indented('impl fmt::Display for Flags {', '}'):
        with fmt.indented(
                'fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {',
                '}'):
            fmt.line('try!(writeln!(f, "[{}]"));'.format(sgrp.name))
            with fmt.indented('for d in &DESCRIPTORS {', '}'):
                fmt.line('try!(write!(f, "{} = ", d.name));')
                fmt.line(
                        'try!(TEMPLATE.format_toml_value(d.detail,' +
                        'self.bytes[d.offset as usize], f));')
                fmt.line('try!(writeln!(f, ""));')
            fmt.line('Ok(())')


def gen_constructor(sgrp, settings_size, byte_size, parent, fmt):
    """
    Generate a Flags constructor.
    """

    with fmt.indented('impl Flags {', '}'):
        args = 'builder: Builder'
        if sgrp.parent:
            p = sgrp.parent
            args = '{}: &{}::Flags, {}'.format(p.name, p.qual_mod, args)
        with fmt.indented(
                'pub fn new({}) -> Flags {{'.format(args), '}'):
            fmt.line('let bvec = builder.finish("{}");'.format(sgrp.name))
            fmt.line('let mut bytes = [0; {}];'.format(byte_size))
            fmt.line('assert_eq!(bvec.len(), {});'.format(settings_size))
            with fmt.indented(
                    'for (i, b) in bvec.into_iter().enumerate() {', '}'):
                fmt.line('bytes[i] = b;')

            # Stop here without predicates.
            if len(sgrp.predicates) == 0:
                fmt.line('Flags { bytes: bytes }')
                return

            # Now compute the predicates.
            fmt.line(
                    'let mut {} = Flags {{ bytes: bytes }};'
                    .format(sgrp.name))

            for pred in sgrp.predicates:
                fmt.comment('Precompute: {}.'.format(pred.name))
                with fmt.indented(
                        'if {} {{'.format(pred.rust_predicate(0)),
                        '}'):
                    fmt.line(
                            '{}.bytes[{}] |= 1 << {};'
                            .format(
                                sgrp.name, pred.byte_offset, pred.bit_offset))

            fmt.line(sgrp.name)


def gen_group(sgrp, fmt):
    """
    Generate a Flags struct representing `sgrp`.
    """
    settings_size, byte_size = layout_group(sgrp)

    fmt.line('#[derive(Clone)]')
    fmt.doc_comment('Flags group `{}`.'.format(sgrp.name))
    with fmt.indented('pub struct Flags {', '}'):
        fmt.line('bytes: [u8; {}],'.format(byte_size))

    gen_constructor(sgrp, settings_size, byte_size, None, fmt)
    gen_enum_types(sgrp, fmt)
    gen_getters(sgrp, fmt)
    gen_descriptors(sgrp, fmt)
    gen_template(sgrp, settings_size, fmt)
    gen_display(sgrp, fmt)


def generate(isas, out_dir):
    # Generate shared settings.
    fmt = srcgen.Formatter()
    settings.group.qual_mod = 'settings'
    gen_group(settings.group, fmt)
    fmt.update_file('settings.rs', out_dir)

    # Generate ISA-specific settings.
    for isa in isas:
        if isa.settings:
            isa.settings.qual_mod = 'isa::{}::settings'.format(
                    isa.settings.name)
            fmt = srcgen.Formatter()
            gen_group(isa.settings, fmt)
            fmt.update_file('settings-{}.rs'.format(isa.name), out_dir)

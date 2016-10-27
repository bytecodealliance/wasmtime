"""
Generate sources with settings.
"""
from __future__ import absolute_import
import srcgen
from unique_table import UniqueSeqTable
import constant_hash
from cretonne import camel_case, BoolSetting, NumSetting, EnumSetting, settings


def gen_enum_types(sgrp, fmt):
    """
    Emit enum types for any enum settings.
    """
    for setting in sgrp.settings:
        if not isinstance(setting, EnumSetting):
            continue
        ty = camel_case(setting.name)
        fmt.doc_comment('Values for {}.'.format(setting))
        fmt.line('#[derive(Debug, PartialEq, Eq)]')
        with fmt.indented('pub enum {} {{'.format(ty), '}'):
            for v in setting.values:
                fmt.doc_comment('`{}`.'.format(v))
                fmt.line(camel_case(v) + ',')


def gen_getter(setting, sgrp, fmt):
    """
    Emit a getter function for `setting`.
    """
    fmt.doc_comment(setting.__doc__)

    if isinstance(setting, BoolSetting):
        proto = 'pub fn {}(&self) -> bool'.format(setting.name)
        with fmt.indented(proto + ' {', '}'):
            fmt.line(
                    'self.numbered_predicate({})'
                    .format(sgrp.predicate_number[setting]))
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


def gen_pred_getter(pred, sgrp, fmt):
    """
    Emit a getter for a pre-computed predicate.
    """
    fmt.doc_comment('Computed predicate `{}`.'.format(pred.rust_predicate(0)))
    proto = 'pub fn {}(&self) -> bool'.format(pred.name)
    with fmt.indented(proto + ' {', '}'):
        fmt.line(
                'self.numbered_predicate({})'
                .format(sgrp.predicate_number[pred]))


def gen_getters(sgrp, fmt):
    """
    Emit getter functions for all the settings in fmt.
    """
    fmt.doc_comment("User-defined settings.")
    with fmt.indented('impl Flags {', '}'):
        fmt.doc_comment('Dynamic numbered predicate getter.')
        with fmt.indented(
                'pub fn numbered_predicate(&self, p: usize) -> bool {', '}'):
            fmt.line(
                    'self.bytes[{} + p/8] & (1 << (p%8)) != 0'
                    .format(sgrp.boolean_offset))
        for setting in sgrp.settings:
            gen_getter(setting, sgrp, fmt)
        for pred in sgrp.named_predicates:
            gen_pred_getter(pred, sgrp, fmt)


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


def gen_template(sgrp, fmt):
    """
    Emit a Template constant.
    """
    v = [0] * sgrp.settings_size
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


def gen_constructor(sgrp, parent, fmt):
    """
    Generate a Flags constructor.
    """

    with fmt.indented('impl Flags {', '}'):
        args = 'builder: &Builder'
        if sgrp.parent:
            p = sgrp.parent
            args = '{}: &{}::Flags, {}'.format(p.name, p.qual_mod, args)
        fmt.doc_comment('Create flags {} settings group.'.format(sgrp.name))
        with fmt.indented(
                'pub fn new({}) -> Flags {{'.format(args), '}'):
            fmt.line('let bvec = builder.state_for("{}");'.format(sgrp.name))
            fmt.line('let mut bytes = [0; {}];'.format(sgrp.byte_size()))
            fmt.line('assert_eq!(bvec.len(), {});'.format(sgrp.settings_size))
            with fmt.indented(
                    'for (i, b) in bvec.iter().enumerate() {', '}'):
                fmt.line('bytes[i] = *b;')

            # Stop here without predicates.
            if len(sgrp.predicate_number) == sgrp.boolean_settings:
                fmt.line('Flags { bytes: bytes }')
                return

            # Now compute the predicates.
            fmt.line(
                    'let mut {} = Flags {{ bytes: bytes }};'
                    .format(sgrp.name))

            for pred, number in sgrp.predicate_number.items():
                # Don't compute our own settings.
                if number < sgrp.boolean_settings:
                    continue
                if pred.name:
                    fmt.comment(
                            'Precompute #{} ({}).'.format(number, pred.name))
                else:
                    fmt.comment('Precompute #{}.'.format(number))
                with fmt.indented(
                        'if {} {{'.format(pred.rust_predicate(0)),
                        '}'):
                    fmt.line(
                            '{}.bytes[{}] |= 1 << {};'
                            .format(
                                sgrp.name,
                                sgrp.boolean_offset + number // 8,
                                number % 8))

            fmt.line(sgrp.name)


def gen_group(sgrp, fmt):
    """
    Generate a Flags struct representing `sgrp`.
    """
    fmt.line('#[derive(Clone)]')
    fmt.doc_comment('Flags group `{}`.'.format(sgrp.name))
    with fmt.indented('pub struct Flags {', '}'):
        fmt.line('bytes: [u8; {}],'.format(sgrp.byte_size()))

    gen_constructor(sgrp, None, fmt)
    gen_enum_types(sgrp, fmt)
    gen_getters(sgrp, fmt)
    gen_descriptors(sgrp, fmt)
    gen_template(sgrp, fmt)
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

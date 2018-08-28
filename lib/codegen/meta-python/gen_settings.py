"""
Generate sources with settings.
"""
from __future__ import absolute_import
import srcgen
from unique_table import UniqueSeqTable
import constant_hash
from cdsl import camel_case
from cdsl.settings import BoolSetting, NumSetting, EnumSetting
from base import settings

try:
    from typing import Sequence, Set, Tuple, List, Union, TYPE_CHECKING  # noqa
    if TYPE_CHECKING:
        from cdsl.isa import TargetISA  # noqa
        from cdsl.settings import Setting, Preset, SettingGroup  # noqa
        from cdsl.predicates import Predicate, PredContext  # noqa
except ImportError:
    pass


def gen_to_and_from_str(ty, values, fmt):
    # type: (str, Tuple[str, ...], srcgen.Formatter) -> None
    """
    Emit Display and FromStr implementations for enum settings.
    """
    with fmt.indented('impl fmt::Display for {} {{'.format(ty), '}'):
        with fmt.indented(
                'fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {',
                '}'):
            with fmt.indented('f.write_str(match *self {', '})'):
                for v in values:
                    fmt.line('{}::{} => "{}",'
                             .format(ty, camel_case(v), v))

    with fmt.indented('impl str::FromStr for {} {{'.format(ty), '}'):
        fmt.line('type Err = ();')
        with fmt.indented(
                'fn from_str(s: &str) -> Result<Self, Self::Err> {',
                '}'):
            with fmt.indented('match s {', '}'):
                for v in values:
                    fmt.line('"{}" => Ok({}::{}),'
                             .format(v, ty, camel_case(v)))
                fmt.line('_ => Err(()),')


def gen_enum_types(sgrp, fmt):
    # type: (SettingGroup, srcgen.Formatter) -> None
    """
    Emit enum types for any enum settings.
    """
    for setting in sgrp.settings:
        if not isinstance(setting, EnumSetting):
            continue
        ty = camel_case(setting.name)
        fmt.doc_comment('Values for `{}`.'.format(setting))
        fmt.line('#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]')
        with fmt.indented('pub enum {} {{'.format(ty), '}'):
            for v in setting.values:
                fmt.doc_comment('`{}`.'.format(v))
                fmt.line(camel_case(v) + ',')

        gen_to_and_from_str(ty, setting.values, fmt)


def gen_getter(setting, sgrp, fmt):
    # type: (Setting, SettingGroup, srcgen.Formatter) -> None
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
            m = srcgen.Match('self.bytes[{}]'.format(setting.byte_offset))
            for i, v in enumerate(setting.values):
                m.arm(str(i), [], '{}::{}'.format(ty, camel_case(v)))
            m.arm('_', [], 'panic!("Invalid enum value")')
            fmt.match(m)
    else:
        raise AssertionError("Unknown setting kind")


def gen_pred_getter(name, pred, sgrp, fmt):
    # type: (str, Predicate, SettingGroup, srcgen.Formatter) -> None
    """
    Emit a getter for a named pre-computed predicate.
    """
    fmt.doc_comment('Computed predicate `{}`.'.format(pred.rust_predicate(0)))
    proto = 'pub fn {}(&self) -> bool'.format(name)
    with fmt.indented(proto + ' {', '}'):
        fmt.line(
                'self.numbered_predicate({})'
                .format(sgrp.predicate_number[pred]))


def gen_getters(sgrp, fmt):
    # type: (SettingGroup, srcgen.Formatter) -> None
    """
    Emit getter functions for all the settings in fmt.
    """
    fmt.doc_comment("User-defined settings.")
    fmt.line('#[allow(dead_code)]')
    with fmt.indented('impl Flags {', '}'):
        fmt.doc_comment('Get a view of the boolean predicates.')
        with fmt.indented(
                'pub fn predicate_view(&self) -> ::settings::PredicateView {',
                '}'):
            fmt.format(
                    '::settings::PredicateView::new(&self.bytes[{}..])',
                    sgrp.boolean_offset)
        if sgrp.settings:
            fmt.doc_comment('Dynamic numbered predicate getter.')
            with fmt.indented(
                    'fn numbered_predicate(&self, p: usize) -> bool {', '}'):
                fmt.line(
                        'self.bytes[{} + p / 8] & (1 << (p % 8)) != 0'
                        .format(sgrp.boolean_offset))
        for setting in sgrp.settings:
            gen_getter(setting, sgrp, fmt)
        for name, pred in sgrp.named_predicates.items():
            gen_pred_getter(name, pred, sgrp, fmt)


def gen_descriptors(sgrp, fmt):
    # type: (SettingGroup, srcgen.Formatter) -> None
    """
    Generate the DESCRIPTORS, ENUMERATORS, and PRESETS tables.
    """

    enums = UniqueSeqTable()

    with fmt.indented(
            'static DESCRIPTORS: [detail::Descriptor; {}] = ['
            .format(len(sgrp.settings) + len(sgrp.presets)),
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

        for idx, preset in enumerate(sgrp.presets):
            preset.descriptor_index = len(sgrp.settings) + idx
            with fmt.indented('detail::Descriptor {', '},'):
                fmt.line('name: "{}",'.format(preset.name))
                fmt.line('offset: {},'.format(idx * sgrp.settings_size))
                fmt.line('detail: detail::Detail::Preset,')

    with fmt.indented(
            'static ENUMERATORS: [&str; {}] = ['
            .format(len(enums.table)),
            '];'):
        for txt in enums.table:
            fmt.line('"{}",'.format(txt))

    def hash_setting(s):
        # type: (Union[Setting, Preset]) -> int
        return constant_hash.simple_hash(s.name)

    hash_elems = []  # type: List[Union[Setting, Preset]]
    hash_elems.extend(sgrp.settings)
    hash_elems.extend(sgrp.presets)
    hash_table = constant_hash.compute_quadratic(hash_elems, hash_setting)
    with fmt.indented(
            'static HASH_TABLE: [u16; {}] = ['
            .format(len(hash_table)),
            '];'):
        for h in hash_table:
            if h is None:
                fmt.line('0xffff,')
            else:
                fmt.line('{},'.format(h.descriptor_index))

    with fmt.indented(
            'static PRESETS: [(u8, u8); {}] = ['
            .format(len(sgrp.presets) * sgrp.settings_size),
            '];'):
        for preset in sgrp.presets:
            fmt.comment(preset.name)
            for mask, value in preset.layout():
                fmt.format('(0b{:08b}, 0b{:08b}),', mask, value)


def gen_template(sgrp, fmt):
    # type: (SettingGroup, srcgen.Formatter) -> None
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
        fmt.line('defaults: &[{}],'.format(vs))
        fmt.line('presets: &PRESETS,')

    fmt.doc_comment(
            'Create a `settings::Builder` for the {} settings group.'
            .format(sgrp.name))
    with fmt.indented('pub fn builder() -> Builder {', '}'):
        fmt.line('Builder::new(&TEMPLATE)')


def gen_display(sgrp, fmt):
    # type: (SettingGroup, srcgen.Formatter) -> None
    """
    Generate the Display impl for Flags.
    """
    with fmt.indented('impl fmt::Display for Flags {', '}'):
        with fmt.indented(
                'fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {',
                '}'):
            fmt.line('writeln!(f, "[{}]")?;'.format(sgrp.name))
            with fmt.indented('for d in &DESCRIPTORS {', '}'):
                with fmt.indented('if !d.detail.is_preset() {', '}'):
                    fmt.line('write!(f, "{} = ", d.name)?;')
                    fmt.line(
                            'TEMPLATE.format_toml_value(d.detail, ' +
                            'self.bytes[d.offset as usize], f)?;')
                    fmt.line('writeln!(f)?;')
            fmt.line('Ok(())')


def gen_constructor(sgrp, parent, fmt):
    # type: (SettingGroup, PredContext, srcgen.Formatter) -> None
    """
    Generate a Flags constructor.
    """

    with fmt.indented('impl Flags {', '}'):
        args = 'builder: Builder'
        if sgrp.parent:
            p = sgrp.parent
            args = '{}: &{}::Flags, {}'.format(p.name, p.qual_mod, args)
        fmt.doc_comment('Create flags {} settings group.'.format(sgrp.name))
        fmt.line('#[allow(unused_variables)]')
        with fmt.indented(
                'pub fn new({}) -> Self {{'.format(args), '}'):
            fmt.line('let bvec = builder.state_for("{}");'.format(sgrp.name))
            fmt.line(
                'let mut {} = Self {{ bytes: [0; {}] }};'
                .format(sgrp.name, sgrp.byte_size()))
            fmt.line(
                'debug_assert_eq!(bvec.len(), {});'
                .format(sgrp.settings_size))
            fmt.line(
                '{}.bytes[0..{}].copy_from_slice(&bvec);'
                .format(sgrp.name, sgrp.settings_size))

            # Now compute the predicates.
            for pred, number in sgrp.predicate_number.items():
                # Don't compute our own settings.
                if number < sgrp.boolean_settings:
                    continue
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
    # type: (SettingGroup, srcgen.Formatter) -> None
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
    # type: (Sequence[TargetISA], str) -> None
    # Generate shared settings.
    fmt = srcgen.Formatter()
    settings.group.qual_mod = 'settings'
    gen_group(settings.group, fmt)
    fmt.update_file('settings.rs', out_dir)

    # Generate ISA-specific settings.
    for isa in isas:
        isa.settings.qual_mod = 'isa::{}::settings'.format(
                isa.settings.name)
        fmt = srcgen.Formatter()
        gen_group(isa.settings, fmt)
        fmt.update_file('settings-{}.rs'.format(isa.name), out_dir)

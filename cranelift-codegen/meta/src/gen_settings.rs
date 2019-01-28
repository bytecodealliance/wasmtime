use crate::base;
use crate::cdsl::camel_case;
use crate::cdsl::isa::TargetIsa;
use crate::cdsl::settings::{
    BoolSetting, Predicate, Preset, Setting, SettingGroup, SpecificSetting,
};
use crate::constant_hash::{generate_table, simple_hash};
use crate::error;
use crate::srcgen::{Formatter, Match};
use crate::unique_table::UniqueTable;
use std::collections::HashMap;

enum ParentGroup {
    None,
    Shared,
}

/// Emits the constructor of the Flags structure.
fn gen_constructor(group: &SettingGroup, parent: ParentGroup, fmt: &mut Formatter) {
    let args = match parent {
        ParentGroup::None => "builder: Builder",
        ParentGroup::Shared => "shared: &settings::Flags, builder: Builder",
    };
    fmt.line("impl Flags {");
    fmt.indent(|fmt| {
        fmt.doc_comment(&format!("Create flags {} settings group.", group.name));
        fmt.line("#[allow(unused_variables)]");
        fmt.line(&format!("pub fn new({}) -> Self {{", args));
        fmt.indent(|fmt| {
            fmt.line(&format!(
                "let bvec = builder.state_for(\"{}\");",
                group.name
            ));
            fmt.line(&format!(
                "let mut {} = Self {{ bytes: [0; {}] }};",
                group.name,
                group.byte_size()
            ));
            fmt.line(&format!(
                "debug_assert_eq!(bvec.len(), {});",
                group.settings_size
            ));
            fmt.line(&format!(
                "{}.bytes[0..{}].copy_from_slice(&bvec);",
                group.name, group.settings_size
            ));

            // Now compute the predicates.
            for p in &group.predicates {
                fmt.comment(&format!("Precompute #{}.", p.number));
                fmt.line(&format!("if {} {{", p.render(group)));
                fmt.indent(|fmt| {
                    fmt.line(&format!(
                        "{}.bytes[{}] |= 1 << {};",
                        group.name,
                        group.bool_start_byte_offset + p.number / 8,
                        p.number % 8
                    ));
                });
                fmt.line("}");
            }

            fmt.line(group.name);
        });
        fmt.line("}");
    });
    fmt.line("}");
}

/// Emit Display and FromStr implementations for enum settings.
fn gen_to_and_from_str(name: &str, values: &[&'static str], fmt: &mut Formatter) {
    fmt.line(&format!("impl fmt::Display for {} {{", name));
    fmt.indent(|fmt| {
        fmt.line("fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {");
        fmt.indent(|fmt| {
            fmt.line("f.write_str(match *self {");
            fmt.indent(|fmt| {
                for v in values.iter() {
                    fmt.line(&format!("{}::{} => \"{}\",", name, camel_case(v), v));
                }
            });
            fmt.line("})");
        });
        fmt.line("}");
    });
    fmt.line("}");

    fmt.line(&format!("impl str::FromStr for {} {{", name));
    fmt.indent(|fmt| {
        fmt.line("type Err = ();");
        fmt.line("fn from_str(s: &str) -> Result<Self, Self::Err> {");
        fmt.indent(|fmt| {
            fmt.line("match s {");
            fmt.indent(|fmt| {
                for v in values.iter() {
                    fmt.line(&format!("\"{}\" => Ok({}::{}),", v, name, camel_case(v)));
                }
                fmt.line("_ => Err(()),");
            });
            fmt.line("}");
        });
        fmt.line("}");
    });
    fmt.line("}");
}

/// Emit real enum for the Enum settings.
fn gen_enum_types(group: &SettingGroup, fmt: &mut Formatter) {
    for setting in group.settings.iter() {
        let values = match setting.specific {
            SpecificSetting::Bool(_) | SpecificSetting::Num(_) => continue,
            SpecificSetting::Enum(ref values) => values,
        };
        let name = camel_case(setting.name);

        fmt.doc_comment(&format!("Values for `{}.{}`.", group.name, setting.name));
        fmt.line("#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]");
        fmt.line(&format!("pub enum {} {{", name));
        fmt.indent(|fmt| {
            for v in values.iter() {
                fmt.doc_comment(&format!("`{}`.", v));
                fmt.line(&format!("{},", camel_case(v)));
            }
        });
        fmt.line("}");

        gen_to_and_from_str(&name, values, fmt);
    }
}

/// Emit a getter function for `setting`.
fn gen_getter(setting: &Setting, fmt: &mut Formatter) {
    fmt.doc_comment(setting.comment);
    match setting.specific {
        SpecificSetting::Bool(BoolSetting {
            predicate_number, ..
        }) => {
            fmt.line(&format!("pub fn {}(&self) -> bool {{", setting.name));
            fmt.indent(|fmt| {
                fmt.line(&format!("self.numbered_predicate({})", predicate_number));
            });
            fmt.line("}");
        }
        SpecificSetting::Enum(ref values) => {
            let ty = camel_case(setting.name);
            fmt.line(&format!("pub fn {}(&self) -> {} {{", setting.name, ty));
            fmt.indent(|fmt| {
                let mut m = Match::new(format!("self.bytes[{}]", setting.byte_offset));
                for (i, v) in values.iter().enumerate() {
                    m.arm(
                        format!("{}", i),
                        vec![],
                        format!("{}::{}", ty, camel_case(v)),
                    );
                }
                m.arm("_", vec![], "panic!(\"Invalid enum value\")");
                fmt.add_match(m);
            });
            fmt.line("}");
        }
        SpecificSetting::Num(_) => {
            fmt.line(&format!("pub fn {}(&self) -> u8 {{", setting.name));
            fmt.indent(|fmt| {
                fmt.line(&format!("self.bytes[{}]", setting.byte_offset));
            });
            fmt.line("}");
        }
    }
}

fn gen_pred_getter(predicate: &Predicate, group: &SettingGroup, fmt: &mut Formatter) {
    fmt.doc_comment(&format!(
        "Computed predicate `{}`.",
        predicate.render(group)
    ));
    fmt.line(&format!("pub fn {}(&self) -> bool {{", predicate.name));
    fmt.indent(|fmt| {
        fmt.line(&format!("self.numbered_predicate({})", predicate.number));
    });
    fmt.line("}");
}

/// Emits getters for each setting value.
fn gen_getters(group: &SettingGroup, fmt: &mut Formatter) {
    fmt.doc_comment("User-defined settings.");
    fmt.line("#[allow(dead_code)]");
    fmt.line("impl Flags {");
    fmt.indent(|fmt| {
        fmt.doc_comment("Get a view of the boolean predicates.");
        fmt.line("pub fn predicate_view(&self) -> ::settings::PredicateView {");
        fmt.indent(|fmt| {
            fmt.line(&format!(
                "::settings::PredicateView::new(&self.bytes[{}..])",
                group.bool_start_byte_offset
            ));
        });
        fmt.line("}");

        if group.settings.len() > 0 {
            fmt.doc_comment("Dynamic numbered predicate getter.");
            fmt.line("fn numbered_predicate(&self, p: usize) -> bool {");
            fmt.indent(|fmt| {
                fmt.line(&format!(
                    "self.bytes[{} + p / 8] & (1 << (p % 8)) != 0",
                    group.bool_start_byte_offset
                ));
            });
            fmt.line("}");
        }

        for setting in &group.settings {
            gen_getter(&setting, fmt);
        }
        for predicate in &group.predicates {
            gen_pred_getter(&predicate, &group, fmt);
        }
    });
    fmt.line("}");
}

#[derive(Hash, PartialEq, Eq)]
enum SettingOrPreset<'a> {
    Setting(&'a Setting),
    Preset(&'a Preset),
}

impl<'a> SettingOrPreset<'a> {
    fn name(&self) -> &str {
        match *self {
            SettingOrPreset::Setting(s) => s.name,
            SettingOrPreset::Preset(p) => p.name,
        }
    }
}

/// Emits DESCRIPTORS, ENUMERATORS, HASH_TABLE and PRESETS.
fn gen_descriptors(group: &SettingGroup, fmt: &mut Formatter) {
    let mut enum_table: UniqueTable<&'static str> = UniqueTable::new();

    let mut descriptor_index_map: HashMap<SettingOrPreset, usize> = HashMap::new();

    // Generate descriptors.
    fmt.line(&format!(
        "static DESCRIPTORS: [detail::Descriptor; {}] = [",
        group.settings.len() + group.presets.len()
    ));
    fmt.indent(|fmt| {
        for (idx, setting) in group.settings.iter().enumerate() {
            fmt.line("detail::Descriptor {");
            fmt.indent(|fmt| {
                fmt.line(&format!("name: \"{}\",", setting.name));
                fmt.line(&format!("offset: {},", setting.byte_offset));
                match setting.specific {
                    SpecificSetting::Bool(BoolSetting { bit_offset, .. }) => {
                        fmt.line(&format!(
                            "detail: detail::Detail::Bool {{ bit: {} }},",
                            bit_offset
                        ));
                    }
                    SpecificSetting::Enum(ref values) => {
                        let offset = enum_table.add(values);
                        fmt.line(&format!(
                            "detail: detail::Detail::Enum {{ last: {}, enumerators: {} }},",
                            values.len() - 1,
                            offset
                        ));
                    }
                    SpecificSetting::Num(_) => {
                        fmt.line("detail: detail::Detail::Num,");
                    }
                }

                descriptor_index_map.insert(SettingOrPreset::Setting(setting), idx);
            });
            fmt.line("},");
        }

        for (idx, preset) in group.presets.iter().enumerate() {
            fmt.line("detail::Descriptor {");
            fmt.indent(|fmt| {
                fmt.line(&format!("name: \"{}\",", preset.name));
                fmt.line(&format!("offset: {},", (idx as u8) * group.settings_size));
                fmt.line("detail: detail::Detail::Preset,");
            });
            fmt.line("},");

            descriptor_index_map.insert(SettingOrPreset::Preset(preset), idx);
        }
    });
    fmt.line("];");

    // Generate enumerators.
    fmt.line(&format!(
        "static ENUMERATORS: [&str; {}] = [",
        enum_table.len()
    ));
    fmt.indent(|fmt| {
        for enum_val in enum_table.iter() {
            fmt.line(&format!("\"{}\",", enum_val));
        }
    });
    fmt.line("];");

    // Generate hash table.
    let mut hash_entries: Vec<SettingOrPreset> = Vec::new();
    hash_entries.extend(
        group
            .settings
            .iter()
            .map(|x| SettingOrPreset::Setting(x))
            .collect::<Vec<SettingOrPreset>>(),
    );
    hash_entries.extend(
        group
            .presets
            .iter()
            .map(|x| SettingOrPreset::Preset(x))
            .collect::<Vec<SettingOrPreset>>(),
    );
    let hash_table = generate_table(&hash_entries, |entry| simple_hash(entry.name()));
    fmt.line(&format!(
        "static HASH_TABLE: [u16; {}] = [",
        hash_table.len()
    ));
    fmt.indent(|fmt| {
        for h in &hash_table {
            match *h {
                Some(setting_or_preset) => fmt.line(&format!(
                    "{},",
                    &descriptor_index_map
                        .get(setting_or_preset)
                        .unwrap()
                        .to_string()
                )),
                None => fmt.line("0xffff,"),
            }
        }
    });
    fmt.line("];");

    // Generate presets.
    fmt.line(&format!(
        "static PRESETS: [(u8, u8); {}] = [",
        group.presets.len()
    ));
    fmt.indent(|fmt| {
        for preset in &group.presets {
            fmt.comment(preset.name);
            for (mask, value) in preset.layout(&group) {
                fmt.line(&format!("(0b{:08b}, 0b{:08b}),", mask, value));
            }
        }
    });
    fmt.line("];");
}

fn gen_template(group: &SettingGroup, fmt: &mut Formatter) {
    let mut default_bytes: Vec<u8> = vec![0; group.settings_size as usize];
    for setting in &group.settings {
        *default_bytes.get_mut(setting.byte_offset as usize).unwrap() |= setting.default_byte();
    }

    let default_bytes: Vec<String> = default_bytes
        .iter()
        .map(|x| format!("{:#04x}", x))
        .collect();
    let default_bytes_str = default_bytes.join(", ");

    fmt.line("static TEMPLATE: detail::Template = detail::Template {");
    fmt.indent(|fmt| {
        fmt.line(&format!("name: \"{}\",", group.name));
        fmt.line("descriptors: &DESCRIPTORS,");
        fmt.line("enumerators: &ENUMERATORS,");
        fmt.line("hash_table: &HASH_TABLE,");
        fmt.line(&format!("defaults: &[{}],", default_bytes_str));
        fmt.line("presets: &PRESETS,");
    });
    fmt.line("};");

    fmt.doc_comment(&format!(
        "Create a `settings::Builder` for the {} settings group.",
        group.name
    ));
    fmt.line("pub fn builder() -> Builder {");
    fmt.indent(|fmt| {
        fmt.line("Builder::new(&TEMPLATE)");
    });
    fmt.line("}");
}

fn gen_display(group: &SettingGroup, fmt: &mut Formatter) {
    fmt.line("impl fmt::Display for Flags {");
    fmt.indent(|fmt| {
        fmt.line("fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {");
        fmt.indent(|fmt| {
            fmt.line(&format!("writeln!(f, \"[{}]\")?;", group.name));
            fmt.line("for d in &DESCRIPTORS {");
            fmt.indent(|fmt| {
                fmt.line("if !d.detail.is_preset() {");
                fmt.indent(|fmt| {
                    fmt.line("write!(f, \"{} = \", d.name)?;");
                    fmt.line(
                        "TEMPLATE.format_toml_value(d.detail, self.bytes[d.offset as usize], f)?;",
                    );
                    fmt.line("writeln!(f)?;");
                });
                fmt.line("}");
            });
            fmt.line("}");
            fmt.line("Ok(())");
        });
        fmt.line("}")
    });
    fmt.line("}");
}

fn gen_group(group: &SettingGroup, parent: ParentGroup, fmt: &mut Formatter) {
    // Generate struct.
    fmt.line("#[derive(Clone)]");
    fmt.doc_comment(&format!("Flags group `{}`.", group.name));
    fmt.line("pub struct Flags {");
    fmt.indent(|fmt| {
        fmt.line(&format!("bytes: [u8; {}],", group.byte_size()));
    });
    fmt.line("}");

    gen_constructor(group, parent, fmt);
    gen_enum_types(group, fmt);
    gen_getters(group, fmt);
    gen_descriptors(group, fmt);
    gen_template(group, fmt);
    gen_display(group, fmt);
}

pub fn generate_common(filename: &str, out_dir: &str) -> Result<SettingGroup, error::Error> {
    let settings = base::settings::generate();
    let mut fmt = Formatter::new();
    gen_group(&settings, ParentGroup::None, &mut fmt);
    fmt.update_file(filename, out_dir)?;
    Ok(settings)
}

pub fn generate(isa: &TargetIsa, prefix: &str, out_dir: &str) -> Result<(), error::Error> {
    let mut fmt = Formatter::new();
    gen_group(&isa.settings, ParentGroup::Shared, &mut fmt);
    fmt.update_file(&format!("{}-{}.rs", prefix, isa.name), out_dir)?;
    Ok(())
}

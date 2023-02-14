use cranelift_codegen::settings;
use std::collections::BTreeMap;
use wasmtime_environ::FlagValue;

mod builder;
pub use builder::*;

pub fn clif_flags_to_wasmtime(
    flags: impl IntoIterator<Item = settings::Value>,
) -> BTreeMap<String, FlagValue> {
    flags
        .into_iter()
        .map(|val| (val.name.to_string(), to_flag_value(&val)))
        .collect()
}

fn to_flag_value(v: &settings::Value) -> FlagValue {
    match v.kind() {
        settings::SettingKind::Enum => FlagValue::Enum(v.as_enum().unwrap().into()),
        settings::SettingKind::Num => FlagValue::Num(v.as_num().unwrap()),
        settings::SettingKind::Bool => FlagValue::Bool(v.as_bool().unwrap()),
        settings::SettingKind::Preset => unreachable!(),
    }
}

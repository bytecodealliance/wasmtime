use cranelift_codegen::binemit;
use cranelift_codegen::ir;
use cranelift_codegen::settings;
use std::collections::BTreeMap;
use wasmtime_environ::{FlagValue, FuncIndex};

mod builder;
pub mod obj;
pub use builder::*;

/// A record of a relocation to perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relocation {
    /// The relocation code.
    pub reloc: binemit::Reloc,
    /// Relocation target.
    pub reloc_target: RelocationTarget,
    /// The offset where to apply the relocation.
    pub offset: binemit::CodeOffset,
    /// The addend to add to the relocation value.
    pub addend: binemit::Addend,
}

/// Destination function. Can be either user function or some special one, like `memory.grow`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RelocationTarget {
    /// The user function index.
    UserFunc(FuncIndex),
    /// A compiler-generated libcall.
    LibCall(ir::LibCall),
}

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

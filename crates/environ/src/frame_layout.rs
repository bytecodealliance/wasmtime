use cranelift_codegen::isa::CallConv;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::DefinedFuncIndex;
use serde::{Deserialize, Serialize};

pub use cranelift_codegen::ir::FrameLayoutChange;

/// Frame layout information: call convention and
/// registers save/restore commands.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct FrameLayout {
    /// Call convention.
    pub call_conv: CallConv,
    /// Frame default/initial commands.
    pub initial_commands: Box<[FrameLayoutChange]>,
    /// Frame commands at specific offset.
    pub commands: Box<[(usize, FrameLayoutChange)]>,
}

/// Functions frame layouts.
pub type FrameLayouts = PrimaryMap<DefinedFuncIndex, FrameLayout>;

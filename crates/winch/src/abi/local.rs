use wasmtime_environ::WasmType;

/// Stack addressing mode
///
/// Slots for stack arguments are addressed from the frame pointer
/// Slots for function-defined locals and for registers are addressed
/// from the stack pointer
enum StackAMode {
    FPOffset,
    SPOffset,
}

/// A local slot
///
/// Represents the type, location and addressing mode of a local
/// in the stack's local and argument area
pub(crate) struct LocalSlot {
    /// The offset of the local slot
    offset: u64,
    /// The type contained by this local slot
    ty: WasmType,
    /// Stack addressing mode associated to this local slot
    amode: StackAMode,
}

impl LocalSlot {
    /// Creates a local slot for a function defined local or
    /// for a spilled argument register
    pub fn new(ty: WasmType, offset: u64) -> Self {
        Self {
            ty,
            offset,
            amode: StackAMode::SPOffset,
        }
    }

    /// Creates a local slot for a stack function argument
    pub fn stack_arg(ty: WasmType, offset: u64) -> Self {
        Self {
            ty,
            offset,
            amode: StackAMode::FPOffset,
        }
    }
}

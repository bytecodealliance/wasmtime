use wasmtime_environ::WasmType;

/// Base register used to address the local slot
///
/// Slots for stack arguments are addressed from the frame pointer
/// Slots for function-defined locals and for registers are addressed
/// from the stack pointer
#[derive(Eq, PartialEq)]
enum Base {
    FP,
    SP,
}

/// A local slot
///
/// Represents the type, location and addressing mode of a local
/// in the stack's local and argument area
pub(crate) struct LocalSlot {
    /// The offset of the local slot
    pub offset: u32,
    /// The type contained by this local slot
    ty: WasmType,
    /// Base register associated to this local slot
    base: Base,
}

impl LocalSlot {
    /// Creates a local slot for a function defined local or
    /// for a spilled argument register
    pub fn new(ty: WasmType, offset: u32) -> Self {
        Self {
            ty,
            offset,
            base: Base::SP,
        }
    }

    /// Creates a local slot for a stack function argument
    pub fn stack_arg(ty: WasmType, offset: u32) -> Self {
        Self {
            ty,
            offset,
            base: Base::FP,
        }
    }

    /// Check if the local is addressed from the stack pointer
    pub fn addressed_from_sp(&self) -> bool {
        self.base == Base::SP
    }
}

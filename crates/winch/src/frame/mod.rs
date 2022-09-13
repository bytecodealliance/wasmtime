// TODO
// * Track SP register
// * Hold a reference to the assembler

/// Frame handler abstraction
#[derive(Default)]
pub(crate) struct Frame {
    /// The local area size
    locals_size: u64,
}

impl Frame {
    /// Allocate a new Frame
    pub fn new(locals_size: u64) -> Self {
        Self { locals_size }
    }
}

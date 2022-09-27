// TODO
// I don't think we need this level of indirection;
// flagging it as a candidate for deletion and moving
// the locals_size to the compilation environment
/// Frame handler abstraction
#[derive(Default)]
pub(crate) struct Frame {
    /// The local area size
    pub locals_size: u32,
}

impl Frame {
    /// Allocate a new Frame
    pub fn new(locals_size: u32) -> Self {
        Self { locals_size }
    }
}

use crate::runtime::Uninhabited;

/// Represents an opaque reference to any data within WebAssembly.
///
/// Due to compilation configuration, this is an uninhabited type: enable the
/// `gc` cargo feature to properly use this type.
#[derive(Clone, Debug)]
pub struct ExternRef {
    pub(crate) _inner: Uninhabited,
}

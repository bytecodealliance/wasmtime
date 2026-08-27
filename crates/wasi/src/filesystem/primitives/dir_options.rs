#[cfg(not(target_os = "wasi"))]
use crate::filesystem::primitives::DirOptionsExt;

/// Options and flags which can be used to configure how a directory is
/// created.
///
/// This is to `create_dir` what to `OpenOptions` is to `open`.
#[derive(Debug, Clone)]
pub struct DirOptions {
    #[cfg(not(target_os = "wasi"))]
    #[allow(dead_code)]
    pub(crate) ext: DirOptionsExt,
}

impl DirOptions {
    /// Creates a blank new set of options ready for configuration.
    #[allow(clippy::new_without_default)]
    #[inline]
    pub const fn new() -> Self {
        Self {
            #[cfg(not(target_os = "wasi"))]
            ext: DirOptionsExt::new(),
        }
    }
}

#[cfg(unix)]
impl crate::filesystem::primitives::DirBuilderExt for DirOptions {
    #[inline]
    fn mode(&mut self, mode: u32) -> &mut Self {
        self.ext.mode(mode);
        self
    }
}

#[cfg(target_os = "vxworks")]
impl crate::fs::DirBuilderExt for DirOptions {
    #[inline]
    fn mode(&mut self, mode: u32) -> &mut Self {
        self.ext.mode(mode);
        self
    }
}

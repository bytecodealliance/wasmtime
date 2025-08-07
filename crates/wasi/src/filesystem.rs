use crate::p2::filesystem::Dir;
use wasmtime::component::{HasData, ResourceTable};

pub(crate) struct WasiFilesystem;

impl HasData for WasiFilesystem {
    type Data<'a> = WasiFilesystemCtxView<'a>;
}

#[derive(Clone, Default)]
pub struct WasiFilesystemCtx {
    pub allow_blocking_current_thread: bool,
    pub preopens: Vec<(Dir, String)>,
}

pub struct WasiFilesystemCtxView<'a> {
    pub ctx: &'a mut WasiFilesystemCtx,
    pub table: &'a mut ResourceTable,
}

pub trait WasiFilesystemView: Send {
    fn filesystem(&mut self) -> WasiFilesystemCtxView<'_>;
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct FilePerms: usize {
        const READ = 0b1;
        const WRITE = 0b10;
    }
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct OpenMode: usize {
        const READ = 0b1;
        const WRITE = 0b10;
    }
}

bitflags::bitflags! {
    /// Permission bits for operating on a directory.
    ///
    /// Directories can be limited to being readonly. This will restrict what
    /// can be done with them, for example preventing creation of new files.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct DirPerms: usize {
        /// This directory can be read, for example its entries can be iterated
        /// over and files can be opened.
        const READ = 0b1;

        /// This directory can be mutated, for example by creating new files
        /// within it.
        const MUTATE = 0b10;
    }
}

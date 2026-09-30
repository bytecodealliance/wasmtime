mod host;

use crate::TrappableError;
use crate::filesystem::{
    WasiFilesystem, WasiFilesystemNamed, WasiFilesystemNamedView, WasiFilesystemView,
};
use crate::p3::Interface;
use crate::p3::bindings::filesystem::{preopens, types};
use crate::{NamedId, WasiCtxNamedView};
use wasmtime::component::{Component, Linker};

pub type FilesystemResult<T> = Result<T, FilesystemError>;
pub type FilesystemError = TrappableError<types::ErrorCode>;

/// Add all WASI interfaces from this module into the `linker` provided.
///
/// This function will add all interfaces implemented by this module to the
/// [`Linker`], which corresponds to the `wasi:filesystem/imports` world supported by
/// this module.
///
/// This is low-level API for advanced use cases,
/// [`wasmtime_wasi::p3::add_to_linker`](crate::p3::add_to_linker) can be used instead
/// to add *all* wasip3 interfaces (including the ones from this module) to the `linker`.
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{Linker, ResourceTable};
/// use wasmtime_wasi::filesystem::{WasiFilesystemCtx, WasiFilesystemCtxView, WasiFilesystemView};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.wasm_component_model_async(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::p3::filesystem::add_to_linker(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     let mut store = Store::new(
///         &engine,
///         MyState::default(),
///     );
///
///     // ... use `linker` to instantiate within `store` ...
///
///     Ok(())
/// }
///
/// #[derive(Default)]
/// struct MyState {
///     filesystem: WasiFilesystemCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiFilesystemView for MyState {
///     fn filesystem(&mut self) -> WasiFilesystemCtxView<'_> {
///         WasiFilesystemCtxView {
///             ctx: &mut self.filesystem,
///             table: &mut self.table,
///         }
///     }
/// }
/// ```
pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiFilesystemView + 'static,
{
    types::add_to_linker::<_, WasiFilesystem>(linker, T::filesystem)?;
    preopens::add_to_linker::<_, WasiFilesystem>(linker, T::filesystem)?;
    Ok(())
}

/// Convenience function to add `wasi:filesystem` interfaces into `linker` for any
/// named imports of `wasi:filesystem` interfaces.
///
/// This function is similar to [`add_to_linker`] except that it's specifically
/// designed to work with named imports of `wasi:filesystem` interfaces that
/// components may have. This requires a [`Component`] parameter to be passed in
/// when populating the [`Linker`] provided to see what the [`Component`]
/// actually imports.
///
/// Like [`add_to_linker`] this is a bit low level and you may want to possibly
/// invoke [`wasmtime_wasi::p3::add_named_to_linker`] instead. Alternatively if
/// this isn't low level enough you can additionally invoke bindgen-generated
/// `add_to_linker` functions directly from within the
/// [`named_imports::wasi::filesystem`] module.
///
/// [`wasmtime_wasi::p3::add_named_to_linker`]: crate::p3::add_named_to_linker
/// [`named_imports::wasi::filesystem`]: crate::p3::bindings::named_imports::wasi::filesystem
///
/// The `lookup` function provided here is invoked for every named import found
/// for a particular interface. The [`Interface`] given is what's being bound,
/// and the `&str` argument is the name that the component imports it as. The
/// embedder can then decide how it would like to allocate a [`NamedId`] for
/// this import. If `Ok` is returned then the linker is populated with this
/// name, and imported functions will pass the [`NamedId`] later to the
/// implementation of [`WasiFilesystemNamedView`] on `T` when invoked. If `Err` is
/// returned then the error will cause this entire function to fail and this
/// function call will return the same error.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use wasmtime::component::{Component, Linker, ResourceTable};
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime_wasi::NamedId;
/// use wasmtime_wasi::filesystem::{WasiFilesystemCtx, WasiFilesystemCtxView, WasiFilesystemNamedView};
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let component = Component::new(&engine, "(component)")?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///
///     // ... add default functionality to `linker` as needed ...
///
///     // and then additionally fill in any specific named imports `component`
///     // might have for `wasi:filesystem` interfaces.
///     let mut name_map = HashMap::new();
///     wasmtime_wasi::p3::filesystem::add_named_to_linker(&mut linker, &component, |_i, name| {
///         let len = name_map.len();
///         Ok(NamedId(*name_map.entry(name.to_string()).or_insert(len)))
///     })?;
///
///     // Here a `WasiFilesystemCtx` is allocated per-named-import and will then be
///     // referred to internally by the [`NamedId`] allocated above. You could
///     // also use `name_map` to configure each context differently.
///     let mut my_state = MyState::default();
///     for _ in 0..name_map.len() {
///         my_state.contexts.push(WasiFilesystemCtx::default());
///     }
///     let mut store = Store::new(&engine, my_state);
///     let instance = linker.instantiate(&mut store, &component)?;
///
///     // ... work with `instance` ...
///
///     Ok(())
/// }
///
/// #[derive(Default)]
/// struct MyState {
///     table: ResourceTable,
///     contexts: Vec<WasiFilesystemCtx>,
/// }
///
/// impl WasiFilesystemNamedView for MyState {
///     fn filesystem(&mut self, id: NamedId) -> WasiFilesystemCtxView<'_> {
///         WasiFilesystemCtxView {
///             ctx: &mut self.contexts[id.0],
///             table: &mut self.table,
///         }
///     }
/// }
/// ```
pub fn add_named_to_linker<T>(
    linker: &mut Linker<T>,
    component: &Component,
    mut lookup: impl FnMut(Interface, &str) -> wasmtime::Result<NamedId>,
) -> wasmtime::Result<()>
where
    T: WasiFilesystemNamedView + 'static,
{
    use crate::p3::bindings::named_imports::wasi::filesystem::{preopens, types};
    types::add_to_linker::<_, WasiFilesystemNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::FilesystemTypes, name),
        |x| WasiCtxNamedView(x),
    )?;
    preopens::add_to_linker::<_, WasiFilesystemNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::FilesystemPreopens, name),
        |x| WasiCtxNamedView(x),
    )?;
    Ok(())
}

impl<'a> From<&'a std::io::Error> for types::ErrorCode {
    fn from(err: &'a std::io::Error) -> Self {
        crate::filesystem::ErrorCode::from(err).into()
    }
}

impl From<std::io::Error> for types::ErrorCode {
    fn from(err: std::io::Error) -> Self {
        Self::from(&err)
    }
}

impl From<std::io::Error> for FilesystemError {
    fn from(error: std::io::Error) -> Self {
        types::ErrorCode::from(error).into()
    }
}

impl From<crate::filesystem::ErrorCode> for types::ErrorCode {
    fn from(error: crate::filesystem::ErrorCode) -> Self {
        match error {
            crate::filesystem::ErrorCode::Access => Self::Access,
            crate::filesystem::ErrorCode::Already => Self::Already,
            crate::filesystem::ErrorCode::BadDescriptor => Self::BadDescriptor,
            crate::filesystem::ErrorCode::Busy => Self::Busy,
            crate::filesystem::ErrorCode::Exist => Self::Exist,
            crate::filesystem::ErrorCode::FileTooLarge => Self::FileTooLarge,
            crate::filesystem::ErrorCode::IllegalByteSequence => Self::IllegalByteSequence,
            crate::filesystem::ErrorCode::InProgress => Self::InProgress,
            crate::filesystem::ErrorCode::Interrupted => Self::Interrupted,
            crate::filesystem::ErrorCode::Invalid => Self::Invalid,
            crate::filesystem::ErrorCode::Io => Self::Io,
            crate::filesystem::ErrorCode::IsDirectory => Self::IsDirectory,
            crate::filesystem::ErrorCode::Loop => Self::Loop,
            crate::filesystem::ErrorCode::TooManyLinks => Self::TooManyLinks,
            crate::filesystem::ErrorCode::NameTooLong => Self::NameTooLong,
            crate::filesystem::ErrorCode::NoEntry => Self::NoEntry,
            crate::filesystem::ErrorCode::InsufficientMemory => Self::InsufficientMemory,
            crate::filesystem::ErrorCode::InsufficientSpace => Self::InsufficientSpace,
            crate::filesystem::ErrorCode::NotDirectory => Self::NotDirectory,
            crate::filesystem::ErrorCode::NotEmpty => Self::NotEmpty,
            crate::filesystem::ErrorCode::Unsupported => Self::Unsupported,
            crate::filesystem::ErrorCode::Overflow => Self::Overflow,
            crate::filesystem::ErrorCode::NotPermitted => Self::NotPermitted,
            crate::filesystem::ErrorCode::Pipe => Self::Pipe,
            crate::filesystem::ErrorCode::InvalidSeek => Self::InvalidSeek,
        }
    }
}

impl From<crate::filesystem::ErrorCode> for FilesystemError {
    fn from(error: crate::filesystem::ErrorCode) -> Self {
        types::ErrorCode::from(error).into()
    }
}

impl From<wasmtime::component::ResourceTableError> for FilesystemError {
    fn from(error: wasmtime::component::ResourceTableError) -> Self {
        Self::trap(error)
    }
}

impl From<types::Advice> for crate::filesystem::Advice {
    fn from(advice: types::Advice) -> Self {
        match advice {
            types::Advice::Normal => Self::Normal,
            types::Advice::Sequential => Self::Sequential,
            types::Advice::Random => Self::Random,
            types::Advice::WillNeed => Self::WillNeed,
            types::Advice::DontNeed => Self::DontNeed,
            types::Advice::NoReuse => Self::NoReuse,
        }
    }
}

impl From<types::OpenFlags> for crate::filesystem::OpenFlags {
    fn from(flags: types::OpenFlags) -> Self {
        let mut out = Self::empty();
        if flags.contains(types::OpenFlags::CREATE) {
            out |= Self::CREATE;
        }
        if flags.contains(types::OpenFlags::DIRECTORY) {
            out |= Self::DIRECTORY;
        }
        if flags.contains(types::OpenFlags::EXCLUSIVE) {
            out |= Self::EXCLUSIVE;
        }
        if flags.contains(types::OpenFlags::TRUNCATE) {
            out |= Self::TRUNCATE;
        }
        out
    }
}

impl From<types::PathFlags> for crate::filesystem::PathFlags {
    fn from(flags: types::PathFlags) -> Self {
        let mut out = Self::empty();
        if flags.contains(types::PathFlags::SYMLINK_FOLLOW) {
            out |= Self::SYMLINK_FOLLOW;
        }
        out
    }
}

impl From<crate::filesystem::DescriptorFlags> for types::DescriptorFlags {
    fn from(flags: crate::filesystem::DescriptorFlags) -> Self {
        let mut out = Self::empty();
        if flags.contains(crate::filesystem::DescriptorFlags::READ) {
            out |= Self::READ;
        }
        if flags.contains(crate::filesystem::DescriptorFlags::WRITE) {
            out |= Self::WRITE;
        }
        if flags.contains(crate::filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC) {
            out |= Self::FILE_INTEGRITY_SYNC;
        }
        if flags.contains(crate::filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC) {
            out |= Self::DATA_INTEGRITY_SYNC;
        }
        if flags.contains(crate::filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC) {
            out |= Self::REQUESTED_WRITE_SYNC;
        }
        if flags.contains(crate::filesystem::DescriptorFlags::MUTATE_DIRECTORY) {
            out |= Self::MUTATE_DIRECTORY;
        }
        out
    }
}

impl From<types::DescriptorFlags> for crate::filesystem::DescriptorFlags {
    fn from(flags: types::DescriptorFlags) -> Self {
        let mut out = Self::empty();
        if flags.contains(types::DescriptorFlags::READ) {
            out |= Self::READ;
        }
        if flags.contains(types::DescriptorFlags::WRITE) {
            out |= Self::WRITE;
        }
        if flags.contains(types::DescriptorFlags::FILE_INTEGRITY_SYNC) {
            out |= Self::FILE_INTEGRITY_SYNC;
        }
        if flags.contains(types::DescriptorFlags::DATA_INTEGRITY_SYNC) {
            out |= Self::DATA_INTEGRITY_SYNC;
        }
        if flags.contains(types::DescriptorFlags::REQUESTED_WRITE_SYNC) {
            out |= Self::REQUESTED_WRITE_SYNC;
        }
        if flags.contains(types::DescriptorFlags::MUTATE_DIRECTORY) {
            out |= Self::MUTATE_DIRECTORY;
        }
        out
    }
}

impl From<crate::filesystem::MetadataHashValue> for types::MetadataHashValue {
    fn from(
        crate::filesystem::MetadataHashValue { lower, upper }: crate::filesystem::MetadataHashValue,
    ) -> Self {
        Self { lower, upper }
    }
}

impl From<crate::filesystem::DescriptorStat> for types::DescriptorStat {
    fn from(
        crate::filesystem::DescriptorStat {
            type_,
            link_count,
            size,
            data_access_timestamp,
            data_modification_timestamp,
            status_change_timestamp,
        }: crate::filesystem::DescriptorStat,
    ) -> Self {
        Self {
            type_: type_.into(),
            link_count,
            size,
            data_access_timestamp: data_access_timestamp.map(Into::into),
            data_modification_timestamp: data_modification_timestamp.map(Into::into),
            status_change_timestamp: status_change_timestamp.map(Into::into),
        }
    }
}

impl From<crate::filesystem::DescriptorType> for types::DescriptorType {
    fn from(ty: crate::filesystem::DescriptorType) -> Self {
        match ty {
            crate::filesystem::DescriptorType::Unknown => Self::Other(None),
            crate::filesystem::DescriptorType::BlockDevice => Self::BlockDevice,
            crate::filesystem::DescriptorType::CharacterDevice => Self::CharacterDevice,
            crate::filesystem::DescriptorType::Directory => Self::Directory,
            crate::filesystem::DescriptorType::SymbolicLink => Self::SymbolicLink,
            crate::filesystem::DescriptorType::RegularFile => Self::RegularFile,
        }
    }
}

impl From<crate::filesystem::primitives::FileType> for types::DescriptorType {
    fn from(ft: crate::filesystem::primitives::FileType) -> Self {
        crate::filesystem::DescriptorType::from(ft).into()
    }
}

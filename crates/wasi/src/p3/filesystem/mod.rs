mod host;

use crate::TrappableError;
use crate::filesystem::{WasiFilesystem, WasiFilesystemView};
use crate::p3::bindings::filesystem::{preopens, types};
use wasmtime::component::Linker;

pub type FilesystemResult<T> = Result<T, FilesystemError>;
pub type FilesystemError = TrappableError<types::ErrorCode>;

/// Add all WASI interfaces from this module into the `linker` provided.
///
/// This function will add all interfaces implemented by this module to the
/// [`Linker`], which corresponds to the `wasi:sockets/imports` world supported by
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
///     config.async_support(true);
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

impl From<types::Advice> for system_interface::fs::Advice {
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
            crate::filesystem::DescriptorType::Unknown => Self::Unknown,
            crate::filesystem::DescriptorType::BlockDevice => Self::BlockDevice,
            crate::filesystem::DescriptorType::CharacterDevice => Self::CharacterDevice,
            crate::filesystem::DescriptorType::Directory => Self::Directory,
            crate::filesystem::DescriptorType::SymbolicLink => Self::SymbolicLink,
            crate::filesystem::DescriptorType::RegularFile => Self::RegularFile,
        }
    }
}

impl From<cap_std::fs::FileType> for types::DescriptorType {
    fn from(ft: cap_std::fs::FileType) -> Self {
        use cap_fs_ext::FileTypeExt as _;
        if ft.is_dir() {
            Self::Directory
        } else if ft.is_symlink() {
            Self::SymbolicLink
        } else if ft.is_block_device() {
            Self::BlockDevice
        } else if ft.is_char_device() {
            Self::CharacterDevice
        } else if ft.is_file() {
            Self::RegularFile
        } else {
            Self::Unknown
        }
    }
}

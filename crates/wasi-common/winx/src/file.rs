#![allow(non_camel_case_types)]

use crate::ntdll::{
    NtQueryInformationFile, RtlNtStatusToDosError, FILE_ACCESS_INFORMATION, FILE_INFORMATION_CLASS,
    FILE_MODE_INFORMATION, IO_STATUS_BLOCK,
};
use bitflags::bitflags;
use cvt::cvt;
use std::ffi::{c_void, OsString};
use std::fs::File;
use std::io::{Error, Result};
use std::os::windows::prelude::{AsRawHandle, OsStringExt, RawHandle};
use winapi::shared::{
    minwindef::{self, DWORD},
    ntstatus, winerror,
};
use winapi::um::{fileapi, fileapi::GetFileType, minwinbase, winbase, winnt};

/// Maximum total path length for Unicode in Windows.
/// [Maximum path length limitation]: https://docs.microsoft.com/en-us/windows/desktop/FileIO/naming-a-file#maximum-path-length-limitation
pub const WIDE_MAX_PATH: DWORD = 0x7fff;

#[derive(Debug, Copy, Clone)]
pub struct FileType(minwindef::DWORD);

// possible types are:
// * FILE_TYPE_CHAR
// * FILE_TYPE_DISK
// * FILE_TYPE_PIPE
// * FILE_TYPE_REMOTE
// * FILE_TYPE_UNKNOWN
//
// FILE_TYPE_REMOTE is unused
// https://technet.microsoft.com/en-us/evalcenter/aa364960(v=vs.100)
impl FileType {
    /// Returns true if character device such as LPT device or console
    pub fn is_char(&self) -> bool {
        self.0 == winbase::FILE_TYPE_CHAR
    }

    /// Returns true if disk device such as file or dir
    pub fn is_disk(&self) -> bool {
        self.0 == winbase::FILE_TYPE_DISK
    }

    /// Returns true if pipe device such as socket, named pipe or anonymous pipe
    pub fn is_pipe(&self) -> bool {
        self.0 == winbase::FILE_TYPE_PIPE
    }

    /// Returns true if unknown device
    pub fn is_unknown(&self) -> bool {
        self.0 == winbase::FILE_TYPE_UNKNOWN
    }
}

pub unsafe fn get_file_type(handle: RawHandle) -> Result<FileType> {
    let file_type = FileType(GetFileType(handle));
    let err = Error::last_os_error();
    if file_type.is_unknown() && err.raw_os_error().unwrap() as u32 != winerror::ERROR_SUCCESS {
        Err(err)
    } else {
        Ok(file_type)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
pub enum CreationDisposition {
    NO_DISPOSITION = 0,
    /// Creates a new file, only if it does not already exist.
    /// If the specified file exists, the function fails and the last-error code is
    /// set to ERROR_FILE_EXISTS (80).
    ///
    /// If the specified file does not exist and is a valid path to a writable location,
    /// a new file is created.
    CREATE_NEW = fileapi::CREATE_NEW,
    /// Creates a new file, always.
    /// If the specified file exists and is writable, the function overwrites the file,
    /// the function succeeds, and last-error code is set to ERROR_ALREADY_EXISTS (183).
    ///
    /// If the specified file does not exist and is a valid path, a new file is created,
    /// the function succeeds, and the last-error code is set to zero.
    CREATE_ALWAYS = fileapi::CREATE_ALWAYS,
    /// Opens a file or device, only if it exists.
    /// If the specified file or device does not exist, the function fails and the
    /// last-error code is set to ERROR_FILE_NOT_FOUND (2).
    OPEN_EXISTING = fileapi::OPEN_EXISTING,
    /// Opens a file, always.
    /// If the specified file exists, the function succeeds and the last-error code is
    /// set to ERROR_ALREADY_EXISTS (183).
    ///
    /// If the specified file does not exist and is a valid path to a writable location,
    /// the function creates a file and the last-error code is set to zero.
    OPEN_ALWAYS = fileapi::OPEN_ALWAYS,
    /// Opens a file and truncates it so that its size is zero bytes, only if it exists.
    /// If the specified file does not exist, the function fails and the last-error code
    /// is set to ERROR_FILE_NOT_FOUND (2).
    ///
    /// The calling process must open the file with the GENERIC_WRITE bit set as part
    /// of the dwDesiredAccess parameter.
    TRUNCATE_EXISTING = fileapi::TRUNCATE_EXISTING,
}

impl CreationDisposition {
    pub fn from_u32(disp: u32) -> Self {
        use CreationDisposition::*;
        match disp {
            fileapi::CREATE_NEW => CREATE_NEW,
            fileapi::CREATE_ALWAYS => CREATE_ALWAYS,
            fileapi::OPEN_EXISTING => OPEN_EXISTING,
            fileapi::OPEN_ALWAYS => OPEN_ALWAYS,
            fileapi::TRUNCATE_EXISTING => TRUNCATE_EXISTING,
            _ => NO_DISPOSITION,
        }
    }
}

bitflags! {
    pub struct Attributes: minwindef::DWORD {
        /// A file or directory that is an archive file or directory.
        /// Applications typically use this attribute to mark files for backup or removal.
        const FILE_ATTRIBUTE_ARCHIVE = winnt::FILE_ATTRIBUTE_ARCHIVE;
        /// A file or directory that is compressed. For a file, all of the data in the file is compressed.
        /// For a directory, compression is the default for newly created files and subdirectories.
        const FILE_ATTRIBUTE_COMPRESSED = winnt::FILE_ATTRIBUTE_COMPRESSED;
        /// This value is reserved for system use.
        const FILE_ATTRIBUTE_DEVICE = winnt::FILE_ATTRIBUTE_DEVICE;
        /// The handle that identifies a directory.
        const FILE_ATTRIBUTE_DIRECTORY = winnt::FILE_ATTRIBUTE_DIRECTORY;
        /// A file or directory that is encrypted. For a file, all data streams in the file are encrypted.
        /// For a directory, encryption is the default for newly created files and subdirectories.
        const FILE_ATTRIBUTE_ENCRYPTED = winnt::FILE_ATTRIBUTE_ENCRYPTED;
        /// The file or directory is hidden. It is not included in an ordinary directory listing.
        const FILE_ATTRIBUTE_HIDDEN = winnt::FILE_ATTRIBUTE_HIDDEN;
        /// The directory or user data stream is configured with integrity (only supported on ReFS volumes).
        /// It is not included in an ordinary directory listing. The integrity setting persists with the file if it's renamed.
        /// If a file is copied the destination file will have integrity set if either the source file or destination directory have integrity set.
        const FILE_ATTRIBUTE_INTEGRITY_STREAM = winnt::FILE_ATTRIBUTE_INTEGRITY_STREAM;
        /// A file that does not have other attributes set. This attribute is valid only when used alone.
        const FILE_ATTRIBUTE_NORMAL = winnt::FILE_ATTRIBUTE_NORMAL;
        /// The file or directory is not to be indexed by the content indexing service.
        const FILE_ATTRIBUTE_NOT_CONTENT_INDEXED = winnt::FILE_ATTRIBUTE_NOT_CONTENT_INDEXED;
        /// The user data stream not to be read by the background data integrity scanner (AKA scrubber).
        /// When set on a directory it only provides inheritance. This flag is only supported on Storage Spaces and ReFS volumes.
        /// It is not included in an ordinary directory listing.
        const FILE_ATTRIBUTE_NO_SCRUB_DATA = winnt::FILE_ATTRIBUTE_NO_SCRUB_DATA;
        /// The data of a file is not available immediately.
        /// This attribute indicates that the file data is physically moved to offline storage.
        /// This attribute is used by Remote Storage, which is the hierarchical storage management software.
        /// Applications should not arbitrarily change this attribute.
        const FILE_ATTRIBUTE_OFFLINE = winnt::FILE_ATTRIBUTE_OFFLINE;
        /// A file that is read-only. Applications can read the file, but cannot write to it or delete it.
        /// This attribute is not honored on directories.
        const FILE_ATTRIBUTE_READONLY = winnt::FILE_ATTRIBUTE_READONLY;
        /// When this attribute is set, it means that the file or directory is not fully present locally.
        /// For a file that means that not all of its data is on local storage (e.g. it may be sparse with some data still in remote storage).
        /// For a directory it means that some of the directory contents are being virtualized from another location.
        /// Reading the file / enumerating the directory will be more expensive than normal, e.g. it will cause at least some of the
        /// file/directory content to be fetched from a remote store. Only kernel-mode callers can set this bit.
        const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS = winnt::FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS;
        /// This attribute only appears in directory enumeration classes (FILE_DIRECTORY_INFORMATION, FILE_BOTH_DIR_INFORMATION, etc.).
        /// When this attribute is set, it means that the file or directory has no physical representation on the local system; the item is virtual.
        /// Opening the item will be more expensive than normal, e.g. it will cause at least some of it to be fetched from a remote store.
        const FILE_ATTRIBUTE_RECALL_ON_OPEN = winnt::FILE_ATTRIBUTE_RECALL_ON_OPEN;
        /// A file or directory that has an associated reparse point, or a file that is a symbolic link.
        const FILE_ATTRIBUTE_REPARSE_POINT = winnt::FILE_ATTRIBUTE_REPARSE_POINT;
        /// A file that is a sparse file.
        const FILE_ATTRIBUTE_SPARSE_FILE = winnt::FILE_ATTRIBUTE_SPARSE_FILE;
        /// A file or directory that the operating system uses a part of, or uses exclusively.
        const FILE_ATTRIBUTE_SYSTEM = winnt::FILE_ATTRIBUTE_SYSTEM;
        /// A file that is being used for temporary storage.
        /// File systems avoid writing data back to mass storage if sufficient cache memory is available, because typically,
        /// an application deletes a temporary file after the handle is closed. In that scenario, the system can entirely
        /// avoid writing the data. Otherwise, the data is written after the handle is closed.
        const FILE_ATTRIBUTE_TEMPORARY = winnt::FILE_ATTRIBUTE_TEMPORARY;
        /// This value is reserved for system use.
        const FILE_ATTRIBUTE_VIRTUAL = winnt::FILE_ATTRIBUTE_VIRTUAL;
    }
}

bitflags! {
    pub struct Flags: minwindef::DWORD {
        /// The file is being opened or created for a backup or restore operation.
        /// The system ensures that the calling process overrides file security checks when the process has SE_BACKUP_NAME and SE_RESTORE_NAME privileges.
        /// You must set this flag to obtain a handle to a directory. A directory handle can be passed to some functions instead of a file handle.
        const FILE_FLAG_BACKUP_SEMANTICS = winbase::FILE_FLAG_BACKUP_SEMANTICS;
        /// The file is to be deleted immediately after all of its handles are closed, which includes the specified handle and any other open or duplicated handles.
        /// If there are existing open handles to a file, the call fails unless they were all opened with the FILE_SHARE_DELETE share mode.
        /// Subsequent open requests for the file fail, unless the FILE_SHARE_DELETE share mode is specified.
        const FILE_FLAG_DELETE_ON_CLOSE = winbase::FILE_FLAG_DELETE_ON_CLOSE;
        /// The file or device is being opened with no system caching for data reads and writes.
        /// This flag does not affect hard disk caching or memory mapped files.
        /// There are strict requirements for successfully working with files opened with
        /// CreateFile using the FILE_FLAG_NO_BUFFERING flag.
        const FILE_FLAG_NO_BUFFERING = winbase::FILE_FLAG_NO_BUFFERING;
        /// The file data is requested, but it should continue to be located in remote storage.
        /// It should not be transported back to local storage. This flag is for use by remote storage systems.
        const FILE_FLAG_OPEN_NO_RECALL = winbase::FILE_FLAG_OPEN_NO_RECALL;
        /// Normal reparse point processing will not occur; CreateFile will attempt to open the reparse point.
        /// When a file is opened, a file handle is returned, whether or not the filter that controls the reparse point is operational.
        /// This flag cannot be used with the CREATE_ALWAYS flag.
        /// If the file is not a reparse point, then this flag is ignored.
        const FILE_FLAG_OPEN_REPARSE_POINT = winbase::FILE_FLAG_OPEN_REPARSE_POINT;
        /// The file or device is being opened or created for asynchronous I/O.
        /// When subsequent I/O operations are completed on this handle, the event specified in the OVERLAPPED structure will be set to the signaled state.
        /// If this flag is specified, the file can be used for simultaneous read and write operations.
        /// If this flag is not specified, then I/O operations are serialized, even if the calls to the read and write functions specify an OVERLAPPED structure.
        const FILE_FLAG_OVERLAPPED = winbase::FILE_FLAG_OVERLAPPED;
        /// Access will occur according to POSIX rules. This includes allowing multiple files with names,
        /// differing only in case, for file systems that support that naming. Use care when using this option,
        /// because files created with this flag may not be accessible by applications that are written for MS-DOS or 16-bit Windows.
        const FILE_FLAG_POSIX_SEMANTICS = winbase::FILE_FLAG_POSIX_SEMANTICS;
        /// Access is intended to be random. The system can use this as a hint to optimize file caching.
        /// This flag has no effect if the file system does not support cached I/O and FILE_FLAG_NO_BUFFERING.
        const FILE_FLAG_RANDOM_ACCESS = winbase::FILE_FLAG_RANDOM_ACCESS;
        /// The file or device is being opened with session awareness.
        /// If this flag is not specified, then per-session devices (such as a device using RemoteFX USB Redirection)
        /// cannot be opened by processes running in session 0. This flag has no effect for callers not in session 0.
        /// This flag is supported only on server editions of Windows.
        const FILE_FLAG_SESSION_AWARE = winbase::FILE_FLAG_SESSION_AWARE;
        /// Access is intended to be sequential from beginning to end. The system can use this as a hint to optimize file caching.
        /// This flag should not be used if read-behind (that is, reverse scans) will be used.
        /// This flag has no effect if the file system does not support cached I/O and FILE_FLAG_NO_BUFFERING.
        const FILE_FLAG_SEQUENTIAL_SCAN = winbase::FILE_FLAG_SEQUENTIAL_SCAN;
        /// Write operations will not go through any intermediate cache, they will go directly to disk.
        const FILE_FLAG_WRITE_THROUGH = winbase::FILE_FLAG_WRITE_THROUGH;
    }
}

bitflags! {
    /// [Access mask]: https://docs.microsoft.com/en-us/windows/desktop/SecAuthZ/access-mask
    pub struct AccessMode: minwindef::DWORD {
        /// For a file object, the right to read the corresponding file data.
        /// For a directory object, the right to read the corresponding directory data.
        const FILE_READ_DATA = winnt::FILE_READ_DATA;
        const FILE_LIST_DIRECTORY = winnt::FILE_LIST_DIRECTORY;
        /// For a file object, the right to write data to the file.
        /// For a directory object, the right to create a file in the directory.
        const FILE_WRITE_DATA = winnt::FILE_WRITE_DATA;
        const FILE_ADD_FILE = winnt::FILE_ADD_FILE;
        /// For a file object, the right to append data to the file.
        /// (For local files, write operations will not overwrite existing data
        /// if this flag is specified without FILE_WRITE_DATA.)
        /// For a directory object, the right to create a subdirectory.
        /// For a named pipe, the right to create a pipe.
        const FILE_APPEND_DATA = winnt::FILE_APPEND_DATA;
        const FILE_ADD_SUBDIRECTORY = winnt::FILE_ADD_SUBDIRECTORY;
        const FILE_CREATE_PIPE_INSTANCE = winnt::FILE_CREATE_PIPE_INSTANCE;
        /// The right to read extended file attributes.
        const FILE_READ_EA = winnt::FILE_READ_EA;
        /// The right to write extended file attributes.
        const FILE_WRITE_EA = winnt::FILE_WRITE_EA;
        /// For a file, the right to execute FILE_EXECUTE.
        /// For a directory, the right to traverse the directory.
        /// By default, users are assigned the BYPASS_TRAVERSE_CHECKING privilege,
        /// which ignores the FILE_TRAVERSE access right.
        const FILE_EXECUTE = winnt::FILE_EXECUTE;
        const FILE_TRAVERSE = winnt::FILE_TRAVERSE;
        /// For a directory, the right to delete a directory and all
        /// the files it contains, including read-only files.
        const FILE_DELETE_CHILD = winnt::FILE_DELETE_CHILD;
        /// The right to read file attributes.
        const FILE_READ_ATTRIBUTES = winnt::FILE_READ_ATTRIBUTES;
        /// The right to write file attributes.
        const FILE_WRITE_ATTRIBUTES = winnt::FILE_WRITE_ATTRIBUTES;
        /// The right to delete the object.
        const DELETE = winnt::DELETE;
        /// The right to read the information in the object's security descriptor,
        /// not including the information in the system access control list (SACL).
        const READ_CONTROL = winnt::READ_CONTROL;
        /// The right to use the object for synchronization. This enables a thread
        /// to wait until the object is in the signaled state. Some object types
        /// do not support this access right.
        const SYNCHRONIZE = winnt::SYNCHRONIZE;
        /// The right to modify the discretionary access control list (DACL) in
        /// the object's security descriptor.
        const WRITE_DAC = winnt::WRITE_DAC;
        /// The right to change the owner in the object's security descriptor.
        const WRITE_OWNER = winnt::WRITE_OWNER;
        /// It is used to indicate access to a system access control list (SACL).
        const ACCESS_SYSTEM_SECURITY = winnt::ACCESS_SYSTEM_SECURITY;
        /// Maximum allowed.
        const MAXIMUM_ALLOWED = winnt::MAXIMUM_ALLOWED;
        /// Reserved
        const RESERVED1 = 0x4000000;
        /// Reserved
        const RESERVED2 = 0x8000000;
        /// Provides all possible access rights.
        /// This is convenience flag which is translated by the OS into actual [`FILE_GENERIC_ALL`] union.
        const GENERIC_ALL = winnt::GENERIC_ALL;
        /// Provides execute access.
        const GENERIC_EXECUTE = winnt::GENERIC_EXECUTE;
        /// Provides write access.
        /// This is convenience flag which is translated by the OS into actual [`FILE_GENERIC_WRITE`] union.
        const GENERIC_WRITE = winnt::GENERIC_WRITE;
        /// Provides read access.
        /// This is convenience flag which is translated by the OS into actual [`FILE_GENERIC_READ`] union.
        const GENERIC_READ = winnt::GENERIC_READ;
        /// Provides read access.
        const FILE_GENERIC_READ = winnt::FILE_GENERIC_READ;
        /// Provides write access.
        const FILE_GENERIC_WRITE = winnt::FILE_GENERIC_WRITE;
        /// Provides execute access.
        const FILE_GENERIC_EXECUTE = winnt::FILE_GENERIC_EXECUTE;
        /// Provides all accesses.
        const FILE_ALL_ACCESS = winnt::FILE_ALL_ACCESS;
    }
}

bitflags! {
    // https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/52df7798-8330-474b-ac31-9afe8075640c
    pub struct FileModeInformation: minwindef::DWORD {
        /// When set, any system services, file system drivers (FSDs), and drivers that write data to
        /// the file are required to actually transfer the data into the file before any requested write
        /// operation is considered complete.
        const FILE_WRITE_THROUGH = 0x2;
        /// This is a hint that informs the cache that it SHOULD optimize for sequential access.
        /// Non-sequential access of the file can result in performance degradation.
        const FILE_SEQUENTIAL_ONLY = 0x4;
        /// When set, the file cannot be cached or buffered in a driver's internal buffers.
        const FILE_NO_INTERMEDIATE_BUFFERING = 0x8;
        /// When set, all operations on the file are performed synchronously.
        /// Any wait on behalf of the caller is subject to premature termination from alerts.
        /// This flag also causes the I/O system to maintain the file position context.
        const FILE_SYNCHRONOUS_IO_ALERT = 0x10;
        /// When set, all operations on the file are performed synchronously.
        /// Wait requests in the system to synchronize I/O queuing and completion are not subject to alerts.
        /// This flag also causes the I/O system to maintain the file position context.
        const FILE_SYNCHRONOUS_IO_NONALERT = 0x20;
        /// This flag is not implemented and is always returned as not set.
        const FILE_DELETE_ON_CLOSE = 0x1000;
    }
}

pub fn get_file_path(file: &File) -> Result<OsString> {
    use winapi::um::fileapi::GetFinalPathNameByHandleW;

    let mut raw_path: Vec<u16> = vec![0; WIDE_MAX_PATH as usize];

    let handle = file.as_raw_handle();
    let read_len =
        cvt(unsafe { GetFinalPathNameByHandleW(handle, raw_path.as_mut_ptr(), WIDE_MAX_PATH, 0) })?;

    // obtain a slice containing the written bytes, and check for it being too long
    // (practically probably impossible)
    let written_bytes = raw_path
        .get(..read_len as usize)
        .ok_or(Error::from_raw_os_error(
            winerror::ERROR_BUFFER_OVERFLOW as i32,
        ))?;

    Ok(OsString::from_wide(written_bytes))
}

pub fn get_fileinfo(file: &File) -> Result<fileapi::BY_HANDLE_FILE_INFORMATION> {
    use fileapi::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};
    use std::mem;

    let handle = file.as_raw_handle();
    let info = unsafe {
        let mut info: BY_HANDLE_FILE_INFORMATION = mem::zeroed();
        cvt(GetFileInformationByHandle(handle, &mut info))?;
        info
    };

    Ok(info)
}

pub fn change_time(file: &File) -> Result<i64> {
    use fileapi::FILE_BASIC_INFO;
    use minwinbase::FileBasicInfo;
    use std::mem;
    use winbase::GetFileInformationByHandleEx;

    let handle = file.as_raw_handle();
    let tm = unsafe {
        let mut info: FILE_BASIC_INFO = mem::zeroed();
        let infosize = mem::size_of_val(&info);
        cvt(GetFileInformationByHandleEx(
            handle,
            FileBasicInfo,
            &mut info as *mut FILE_BASIC_INFO as *mut c_void,
            infosize as u32,
        ))?;
        *info.ChangeTime.QuadPart()
    };

    Ok(tm)
}

pub fn query_access_information(handle: RawHandle) -> Result<AccessMode> {
    let mut io_status_block = IO_STATUS_BLOCK::default();
    let mut info = FILE_ACCESS_INFORMATION::default();

    unsafe {
        let status = NtQueryInformationFile(
            handle,
            &mut io_status_block,
            &mut info as *mut _ as *mut c_void,
            std::mem::size_of::<FILE_ACCESS_INFORMATION>() as u32,
            FILE_INFORMATION_CLASS::FileAccessInformation,
        );

        if status != ntstatus::STATUS_SUCCESS {
            return Err(Error::from_raw_os_error(
                RtlNtStatusToDosError(status) as i32
            ));
        }
    }

    Ok(AccessMode::from_bits_truncate(info.AccessFlags))
}

pub fn query_mode_information(handle: RawHandle) -> Result<FileModeInformation> {
    let mut io_status_block = IO_STATUS_BLOCK::default();
    let mut info = FILE_MODE_INFORMATION::default();

    unsafe {
        let status = NtQueryInformationFile(
            handle,
            &mut io_status_block,
            &mut info as *mut _ as *mut c_void,
            std::mem::size_of::<FILE_MODE_INFORMATION>() as u32,
            FILE_INFORMATION_CLASS::FileModeInformation,
        );

        if status != ntstatus::STATUS_SUCCESS {
            return Err(Error::from_raw_os_error(
                RtlNtStatusToDosError(status) as i32
            ));
        }
    }

    Ok(FileModeInformation::from_bits_truncate(info.Mode))
}

pub fn reopen_file(handle: RawHandle, access_mode: AccessMode, flags: Flags) -> Result<RawHandle> {
    // Files on Windows are opened with DELETE, READ, and WRITE share mode by default (see OpenOptions in stdlib)
    // This keeps the same share mode when reopening the file handle
    let new_handle = unsafe {
        winbase::ReOpenFile(
            handle,
            access_mode.bits(),
            winnt::FILE_SHARE_DELETE | winnt::FILE_SHARE_READ | winnt::FILE_SHARE_WRITE,
            flags.bits(),
        )
    };

    if new_handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
        return Err(Error::last_os_error());
    }

    Ok(new_handle)
}

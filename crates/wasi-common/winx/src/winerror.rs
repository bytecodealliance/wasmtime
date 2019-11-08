#![allow(non_camel_case_types)]
use winapi::shared::winerror;
use winapi::um::errhandlingapi::GetLastError;

macro_rules! win_error_expand {
    {
        $(
            #[doc=$doc:literal]
            $error:ident,
        )*
    } => {
        /// Wraps WINAPI error code as enum.
        #[derive(Debug, Clone, Copy, Eq, PartialEq)]
        #[repr(u32)]
        pub enum WinError {
            /// Unknown error occurred.
            UnknownError = std::u32::MAX,
            $(
                #[doc=$doc]
                $error = winerror::$error,
            )*
        }

        fn desc(err: WinError) -> &'static str {
            use WinError::*;
            match err {
                UnknownError => r" Unknown error occurred.",
                $($error => $doc,)*
            }
        }

        fn from_u32(err: u32) -> WinError {
            use WinError::*;
            match err {
                $(winerror::$error => $error,)*
                _ => UnknownError,
            }
        }
    }
}

win_error_expand! {
    /// The operation completed successfully.
    ERROR_SUCCESS,
    /// The system cannot find the file specified.
    ERROR_FILE_NOT_FOUND,
    /// The system cannot find the path specified.
    ERROR_PATH_NOT_FOUND,
    /// The system cannot open the file.
    ERROR_TOO_MANY_OPEN_FILES,
    /// Access is denied.
    ERROR_ACCESS_DENIED,
    /// The handle is invalid.
    ERROR_INVALID_HANDLE,
    /// Not enough storage is available to process this command.
    ERROR_NOT_ENOUGH_MEMORY,
    /// The environment is incorrect.
    ERROR_BAD_ENVIRONMENT,
    /// Not enough storage is available to complete this operation.
    ERROR_OUTOFMEMORY,
    /// The device is not ready.
    ERROR_NOT_READY,
    /// The request is not supported.
    ERROR_NOT_SUPPORTED,
    /// The file exists.
    ERROR_FILE_EXISTS,
    /// The pipe has been ended.
    ERROR_BROKEN_PIPE,
    /// The file name is too long.
    ERROR_BUFFER_OVERFLOW,
    /// The directory is not empty.
    ERROR_DIR_NOT_EMPTY,
    /// The volume label you entered exceeds the label character limit of the destination file system.
    ERROR_LABEL_TOO_LONG,
    /// The requested resource is in use.
    ERROR_BUSY,
    /// The file name, directory name, or volume label syntax is incorrect.
    ERROR_INVALID_NAME,
    /// The process cannot access the file because it is being used by another process.
    ERROR_SHARING_VIOLATION,
    /// A required privilege is not held by the client.
    ERROR_PRIVILEGE_NOT_HELD,
    /// The file or directory is not a reparse point.
    ERROR_NOT_A_REPARSE_POINT,
    /// An attempt was made to move the file pointer before the beginning of the file.
    ERROR_NEGATIVE_SEEK,
    /// The directory name is invalid.
    ERROR_DIRECTORY,
    /// Cannot create a file when that file already exists.
    ERROR_ALREADY_EXISTS,
}

impl WinError {
    /// Returns the last error as WinError.
    pub fn last() -> Self {
        Self::from_u32(unsafe { GetLastError() })
    }

    /// Constructs WinError from error code.
    pub fn from_u32(err: u32) -> Self {
        from_u32(err)
    }

    /// Returns error's description string. This description matches
    /// the docs for the error.
    pub fn desc(self) -> &'static str {
        desc(self)
    }
}

impl std::error::Error for WinError {
    fn description(&self) -> &str {
        self.desc()
    }
}

impl std::fmt::Display for WinError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}: {}", self, self.desc())
    }
}

impl From<WinError> for std::io::Error {
    fn from(err: WinError) -> Self {
        Self::from_raw_os_error(err as i32)
    }
}

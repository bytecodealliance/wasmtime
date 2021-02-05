pub struct TestConfig {
    errno_mode: ErrnoMode,
    no_dangling_filesystem: bool,
    no_fd_allocate: bool,
    no_rename_dir_to_empty_dir: bool,
    no_fdflags_sync_support: bool,
}

enum ErrnoMode {
    Unix,
    MacOS,
    Windows,
    Permissive,
}

impl TestConfig {
    pub fn from_env() -> Self {
        let errno_mode = if std::env::var("ERRNO_MODE_UNIX").is_ok() {
            ErrnoMode::Unix
        } else if std::env::var("ERRNO_MODE_MACOS").is_ok() {
            ErrnoMode::MacOS
        } else if std::env::var("ERRNO_MODE_WINDOWS").is_ok() {
            ErrnoMode::Windows
        } else {
            ErrnoMode::Permissive
        };
        let no_dangling_filesystem = std::env::var("NO_DANGLING_FILESYSTEM").is_ok();
        let no_fd_allocate = std::env::var("NO_FD_ALLOCATE").is_ok();
        let no_rename_dir_to_empty_dir = std::env::var("NO_RENAME_DIR_TO_EMPTY_DIR").is_ok();
        let no_fdflags_sync_support = std::env::var("NO_FDFLAGS_SYNC_SUPPORT").is_ok();
        TestConfig {
            errno_mode,
            no_dangling_filesystem,
            no_fd_allocate,
            no_rename_dir_to_empty_dir,
            no_fdflags_sync_support,
        }
    }
    pub fn errno_expect_unix(&self) -> bool {
        match self.errno_mode {
            ErrnoMode::Unix | ErrnoMode::MacOS => true,
            _ => false,
        }
    }
    pub fn errno_expect_macos(&self) -> bool {
        match self.errno_mode {
            ErrnoMode::MacOS => true,
            _ => false,
        }
    }
    pub fn errno_expect_windows(&self) -> bool {
        match self.errno_mode {
            ErrnoMode::Windows => true,
            _ => false,
        }
    }
    pub fn support_dangling_filesystem(&self) -> bool {
        !self.no_dangling_filesystem
    }
    pub fn support_fd_allocate(&self) -> bool {
        !self.no_fd_allocate
    }
    pub fn support_rename_dir_to_empty_dir(&self) -> bool {
        !self.no_rename_dir_to_empty_dir
    }
    pub fn support_fdflags_sync(&self) -> bool {
        !self.no_fdflags_sync_support
    }
}

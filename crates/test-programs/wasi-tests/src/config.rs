pub struct TestConfig {
    errno_mode: ErrnoMode,
    no_dangling_symlinks: bool,
    no_fd_allocate: bool,
    no_rename_dir_to_empty_dir: bool,
    no_dangling_directory: bool,
}

enum ErrnoMode {
    Unix,
    Windows,
    Permissive,
}

impl TestConfig {
    pub fn from_env() -> Self {
        let errno_mode = if std::env::var("ERRNO_MODE_UNIX").is_ok() {
            ErrnoMode::Unix
        } else if std::env::var("ERRNO_MODE_WINDOWS").is_ok() {
            ErrnoMode::Windows
        } else {
            ErrnoMode::Permissive
        };
        let no_dangling_symlinks = std::env::var("NO_DANGLING_SYMLINKS").is_ok();
        let no_fd_allocate = std::env::var("NO_FD_ALLOCATE").is_ok();
        let no_rename_dir_to_empty_dir = std::env::var("NO_RENAME_DIR_TO_EMPTY_DIR").is_ok();
        let no_dangling_directory = std::env::var("NO_DANGLING_DIRECTORY").is_ok();
        TestConfig {
            errno_mode,
            no_dangling_symlinks,
            no_fd_allocate,
            no_rename_dir_to_empty_dir,
            no_dangling_directory,
        }
    }
    pub fn errno_expect_unix(&self) -> bool {
        match self.errno_mode {
            ErrnoMode::Unix => true,
            _ => false,
        }
    }
    pub fn errno_expect_windows(&self) -> bool {
        match self.errno_mode {
            ErrnoMode::Windows => true,
            _ => false,
        }
    }
    pub fn support_dangling_symlinks(&self) -> bool {
        !self.no_dangling_symlinks
    }
    pub fn support_fd_allocate(&self) -> bool {
        !self.no_fd_allocate
    }
    pub fn support_rename_dir_to_empty_dir(&self) -> bool {
        !self.no_rename_dir_to_empty_dir
    }
    pub fn support_dangling_directory(&self) -> bool {
        !self.no_dangling_directory
    }
}

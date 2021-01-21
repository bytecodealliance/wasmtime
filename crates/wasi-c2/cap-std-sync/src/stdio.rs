use wasi_c2::pipe::{ReadPipe, WritePipe};

pub type Stdin = ReadPipe<std::io::Stdin>;

pub fn stdin() -> Stdin {
    ReadPipe::new(std::io::stdin())
}

pub type Stdout = WritePipe<std::io::Stdout>;

pub fn stdout() -> Stdout {
    WritePipe::new(std::io::stdout())
}

pub type Stderr = WritePipe<std::io::Stderr>;

pub fn stderr() -> Stderr {
    WritePipe::new(std::io::stderr())
}

/*
#[cfg(windows)]
mod windows {
    use super::*;
    use std::os::windows::io::{AsRawHandle, RawHandle};
    impl AsRawHandle for Stdin {
        fn as_raw_handle(&self) -> RawHandle {
            self.borrow().as_raw_handle()
        }
    }
    impl AsRawHandle for Stdout {
        fn as_raw_handle(&self) -> RawHandle {
            self.borrow().as_raw_handle()
        }
    }
    impl AsRawHandle for Stderr {
        fn as_raw_handle(&self) -> RawHandle {
            self.borrow().as_raw_handle()
        }
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::io::{AsRawFd, RawFd};
    impl AsRawFd for Stdin {
        fn as_raw_fd(&self) -> RawFd {
            self.borrow().as_raw_fd()
        }
    }
    impl AsRawFd for Stdout {
        fn as_raw_fd(&self) -> RawFd {
            self.borrow().as_raw_fd()
        }
    }
    impl AsRawFd for Stderr {
        fn as_raw_fd(&self) -> RawFd {
            self.borrow().as_raw_fd()
        }
    }
}
*/

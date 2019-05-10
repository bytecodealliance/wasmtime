use crate::fdentry::FdEntry;
use crate::host;
use crate::wasm32;

use failure::{bail, format_err, Error};
use nix::unistd::dup;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{stderr, stdin, stdout};
use std::os::unix::prelude::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::path::{Path, PathBuf};

pub trait VmContext {
    fn as_wasi_ctx(&self) -> *const WasiCtx;
    fn as_wasi_ctx_mut(&mut self) -> *mut WasiCtx;

    unsafe fn dec_ptr(
        &mut self,
        ptr: wasm32::uintptr_t,
        len: usize,
    ) -> Result<*mut u8, host::__wasi_errno_t>;
}

pub struct WasiCtxBuilder {
    fds: HashMap<host::__wasi_fd_t, FdEntry>,
    preopens: HashMap<PathBuf, File>,
    args: Vec<CString>,
    env: HashMap<CString, CString>,
}

impl WasiCtxBuilder {
    /// Builder for a new `WasiCtx`.
    pub fn new() -> Self {
        let null = dev_null();
        WasiCtxBuilder {
            fds: HashMap::new(),
            preopens: HashMap::new(),
            args: vec![],
            env: HashMap::new(),
        }
        .fd_dup(0, &null)
        .fd_dup(1, &null)
        .fd_dup(2, &null)
    }

    pub fn args(mut self, args: &[&str]) -> Self {
        self.args = args
            .into_iter()
            .map(|arg| CString::new(*arg).expect("argument can be converted to a CString"))
            .collect();
        self
    }

    pub fn arg(mut self, arg: &str) -> Self {
        self.args
            .push(CString::new(arg).expect("argument can be converted to a CString"));
        self
    }

    pub fn c_args<S: AsRef<CStr>>(mut self, args: &[S]) -> Self {
        self.args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_owned())
            .collect();
        self
    }

    pub fn c_arg<S: AsRef<CStr>>(mut self, arg: S) -> Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    pub fn inherit_stdio(self) -> Self {
        self.fd_dup(0, &stdin())
            .fd_dup(1, &stdout())
            .fd_dup(2, &stderr())
    }

    pub fn inherit_env(mut self) -> Self {
        self.env = std::env::vars()
            .map(|(k, v)| {
                // TODO: handle errors, and possibly assert that the key is valid per POSIX
                (
                    CString::new(k).expect("environment key can be converted to a CString"),
                    CString::new(v).expect("environment value can be converted to a CString"),
                )
            })
            .collect();
        self
    }

    pub fn env(mut self, k: &str, v: &str) -> Self {
        self.env.insert(
            // TODO: handle errors, and possibly assert that the key is valid per POSIX
            CString::new(k).expect("environment key can be converted to a CString"),
            CString::new(v).expect("environment value can be converted to a CString"),
        );
        self
    }

    pub fn c_env<S, T>(mut self, k: S, v: T) -> Self
    where
        S: AsRef<CStr>,
        T: AsRef<CStr>,
    {
        self.env
            .insert(k.as_ref().to_owned(), v.as_ref().to_owned());
        self
    }

    /// Add an existing file-like object as a file descriptor in the context.
    ///
    /// When the `WasiCtx` is dropped, all of its associated file descriptors are `close`d. If you
    /// do not want this to close the existing object, use `WasiCtxBuilder::fd_dup()`.
    pub fn fd<F: IntoRawFd>(self, wasm_fd: host::__wasi_fd_t, fd: F) -> Self {
        // safe because we're getting a valid RawFd from the F directly
        unsafe { self.raw_fd(wasm_fd, fd.into_raw_fd()) }
    }

    /// Add an existing file-like object as a duplicate file descriptor in the context.
    ///
    /// The underlying file descriptor of this object will be duplicated before being added to the
    /// context, so it will not be closed when the `WasiCtx` is dropped.
    ///
    /// TODO: handle `dup` errors
    pub fn fd_dup<F: AsRawFd>(self, wasm_fd: host::__wasi_fd_t, fd: &F) -> Self {
        // safe because we're getting a valid RawFd from the F directly
        unsafe { self.raw_fd(wasm_fd, dup(fd.as_raw_fd()).unwrap()) }
    }

    /// Add an existing file descriptor to the context.
    ///
    /// When the `WasiCtx` is dropped, this file descriptor will be `close`d. If you do not want to
    /// close the existing descriptor, use `WasiCtxBuilder::raw_fd_dup()`.
    pub unsafe fn raw_fd(mut self, wasm_fd: host::__wasi_fd_t, fd: RawFd) -> Self {
        self.fds.insert(wasm_fd, FdEntry::from_raw_fd(fd));
        self
    }

    /// Add a duplicate of an existing file descriptor to the context.
    ///
    /// The file descriptor will be duplicated before being added to the context, so it will not be
    /// closed when the `WasiCtx` is dropped.
    ///
    /// TODO: handle `dup` errors
    pub unsafe fn raw_fd_dup(self, wasm_fd: host::__wasi_fd_t, fd: RawFd) -> Self {
        self.raw_fd(wasm_fd, dup(fd).unwrap())
    }

    pub fn preopened_dir<P: AsRef<Path>>(mut self, dir: File, guest_path: P) -> Self {
        self.preopens.insert(guest_path.as_ref().to_owned(), dir);
        self
    }

    pub fn build(mut self) -> Result<WasiCtx, Error> {
        // startup code starts looking at fd 3 for preopens
        let mut preopen_fd = 3;
        for (guest_path, dir) in self.preopens {
            if !dir.metadata()?.is_dir() {
                bail!("preopened file is not a directory");
            }
            while self.fds.contains_key(&preopen_fd) {
                preopen_fd = preopen_fd
                    .checked_add(1)
                    .ok_or(format_err!("not enough file handles"))?;
            }
            let mut fe = FdEntry::from_file(dir);
            fe.preopen_path = Some(guest_path);
            self.fds.insert(preopen_fd, fe);
            preopen_fd += 1;
        }

        let env = self
            .env
            .into_iter()
            .map(|(k, v)| {
                let mut pair = k.into_bytes();
                pair.extend_from_slice(b"=");
                pair.extend_from_slice(v.to_bytes_with_nul());
                // constructing a new CString from existing CStrings is safe
                unsafe { CString::from_vec_unchecked(pair) }
            })
            .collect();

        Ok(WasiCtx {
            fds: self.fds,
            args: self.args,
            env,
        })
    }
}

#[derive(Debug)]
pub struct WasiCtx {
    pub fds: HashMap<host::__wasi_fd_t, FdEntry>,
    pub args: Vec<CString>,
    pub env: Vec<CString>,
}

impl Default for WasiCtx {
    fn default() -> Self {
        Self {
            fds: HashMap::new(),
            args: Vec::new(),
            env: Vec::new(),
        }
    }
}

impl WasiCtx {
    /// Make a new `WasiCtx` with some default settings.
    ///
    /// - File descriptors 0, 1, and 2 inherit stdin, stdout, and stderr from the host process.
    ///
    /// - Environment variables are inherited from the host process.
    ///
    /// To override these behaviors, use `WasiCtxBuilder`.
    pub fn new(args: &[&str]) -> WasiCtx {
        WasiCtxBuilder::new()
            .args(args)
            .inherit_stdio()
            .inherit_env()
            .build()
            .expect("default options don't fail")
    }

    pub fn get_fd_entry(
        &self,
        fd: host::__wasi_fd_t,
        rights_base: host::__wasi_rights_t,
        rights_inheriting: host::__wasi_rights_t,
    ) -> Result<&FdEntry, host::__wasi_errno_t> {
        if let Some(fe) = self.fds.get(&fd) {
            // validate rights
            if !fe.rights_base & rights_base != 0 || !fe.rights_inheriting & rights_inheriting != 0
            {
                Err(host::__WASI_ENOTCAPABLE)
            } else {
                Ok(fe)
            }
        } else {
            Err(host::__WASI_EBADF)
        }
    }

    pub fn insert_fd_entry(
        &mut self,
        fe: FdEntry,
    ) -> Result<host::__wasi_fd_t, host::__wasi_errno_t> {
        // never insert where stdio handles usually are
        let mut fd = 3;
        while self.fds.contains_key(&fd) {
            if let Some(next_fd) = fd.checked_add(1) {
                fd = next_fd;
            } else {
                return Err(host::__WASI_EMFILE);
            }
        }
        self.fds.insert(fd, fe);
        Ok(fd)
    }
}

fn dev_null() -> File {
    File::open("/dev/null").expect("failed to open /dev/null")
}

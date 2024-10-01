use crate::bindings::wasi::cli::{stderr, stdin, stdout};
use crate::bindings::wasi::io::streams::{InputStream, OutputStream};
use crate::{BlockingMode, ImportAlloc, State, TrappingUnwrap, WasmStr};
use core::cell::{Cell, OnceCell, UnsafeCell};
use core::mem::MaybeUninit;
use core::num::NonZeroUsize;
use wasi::{Errno, Fd};

#[cfg(not(feature = "proxy"))]
use crate::bindings::wasi::filesystem::types as filesystem;
#[cfg(not(feature = "proxy"))]
use crate::File;

pub const MAX_DESCRIPTORS: usize = 128;

#[repr(C)]
pub enum Descriptor {
    /// A closed descriptor, holding a reference to the previous closed
    /// descriptor to support reusing them.
    Closed(Option<Fd>),

    /// Input and/or output wasi-streams, along with stream metadata.
    Streams(Streams),

    Bad,
}

/// Input and/or output wasi-streams, along with a stream type that
/// identifies what kind of stream they are and possibly supporting
/// type-specific operations like seeking.
pub struct Streams {
    /// The input stream, if present.
    pub input: OnceCell<InputStream>,

    /// The output stream, if present.
    pub output: OnceCell<OutputStream>,

    /// Information about the source of the stream.
    pub type_: StreamType,
}

impl Streams {
    /// Return the input stream, initializing it on the fly if needed.
    pub fn get_read_stream(&self) -> Result<&InputStream, Errno> {
        match self.input.get() {
            Some(wasi_stream) => Ok(wasi_stream),
            None => {
                let input = match &self.type_ {
                    // For directories, preview 1 behavior was to return ERRNO_BADF on attempts to read
                    // or write.
                    #[cfg(not(feature = "proxy"))]
                    StreamType::File(File {
                        descriptor_type: filesystem::DescriptorType::Directory,
                        ..
                    }) => return Err(wasi::ERRNO_BADF),
                    // For files, we may have adjusted the position for seeking, so
                    // create a new stream.
                    #[cfg(not(feature = "proxy"))]
                    StreamType::File(file) => {
                        let input = file.fd.read_via_stream(file.position.get())?;
                        input
                    }
                    _ => return Err(wasi::ERRNO_BADF),
                };
                self.input.set(input).trapping_unwrap();
                Ok(self.input.get().trapping_unwrap())
            }
        }
    }

    /// Return the output stream, initializing it on the fly if needed.
    pub fn get_write_stream(&self) -> Result<&OutputStream, Errno> {
        match self.output.get() {
            Some(wasi_stream) => Ok(wasi_stream),
            None => {
                let output = match &self.type_ {
                    // For directories, preview 1 behavior was to return ERRNO_BADF on attempts to read
                    // or write.
                    #[cfg(not(feature = "proxy"))]
                    StreamType::File(File {
                        descriptor_type: filesystem::DescriptorType::Directory,
                        ..
                    }) => return Err(wasi::ERRNO_BADF),
                    // For files, we may have adjusted the position for seeking, so
                    // create a new stream.
                    #[cfg(not(feature = "proxy"))]
                    StreamType::File(file) => {
                        let output = if file.append {
                            file.fd.append_via_stream()?
                        } else {
                            file.fd.write_via_stream(file.position.get())?
                        };
                        output
                    }
                    _ => return Err(wasi::ERRNO_BADF),
                };
                self.output.set(output).trapping_unwrap();
                Ok(self.output.get().trapping_unwrap())
            }
        }
    }
}

pub enum StreamType {
    /// Streams for implementing stdio.
    Stdio(Stdio),

    /// Streaming data with a file.
    #[cfg(not(feature = "proxy"))]
    File(File),
}

pub enum Stdio {
    Stdin,
    Stdout,
    Stderr,
}

impl Stdio {
    pub fn filetype(&self) -> wasi::Filetype {
        #[cfg(not(feature = "proxy"))]
        let is_terminal = {
            use crate::bindings::wasi::cli;
            match self {
                Stdio::Stdin => cli::terminal_stdin::get_terminal_stdin().is_some(),
                Stdio::Stdout => cli::terminal_stdout::get_terminal_stdout().is_some(),
                Stdio::Stderr => cli::terminal_stderr::get_terminal_stderr().is_some(),
            }
        };
        #[cfg(feature = "proxy")]
        let is_terminal = false;
        if is_terminal {
            wasi::FILETYPE_CHARACTER_DEVICE
        } else {
            wasi::FILETYPE_UNKNOWN
        }
    }
}

#[repr(C)]
pub struct Descriptors {
    /// Storage of mapping from preview1 file descriptors to preview2 file
    /// descriptors.
    table: UnsafeCell<MaybeUninit<[Descriptor; MAX_DESCRIPTORS]>>,
    table_len: Cell<u16>,

    /// Points to the head of a free-list of closed file descriptors.
    closed: Option<Fd>,
}

#[cfg(not(feature = "proxy"))]
#[link(wasm_import_module = "wasi:filesystem/preopens@0.2.1")]
extern "C" {
    #[link_name = "get-directories"]
    fn wasi_filesystem_get_directories(rval: *mut PreopenList);
}

impl Descriptors {
    pub fn new(state: &State) -> Self {
        let d = Descriptors {
            table: UnsafeCell::new(MaybeUninit::uninit()),
            table_len: Cell::new(0),
            closed: None,
        };

        fn new_once<T>(val: T) -> OnceCell<T> {
            let cell = OnceCell::new();
            let _ = cell.set(val);
            cell
        }

        d.push(Descriptor::Streams(Streams {
            input: new_once(stdin::get_stdin()),
            output: OnceCell::new(),
            type_: StreamType::Stdio(Stdio::Stdin),
        }))
        .trapping_unwrap();
        d.push(Descriptor::Streams(Streams {
            input: OnceCell::new(),
            output: new_once(stdout::get_stdout()),
            type_: StreamType::Stdio(Stdio::Stdout),
        }))
        .trapping_unwrap();
        d.push(Descriptor::Streams(Streams {
            input: OnceCell::new(),
            output: new_once(stderr::get_stderr()),
            type_: StreamType::Stdio(Stdio::Stderr),
        }))
        .trapping_unwrap();

        #[cfg(not(feature = "proxy"))]
        d.open_preopens(state);
        d
    }

    #[cfg(not(feature = "proxy"))]
    fn open_preopens(&self, state: &State) {
        unsafe {
            let alloc = ImportAlloc::CountAndDiscardStrings {
                strings_size: 0,
                alloc: state.temporary_alloc(),
            };
            let (preopens, _) = state.with_import_alloc(alloc, || {
                let mut preopens = PreopenList {
                    base: std::ptr::null(),
                    len: 0,
                };
                wasi_filesystem_get_directories(&mut preopens);
                preopens
            });
            for i in 0..preopens.len {
                let preopen = preopens.base.add(i).read();
                // Expectation is that the descriptor index is initialized with
                // stdio (0,1,2) and no others, so that preopens are 3..
                let descriptor_type = preopen.descriptor.get_type().trapping_unwrap();
                self.push(Descriptor::Streams(Streams {
                    input: OnceCell::new(),
                    output: OnceCell::new(),
                    type_: StreamType::File(File {
                        fd: preopen.descriptor,
                        descriptor_type,
                        position: Cell::new(0),
                        append: false,
                        blocking_mode: BlockingMode::Blocking,
                        preopen_name_len: NonZeroUsize::new(preopen.path.len),
                    }),
                }))
                .trapping_unwrap();
            }
        }
    }

    #[cfg(not(feature = "proxy"))]
    pub unsafe fn get_preopen_path(&self, state: &State, fd: Fd, path: *mut u8, len: usize) {
        let nth = fd - 3;
        let alloc = ImportAlloc::GetPreopenPath {
            cur: 0,
            nth,
            alloc: state.temporary_alloc(),
        };
        let (preopens, _) = state.with_import_alloc(alloc, || {
            let mut preopens = PreopenList {
                base: std::ptr::null(),
                len: 0,
            };
            wasi_filesystem_get_directories(&mut preopens);
            preopens
        });

        // NB: we just got owned handles for all preopened directories. We're
        // only interested in one individual string allocation, however, so
        // discard all of the descriptors and close them since we otherwise
        // don't want to leak them.
        for i in 0..preopens.len {
            let preopen = preopens.base.add(i).read();
            drop(preopen.descriptor);

            if (i as u32) != nth {
                continue;
            }
            assert!(preopen.path.len <= len);
            core::ptr::copy(preopen.path.ptr, path, preopen.path.len);
        }
    }

    fn push(&self, desc: Descriptor) -> Result<Fd, Errno> {
        unsafe {
            let table = (*self.table.get()).as_mut_ptr();
            let len = usize::from(self.table_len.get());
            if len >= (*table).len() {
                return Err(wasi::ERRNO_NOMEM);
            }
            core::ptr::addr_of_mut!((*table)[len]).write(desc);
            self.table_len.set(u16::try_from(len + 1).trapping_unwrap());
            Ok(Fd::from(u32::try_from(len).trapping_unwrap()))
        }
    }

    fn table(&self) -> &[Descriptor] {
        unsafe {
            std::slice::from_raw_parts(
                (*self.table.get()).as_ptr().cast(),
                usize::from(self.table_len.get()),
            )
        }
    }

    fn table_mut(&mut self) -> &mut [Descriptor] {
        unsafe {
            std::slice::from_raw_parts_mut(
                (*self.table.get()).as_mut_ptr().cast(),
                usize::from(self.table_len.get()),
            )
        }
    }

    pub fn open(&mut self, d: Descriptor) -> Result<Fd, Errno> {
        match self.closed {
            // No closed descriptors: expand table
            None => self.push(d),
            Some(freelist_head) => {
                // Pop an item off the freelist
                let freelist_desc = self.get_mut(freelist_head).trapping_unwrap();
                let next_closed = match freelist_desc {
                    Descriptor::Closed(next) => *next,
                    _ => unreachable!("impossible: freelist points to a closed descriptor"),
                };
                // Write descriptor to the entry at the head of the list
                *freelist_desc = d;
                // Point closed to the following item
                self.closed = next_closed;
                Ok(freelist_head)
            }
        }
    }

    pub fn get(&self, fd: Fd) -> Result<&Descriptor, Errno> {
        self.table()
            .get(usize::try_from(fd).trapping_unwrap())
            .ok_or(wasi::ERRNO_BADF)
    }

    pub fn get_mut(&mut self, fd: Fd) -> Result<&mut Descriptor, Errno> {
        self.table_mut()
            .get_mut(usize::try_from(fd).trapping_unwrap())
            .ok_or(wasi::ERRNO_BADF)
    }

    // Internal: close a fd, returning the descriptor.
    fn close_(&mut self, fd: Fd) -> Result<Descriptor, Errno> {
        // Throw an error if closing an fd which is already closed
        match self.get(fd)? {
            Descriptor::Closed(_) => Err(wasi::ERRNO_BADF)?,
            _ => {}
        }
        // Mutate the descriptor to be closed, and push the closed fd onto the head of the linked list:
        let last_closed = self.closed;
        let prev = std::mem::replace(self.get_mut(fd)?, Descriptor::Closed(last_closed));
        self.closed = Some(fd);
        Ok(prev)
    }

    // Close an fd.
    pub fn close(&mut self, fd: Fd) -> Result<(), Errno> {
        drop(self.close_(fd)?);
        Ok(())
    }

    // Expand the table by pushing a closed descriptor to the end. Used for renumbering.
    fn push_closed(&mut self) -> Result<(), Errno> {
        let old_closed = self.closed;
        let new_closed = self.push(Descriptor::Closed(old_closed))?;
        self.closed = Some(new_closed);
        Ok(())
    }

    // Implementation of fd_renumber
    pub fn renumber(&mut self, from_fd: Fd, to_fd: Fd) -> Result<(), Errno> {
        // First, ensure from_fd is in bounds:
        let _ = self.get(from_fd)?;
        // Expand table until to_fd is in bounds as well:
        while self.table_len.get() as u32 <= to_fd {
            self.push_closed()?;
        }
        // Then, close from_fd and put its contents into to_fd:
        let desc = self.close_(from_fd)?;
        // TODO FIXME if this overwrites a preopen, do we need to clear it from the preopen table?
        *self.get_mut(to_fd)? = desc;

        Ok(())
    }

    // A bunch of helper functions implemented in terms of the above pub functions:

    pub fn get_stream_with_error_mut(
        &mut self,
        fd: Fd,
        error: Errno,
    ) -> Result<&mut Streams, Errno> {
        match self.get_mut(fd)? {
            Descriptor::Streams(streams) => Ok(streams),
            Descriptor::Closed(_) | Descriptor::Bad => Err(error),
        }
    }

    #[cfg(not(feature = "proxy"))]
    pub fn get_file_with_error(&self, fd: Fd, error: Errno) -> Result<&File, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(Streams {
                type_:
                    StreamType::File(File {
                        descriptor_type: filesystem::DescriptorType::Directory,
                        ..
                    }),
                ..
            }) => Err(wasi::ERRNO_BADF),
            Descriptor::Streams(Streams {
                type_: StreamType::File(file),
                ..
            }) => Ok(file),
            Descriptor::Closed(_) => Err(wasi::ERRNO_BADF),
            _ => Err(error),
        }
    }

    #[cfg(not(feature = "proxy"))]
    pub fn get_file(&self, fd: Fd) -> Result<&File, Errno> {
        self.get_file_with_error(fd, wasi::ERRNO_INVAL)
    }

    #[cfg(not(feature = "proxy"))]
    pub fn get_dir(&self, fd: Fd) -> Result<&File, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(Streams {
                type_:
                    StreamType::File(
                        file @ File {
                            descriptor_type: filesystem::DescriptorType::Directory,
                            ..
                        },
                    ),
                ..
            }) => Ok(file),
            Descriptor::Streams(Streams {
                type_: StreamType::File(File { .. }),
                ..
            }) => Err(wasi::ERRNO_NOTDIR),
            _ => Err(wasi::ERRNO_BADF),
        }
    }

    #[cfg(not(feature = "proxy"))]
    pub fn get_seekable_file(&self, fd: Fd) -> Result<&File, Errno> {
        self.get_file_with_error(fd, wasi::ERRNO_SPIPE)
    }

    pub fn get_seekable_stream_mut(&mut self, fd: Fd) -> Result<&mut Streams, Errno> {
        self.get_stream_with_error_mut(fd, wasi::ERRNO_SPIPE)
    }

    pub fn get_read_stream(&self, fd: Fd) -> Result<&InputStream, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(streams) => streams.get_read_stream(),
            Descriptor::Closed(_) | Descriptor::Bad => Err(wasi::ERRNO_BADF),
        }
    }

    pub fn get_write_stream(&self, fd: Fd) -> Result<&OutputStream, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(streams) => streams.get_write_stream(),
            Descriptor::Closed(_) | Descriptor::Bad => Err(wasi::ERRNO_BADF),
        }
    }
}

#[cfg(not(feature = "proxy"))]
#[repr(C)]
pub struct Preopen {
    pub descriptor: filesystem::Descriptor,
    pub path: WasmStr,
}

#[cfg(not(feature = "proxy"))]
#[repr(C)]
pub struct PreopenList {
    pub base: *const Preopen,
    pub len: usize,
}

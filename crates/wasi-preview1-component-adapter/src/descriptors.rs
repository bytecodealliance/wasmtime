use crate::bindings::wasi::cli_base::{stderr, stdin, stdout};
use crate::bindings::wasi::filesystem::types as filesystem;
use crate::bindings::wasi::io::streams::{self, InputStream, OutputStream};
use crate::bindings::wasi::sockets::tcp;
use crate::{set_stderr_stream, BumpArena, File, ImportAlloc, TrappingUnwrap, WasmStr};
use core::cell::{Cell, UnsafeCell};
use core::mem::MaybeUninit;
use wasi::{Errno, Fd};

pub const MAX_DESCRIPTORS: usize = 128;

#[repr(C)]
pub enum Descriptor {
    /// A closed descriptor, holding a reference to the previous closed
    /// descriptor to support reusing them.
    Closed(Option<Fd>),

    /// Input and/or output wasi-streams, along with stream metadata.
    Streams(Streams),
}

impl Drop for Descriptor {
    fn drop(&mut self) {
        match self {
            Descriptor::Streams(stream) => {
                if let Some(input) = stream.input.get() {
                    streams::drop_input_stream(input);
                }
                if let Some(output) = stream.output.get() {
                    streams::drop_output_stream(output);
                }
                match &stream.type_ {
                    StreamType::File(file) => filesystem::drop_descriptor(file.fd),
                    StreamType::Socket(_) => unreachable!(),
                    StreamType::Stdio => {}
                }
            }
            Descriptor::Closed(_) => {}
        }
    }
}

/// Input and/or output wasi-streams, along with a stream type that
/// identifies what kind of stream they are and possibly supporting
/// type-specific operations like seeking.
pub struct Streams {
    /// The output stream, if present.
    pub input: Cell<Option<InputStream>>,

    /// The input stream, if present.
    pub output: Cell<Option<OutputStream>>,

    /// Information about the source of the stream.
    pub type_: StreamType,
}

impl Streams {
    /// Return the input stream, initializing it on the fly if needed.
    pub fn get_read_stream(&self) -> Result<InputStream, Errno> {
        match &self.input.get() {
            Some(wasi_stream) => Ok(*wasi_stream),
            None => match &self.type_ {
                // For directories, preview 1 behavior was to return ERRNO_BADF on attempts to read
                // or write.
                StreamType::File(File {
                    descriptor_type: filesystem::DescriptorType::Directory,
                    ..
                }) => Err(wasi::ERRNO_BADF),
                // For files, we may have adjusted the position for seeking, so
                // create a new stream.
                StreamType::File(file) => {
                    let input = filesystem::read_via_stream(file.fd, file.position.get())?;
                    self.input.set(Some(input));
                    Ok(input)
                }
                _ => Err(wasi::ERRNO_BADF),
            },
        }
    }

    /// Return the output stream, initializing it on the fly if needed.
    pub fn get_write_stream(&self) -> Result<OutputStream, Errno> {
        match &self.output.get() {
            Some(wasi_stream) => Ok(*wasi_stream),
            None => match &self.type_ {
                // For directories, preview 1 behavior was to return ERRNO_BADF on attempts to read
                // or write.
                StreamType::File(File {
                    descriptor_type: filesystem::DescriptorType::Directory,
                    ..
                }) => Err(wasi::ERRNO_BADF),
                // For files, we may have adjusted the position for seeking, so
                // create a new stream.
                StreamType::File(file) => {
                    let output = if file.append {
                        filesystem::append_via_stream(file.fd)?
                    } else {
                        filesystem::write_via_stream(file.fd, file.position.get())?
                    };
                    self.output.set(Some(output));
                    Ok(output)
                }
                _ => Err(wasi::ERRNO_BADF),
            },
        }
    }
}

#[allow(dead_code)] // until Socket is implemented
pub enum StreamType {
    /// Stream is used for implementing stdio.
    Stdio,

    /// Streaming data with a file.
    File(File),

    /// Streaming data with a socket connection.
    Socket(tcp::TcpSocket),
}

#[repr(C)]
pub struct Descriptors {
    /// Storage of mapping from preview1 file descriptors to preview2 file
    /// descriptors.
    table: UnsafeCell<MaybeUninit<[Descriptor; MAX_DESCRIPTORS]>>,
    table_len: Cell<u16>,

    /// Points to the head of a free-list of closed file descriptors.
    closed: Option<Fd>,

    /// Preopened directories. Initialized lazily. Access with `State::get_preopens`
    /// to take care of initialization.
    preopens: Cell<Option<&'static [Preopen]>>,
}

impl Descriptors {
    pub fn new(import_alloc: &ImportAlloc, arena: &BumpArena) -> Self {
        let d = Descriptors {
            table: UnsafeCell::new(MaybeUninit::uninit()),
            table_len: Cell::new(0),
            closed: None,
            preopens: Cell::new(None),
        };

        let stdin = stdin::get_stdin();
        let stdout = stdout::get_stdout();
        let stderr = stderr::get_stderr();
        unsafe { set_stderr_stream(stderr) };

        d.push(Descriptor::Streams(Streams {
            input: Cell::new(Some(stdin)),
            output: Cell::new(None),
            type_: StreamType::Stdio,
        }))
        .trapping_unwrap();
        d.push(Descriptor::Streams(Streams {
            input: Cell::new(None),
            output: Cell::new(Some(stdout)),
            type_: StreamType::Stdio,
        }))
        .trapping_unwrap();
        d.push(Descriptor::Streams(Streams {
            input: Cell::new(None),
            output: Cell::new(Some(stderr)),
            type_: StreamType::Stdio,
        }))
        .trapping_unwrap();

        #[link(wasm_import_module = "wasi:filesystem/preopens")]
        extern "C" {
            #[link_name = "get-directories"]
            fn get_preopens_import(rval: *mut PreopenList);
        }
        let mut list = PreopenList {
            base: std::ptr::null(),
            len: 0,
        };
        import_alloc.with_arena(arena, || unsafe {
            get_preopens_import(&mut list as *mut _)
        });
        let preopens: &'static [Preopen] = unsafe {
            // allocation comes from long lived arena, so it is safe to
            // cast this to a &'static slice:
            std::slice::from_raw_parts(list.base, list.len)
        };
        for preopen in preopens {
            // Expectation is that the descriptor index is initialized with
            // stdio (0,1,2) and no others, so that preopens are 3..
            d.push(Descriptor::Streams(Streams {
                input: Cell::new(None),
                output: Cell::new(None),
                type_: StreamType::File(File {
                    fd: preopen.descriptor,
                    descriptor_type: filesystem::get_type(preopen.descriptor).trapping_unwrap(),
                    position: Cell::new(0),
                    append: false,
                    blocking: false,
                }),
            }))
            .trapping_unwrap();
        }

        d.preopens.set(Some(preopens));
        d
    }

    fn push(&self, desc: Descriptor) -> Result<Fd, Errno> {
        unsafe {
            let table = (*self.table.get()).as_mut_ptr();
            let len = usize::try_from(self.table_len.get()).trapping_unwrap();
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
                usize::try_from(self.table_len.get()).trapping_unwrap(),
            )
        }
    }

    fn table_mut(&mut self) -> &mut [Descriptor] {
        unsafe {
            std::slice::from_raw_parts_mut(
                (*self.table.get()).as_mut_ptr().cast(),
                usize::try_from(self.table_len.get()).trapping_unwrap(),
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
                // Write descriptor to the entry at the nead of the list
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

    pub fn get_preopen(&self, fd: Fd) -> Option<&Preopen> {
        let preopens = self.preopens.get().trapping_unwrap();
        // Subtract 3 for the stdio indices to compute the preopen index.
        let index = fd.checked_sub(3)? as usize;
        preopens.get(index)
    }

    // Internal: close a fd, returning the descriptor.
    fn close_(&mut self, fd: Fd) -> Result<Descriptor, Errno> {
        // Throw an error if closing an fd which is already closed
        match self.get_mut(fd)? {
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
        while self.table_len.get() as u32 <= to_fd as u32 {
            self.push_closed()?;
        }
        // Then, close from_fd and put its contents into to_fd:
        let desc = self.close_(from_fd)?;
        // TODO FIXME if this overwrites a preopen, do we need to clear it from the preopen table?
        *self.get_mut(to_fd)? = desc;

        Ok(())
    }

    // A bunch of helper functions implemented in terms of the above pub functions:

    pub fn get_stream_with_error(&self, fd: Fd, error: Errno) -> Result<&Streams, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(streams) => Ok(streams),
            Descriptor::Closed(_) => Err(wasi::ERRNO_BADF),
        }
    }

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

    #[allow(dead_code)] // until Socket is implemented
    pub fn get_socket(&self, fd: Fd) -> Result<tcp::TcpSocket, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(Streams {
                type_: StreamType::Socket(socket),
                ..
            }) => Ok(*socket),
            Descriptor::Closed(_) => Err(wasi::ERRNO_BADF),
            _ => Err(wasi::ERRNO_INVAL),
        }
    }

    pub fn get_file(&self, fd: Fd) -> Result<&File, Errno> {
        self.get_file_with_error(fd, wasi::ERRNO_INVAL)
    }

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
                type_:
                    StreamType::File(
                        file @ File {
                            descriptor_type: _, ..
                        },
                    ),
                ..
            }) => Err(wasi::ERRNO_NOTDIR),
            _ => Err(wasi::ERRNO_BADF),
        }
    }

    pub fn get_seekable_file(&self, fd: Fd) -> Result<&File, Errno> {
        self.get_file_with_error(fd, wasi::ERRNO_SPIPE)
    }

    pub fn get_seekable_stream(&self, fd: Fd) -> Result<&Streams, Errno> {
        self.get_stream_with_error(fd, wasi::ERRNO_SPIPE)
    }

    pub fn get_read_stream(&self, fd: Fd) -> Result<InputStream, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(streams) => streams.get_read_stream(),
            Descriptor::Closed(_) => Err(wasi::ERRNO_BADF),
        }
    }

    pub fn get_write_stream(&self, fd: Fd) -> Result<OutputStream, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(streams) => streams.get_write_stream(),
            Descriptor::Closed(_) => Err(wasi::ERRNO_BADF),
        }
    }
}

#[repr(C)]
pub struct Preopen {
    pub descriptor: u32,
    pub path: WasmStr,
}

#[repr(C)]
pub struct PreopenList {
    pub base: *const Preopen,
    pub len: usize,
}

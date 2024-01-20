use crate::preview2::bindings::cli::{
    stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr, terminal_stdin,
    terminal_stdout,
};
use crate::preview2::bindings::io::streams;
use crate::preview2::pipe;
use crate::preview2::{
    HostInputStream, HostOutputStream, StreamError, StreamResult, Subscribe, WasiView,
};
use bytes::Bytes;
use std::io::IsTerminal;
use wasmtime::component::Resource;

/// A trait used to represent the standard input to a guest program.
///
/// This is used to implement various WASI APIs via the method implementations
/// below.
///
/// Built-in implementations are provided for [`Stdin`],
/// [`pipe::MemoryInputPipe`], and [`pipe::ClosedInputStream`].
pub trait StdinStream: Send {
    /// Creates a fresh stream which is reading stdin.
    ///
    /// Note that the returned stream must share state with all other streams
    /// previously created. Guests may create multiple handles to the same stdin
    /// and they should all be synchronized in their progress through the
    /// program's input.
    ///
    /// Note that this means that if one handle becomes ready for reading they
    /// all become ready for reading. Subsequently if one is read from it may
    /// mean that all the others are no longer ready for reading. This is
    /// basically a consequence of the way the WIT APIs are designed today.
    fn stream(&self) -> Box<dyn HostInputStream>;

    /// Returns whether this stream is backed by a TTY.
    fn isatty(&self) -> bool;
}

impl StdinStream for pipe::MemoryInputPipe {
    fn stream(&self) -> Box<dyn HostInputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}

impl StdinStream for pipe::ClosedInputStream {
    fn stream(&self) -> Box<dyn HostInputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}

mod worker_thread_stdin;
pub use self::worker_thread_stdin::{stdin, Stdin};

/// Similar to [`StdinStream`], except for output.
pub trait StdoutStream: Send {
    /// Returns a fresh new stream which can write to this output stream.
    ///
    /// Note that all output streams should output to the same logical source.
    /// This means that it's possible for each independent stream to acquire a
    /// separate "permit" to write and then act on that permit. Note that
    /// additionally at this time once a permit is "acquired" there's no way to
    /// release it, for example you can wait for readiness and then never
    /// actually write in WASI. This means that acquisition of a permit for one
    /// stream cannot discount the size of a permit another stream could
    /// obtain.
    ///
    /// Implementations must be able to handle this
    fn stream(&self) -> Box<dyn HostOutputStream>;

    /// Returns whether this stream is backed by a TTY.
    fn isatty(&self) -> bool;
}

impl StdoutStream for pipe::MemoryOutputPipe {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}

impl StdoutStream for pipe::SinkOutputStream {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}

impl StdoutStream for pipe::ClosedOutputStream {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}

/// This implementation will yield output streams that block on writes, as they
/// inherit the implementation directly from the rust std library. A different
/// implementation of [`StdoutStream`] will be necessary if truly async output
/// streams are required.
pub struct Stdout;

pub fn stdout() -> Stdout {
    Stdout
}

impl StdoutStream for Stdout {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(OutputStream::Stdout)
    }

    fn isatty(&self) -> bool {
        std::io::stdout().is_terminal()
    }
}

/// This implementation will yield output streams that block on writes, as they
/// inherit the implementation directly from the rust std library. A different
/// implementation of [`StdoutStream`] will be necessary if truly async output
/// streams are required.
pub struct Stderr;

pub fn stderr() -> Stderr {
    Stderr
}

impl StdoutStream for Stderr {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(OutputStream::Stderr)
    }

    fn isatty(&self) -> bool {
        std::io::stderr().is_terminal()
    }
}

enum OutputStream {
    Stdout,
    Stderr,
}

impl HostOutputStream for OutputStream {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        use std::io::Write;
        match self {
            OutputStream::Stdout => std::io::stdout().write_all(&bytes),
            OutputStream::Stderr => std::io::stderr().write_all(&bytes),
        }
        .map_err(|e| StreamError::LastOperationFailed(anyhow::anyhow!(e)))
    }

    fn flush(&mut self) -> StreamResult<()> {
        use std::io::Write;
        match self {
            OutputStream::Stdout => std::io::stdout().flush(),
            OutputStream::Stderr => std::io::stderr().flush(),
        }
        .map_err(|e| StreamError::LastOperationFailed(anyhow::anyhow!(e)))
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        Ok(1024 * 1024)
    }
}

#[async_trait::async_trait]
impl Subscribe for OutputStream {
    async fn ready(&mut self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsATTY {
    Yes,
    No,
}

impl<T: WasiView> stdin::Host for T {
    fn get_stdin(&mut self) -> Result<Resource<streams::InputStream>, anyhow::Error> {
        let stream = self.ctx().stdin.stream();
        Ok(self.table().push(streams::InputStream::Host(stream))?)
    }
}

impl<T: WasiView> stdout::Host for T {
    fn get_stdout(&mut self) -> Result<Resource<streams::OutputStream>, anyhow::Error> {
        let stream = self.ctx().stdout.stream();
        Ok(self.table().push(stream)?)
    }
}

impl<T: WasiView> stderr::Host for T {
    fn get_stderr(&mut self) -> Result<Resource<streams::OutputStream>, anyhow::Error> {
        let stream = self.ctx().stderr.stream();
        Ok(self.table().push(stream)?)
    }
}

pub struct TerminalInput;
pub struct TerminalOutput;

impl<T: WasiView> terminal_input::Host for T {}
impl<T: WasiView> terminal_input::HostTerminalInput for T {
    fn drop(&mut self, r: Resource<TerminalInput>) -> anyhow::Result<()> {
        self.table().delete(r)?;
        Ok(())
    }
}
impl<T: WasiView> terminal_output::Host for T {}
impl<T: WasiView> terminal_output::HostTerminalOutput for T {
    fn drop(&mut self, r: Resource<TerminalOutput>) -> anyhow::Result<()> {
        self.table().delete(r)?;
        Ok(())
    }
}
impl<T: WasiView> terminal_stdin::Host for T {
    fn get_terminal_stdin(&mut self) -> anyhow::Result<Option<Resource<TerminalInput>>> {
        if self.ctx().stdin.isatty() {
            let fd = self.table().push(TerminalInput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}
impl<T: WasiView> terminal_stdout::Host for T {
    fn get_terminal_stdout(&mut self) -> anyhow::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx().stdout.isatty() {
            let fd = self.table().push(TerminalOutput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}
impl<T: WasiView> terminal_stderr::Host for T {
    fn get_terminal_stderr(&mut self) -> anyhow::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx().stderr.isatty() {
            let fd = self.table().push(TerminalOutput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}

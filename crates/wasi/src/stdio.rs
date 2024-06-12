use crate::bindings::cli::{
    stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr, terminal_stdin,
    terminal_stdout,
};
use crate::bindings::io::streams;
use crate::pipe;
use crate::{
    HostInputStream, HostOutputStream, StreamError, StreamResult, Subscribe, WasiImpl, WasiView,
};
use bytes::Bytes;
use std::io::IsTerminal;
use std::sync::Arc;
use tokio::sync::Mutex;
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

/// An impl of [`StdinStream`] built on top of [`crate::pipe::AsyncReadStream`].
//
// Note the usage of `tokio::sync::Mutex` here as opposed to a
// `std::sync::Mutex`. This is intentionally done to implement the `Subscribe`
// variant of this trait. Note that in doing so we're left with the quandry of
// how to implement methods of `HostInputStream` since those methods are not
// `async`. They're currently implemented with `try_lock`, which then raises the
// question of what to do on contention. Currently traps are returned.
//
// Why should it be ok to return a trap? In general concurrency/contention
// shouldn't return a trap since it should be able to happen normally. The
// current assumption, though, is that WASI stdin/stdout streams are special
// enough that the contention case should never come up in practice. Currently
// in WASI there is no actually concurrency, there's just the items in a single
// `Store` and that store owns all of its I/O in a single Tokio task. There's no
// means to actually spawn multiple Tokio tasks that use the same store. This
// means at the very least that there's zero parallelism. Due to the lack of
// multiple tasks that also means that there's no concurrency either.
//
// This `AsyncStdinStream` wrapper is only intended to be used by the WASI
// bindings themselves. It's possible for the host to take this and work with it
// on its own task, but that's niche enough it's not designed for.
//
// Overall that means that the guest is either calling `Subscribe` or it's
// calling `HostInputStream` methods. This means that there should never be
// contention between the two at this time. This may all change in the future
// with WASI 0.3, but perhaps we'll have a better story for stdio at that time
// (see the doc block on the `HostOutputStream` impl below)
pub struct AsyncStdinStream(Arc<Mutex<crate::pipe::AsyncReadStream>>);

impl AsyncStdinStream {
    pub fn new(s: crate::pipe::AsyncReadStream) -> Self {
        Self(Arc::new(Mutex::new(s)))
    }
}

impl StdinStream for AsyncStdinStream {
    fn stream(&self) -> Box<dyn HostInputStream> {
        Box::new(Self(self.0.clone()))
    }
    fn isatty(&self) -> bool {
        false
    }
}

impl HostInputStream for AsyncStdinStream {
    fn read(&mut self, size: usize) -> Result<bytes::Bytes, StreamError> {
        match self.0.try_lock() {
            Ok(mut stream) => stream.read(size),
            Err(_) => Err(StreamError::trap("concurrent reads are not supported")),
        }
    }
    fn skip(&mut self, size: usize) -> Result<usize, StreamError> {
        match self.0.try_lock() {
            Ok(mut stream) => stream.skip(size),
            Err(_) => Err(StreamError::trap("concurrent skips are not supported")),
        }
    }
}

#[async_trait::async_trait]
impl Subscribe for AsyncStdinStream {
    async fn ready(&mut self) {
        self.0.lock().await.ready().await
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

/// This implementation will yield output streams that block on writes, and
/// output directly to a file. If truly async output is required, [`AsyncStdoutStream`]
/// should be used instead.
pub struct OutputFile {
    file: Arc<std::fs::File>,
}

impl OutputFile {
    pub fn new(file: std::fs::File) -> Self {
        Self {
            file: Arc::new(file),
        }
    }
}

impl StdoutStream for OutputFile {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(OutputFileStream {
            file: Arc::clone(&self.file),
        })
    }

    fn isatty(&self) -> bool {
        false
    }
}

struct OutputFileStream {
    file: Arc<std::fs::File>,
}

#[async_trait::async_trait]
impl Subscribe for OutputFileStream {
    async fn ready(&mut self) {}
}

impl HostOutputStream for OutputFileStream {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        use std::io::Write;
        self.file
            .write_all(&bytes)
            .map_err(|e| StreamError::LastOperationFailed(anyhow::anyhow!(e)))
    }

    fn flush(&mut self) -> StreamResult<()> {
        use std::io::Write;
        self.file
            .flush()
            .map_err(|e| StreamError::LastOperationFailed(anyhow::anyhow!(e)))
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        Ok(1024 * 1024)
    }
}

/// This implementation will yield output streams that block on writes, as they
/// inherit the implementation directly from the rust std library. A different
/// implementation of [`StdoutStream`] will be necessary if truly async output
/// streams are required.
pub struct Stdout;

/// Returns a stream that represents the host's standard out.
///
/// Suitable for passing to
/// [`WasiCtxBuilder::stdout`](crate::WasiCtxBuilder::stdout).
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

/// Returns a stream that represents the host's standard err.
///
/// Suitable for passing to
/// [`WasiCtxBuilder::stderr`](crate::WasiCtxBuilder::stderr).
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

/// A wrapper of [`crate::pipe::AsyncWriteStream`] that implements
/// [`StdoutStream`]. Note that the [`HostOutputStream`] impl for this is not
/// correct when used for interleaved async IO.
//
// Note that the use of `tokio::sync::Mutex` here is intentional, in addition to
// the `try_lock()` calls below in the implementation of `HostOutputStream`. For
// more information see the documentation on `AsyncStdinStream`.
pub struct AsyncStdoutStream(Arc<Mutex<crate::pipe::AsyncWriteStream>>);

impl AsyncStdoutStream {
    pub fn new(s: crate::pipe::AsyncWriteStream) -> Self {
        Self(Arc::new(Mutex::new(s)))
    }
}

impl StdoutStream for AsyncStdoutStream {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(Self(self.0.clone()))
    }
    fn isatty(&self) -> bool {
        false
    }
}

// This implementation is known to be bogus. All check-writes and writes are
// directed at the same underlying stream. The check-write/write protocol does
// require the size returned by a check-write to be accepted by write, even if
// other side-effects happen between those calls, and this implementation
// permits another view (created by StdoutStream::stream()) of the same
// underlying stream to accept a write which will invalidate a prior
// check-write of another view.
// Ultimately, the Std{in,out}Stream::stream() methods exist because many
// different places in a linked component (which may itself contain many
// modules) may need to access stdio without any coordination to keep those
// accesses all using pointing to the same resource. So, we allow many
// resources to be created. We have the reasonable expectation that programs
// won't attempt to interleave async IO from these disparate uses of stdio.
// If that expectation doesn't turn out to be true, and you find yourself at
// this comment to correct it: sorry about that.
impl HostOutputStream for AsyncStdoutStream {
    fn check_write(&mut self) -> Result<usize, StreamError> {
        match self.0.try_lock() {
            Ok(mut stream) => stream.check_write(),
            Err(_) => Err(StreamError::trap("concurrent writes are not supported")),
        }
    }
    fn write(&mut self, bytes: Bytes) -> Result<(), StreamError> {
        match self.0.try_lock() {
            Ok(mut stream) => stream.write(bytes),
            Err(_) => Err(StreamError::trap("concurrent writes not supported yet")),
        }
    }
    fn flush(&mut self) -> Result<(), StreamError> {
        match self.0.try_lock() {
            Ok(mut stream) => stream.flush(),
            Err(_) => Err(StreamError::trap("concurrent flushes not supported yet")),
        }
    }
}

#[async_trait::async_trait]
impl Subscribe for AsyncStdoutStream {
    async fn ready(&mut self) {
        self.0.lock().await.ready().await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsATTY {
    Yes,
    No,
}

impl<T> stdin::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn get_stdin(&mut self) -> Result<Resource<streams::InputStream>, anyhow::Error> {
        let stream = self.ctx().stdin.stream();
        Ok(self.table().push(streams::InputStream::Host(stream))?)
    }
}

impl<T> stdout::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn get_stdout(&mut self) -> Result<Resource<streams::OutputStream>, anyhow::Error> {
        let stream = self.ctx().stdout.stream();
        Ok(self.table().push(stream)?)
    }
}

impl<T> stderr::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn get_stderr(&mut self) -> Result<Resource<streams::OutputStream>, anyhow::Error> {
        let stream = self.ctx().stderr.stream();
        Ok(self.table().push(stream)?)
    }
}

pub struct TerminalInput;
pub struct TerminalOutput;

impl<T> terminal_input::Host for WasiImpl<T> where T: WasiView {}
impl<T> terminal_input::HostTerminalInput for WasiImpl<T>
where
    T: WasiView,
{
    fn drop(&mut self, r: Resource<TerminalInput>) -> anyhow::Result<()> {
        self.table().delete(r)?;
        Ok(())
    }
}
impl<T> terminal_output::Host for WasiImpl<T> where T: WasiView {}
impl<T> terminal_output::HostTerminalOutput for WasiImpl<T>
where
    T: WasiView,
{
    fn drop(&mut self, r: Resource<TerminalOutput>) -> anyhow::Result<()> {
        self.table().delete(r)?;
        Ok(())
    }
}
impl<T> terminal_stdin::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn get_terminal_stdin(&mut self) -> anyhow::Result<Option<Resource<TerminalInput>>> {
        if self.ctx().stdin.isatty() {
            let fd = self.table().push(TerminalInput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}
impl<T> terminal_stdout::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn get_terminal_stdout(&mut self) -> anyhow::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx().stdout.isatty() {
            let fd = self.table().push(TerminalOutput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}
impl<T> terminal_stderr::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn get_terminal_stderr(&mut self) -> anyhow::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx().stderr.isatty() {
            let fd = self.table().push(TerminalOutput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::stdio::StdoutStream;
    use crate::write_stream::AsyncWriteStream;
    use crate::{AsyncStdoutStream, HostOutputStream};
    use anyhow::Result;
    use bytes::Bytes;
    use tokio::io::AsyncReadExt;

    #[test]
    fn memory_stdin_stream() {
        // A StdinStream has the property that there are multiple
        // HostInputStreams created, using the stream() method which are each
        // views on the same shared state underneath. Consuming input on one
        // stream results in consuming that input on all streams.
        //
        // The simplest way to measure this is to check if the MemoryInputPipe
        // impl of StdinStream follows this property.

        let pipe = super::pipe::MemoryInputPipe::new(
            "the quick brown fox jumped over the three lazy dogs",
        );

        use super::StdinStream;

        let mut view1 = pipe.stream();
        let mut view2 = pipe.stream();

        let read1 = view1.read(10).expect("read first 10 bytes");
        assert_eq!(read1, "the quick ".as_bytes(), "first 10 bytes");
        let read2 = view2.read(10).expect("read second 10 bytes");
        assert_eq!(read2, "brown fox ".as_bytes(), "second 10 bytes");
        let read3 = view1.read(10).expect("read third 10 bytes");
        assert_eq!(read3, "jumped ove".as_bytes(), "third 10 bytes");
        let read4 = view2.read(10).expect("read fourth 10 bytes");
        assert_eq!(read4, "r the thre".as_bytes(), "fourth 10 bytes");
    }

    #[tokio::test]
    async fn async_stdin_stream() {
        // A StdinStream has the property that there are multiple
        // HostInputStreams created, using the stream() method which are each
        // views on the same shared state underneath. Consuming input on one
        // stream results in consuming that input on all streams.
        //
        // AsyncStdinStream is a slightly more complex impl of StdinStream
        // than the MemoryInputPipe above. We can create an AsyncReadStream
        // from a file on the disk, and an AsyncStdinStream from that common
        // stream, then check that the same property holds as above.

        let dir = tempfile::tempdir().unwrap();
        let mut path = std::path::PathBuf::from(dir.path());
        path.push("file");
        std::fs::write(&path, "the quick brown fox jumped over the three lazy dogs").unwrap();

        let file = tokio::fs::File::open(&path)
            .await
            .expect("open created file");
        let stdin_stream = super::AsyncStdinStream::new(crate::pipe::AsyncReadStream::new(file));

        use super::StdinStream;

        let mut view1 = stdin_stream.stream();
        let mut view2 = stdin_stream.stream();

        view1.ready().await;

        let read1 = view1.read(10).expect("read first 10 bytes");
        assert_eq!(read1, "the quick ".as_bytes(), "first 10 bytes");
        let read2 = view2.read(10).expect("read second 10 bytes");
        assert_eq!(read2, "brown fox ".as_bytes(), "second 10 bytes");
        let read3 = view1.read(10).expect("read third 10 bytes");
        assert_eq!(read3, "jumped ove".as_bytes(), "third 10 bytes");
        let read4 = view2.read(10).expect("read fourth 10 bytes");
        assert_eq!(read4, "r the thre".as_bytes(), "fourth 10 bytes");
    }

    #[tokio::test]
    async fn async_stdout_stream_unblocks() {
        let (mut read, write) = tokio::io::duplex(32);
        let stdout = AsyncStdoutStream::new(AsyncWriteStream::new(32, write));

        let task = tokio::task::spawn(async move {
            let mut stream = stdout.stream();
            blocking_write_and_flush(&mut *stream, "x".into())
                .await
                .unwrap();
        });

        let mut buf = [0; 100];
        let n = read.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"x");

        task.await.unwrap();
    }

    async fn blocking_write_and_flush(
        s: &mut dyn HostOutputStream,
        mut bytes: Bytes,
    ) -> Result<()> {
        while !bytes.is_empty() {
            let permit = s.write_ready().await?;
            let len = bytes.len().min(permit);
            let chunk = bytes.split_to(len);
            s.write(chunk)?;
        }

        s.flush()?;
        s.write_ready().await?;
        Ok(())
    }
}

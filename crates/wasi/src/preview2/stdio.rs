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
pub trait StdinStream: Send + Sync {
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
pub trait StdoutStream: Send + Sync {
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
        let stream = self.ctx_mut().stdin.stream();
        Ok(self.table_mut().push(streams::InputStream::Host(stream))?)
    }
}

impl<T: WasiView> stdout::Host for T {
    fn get_stdout(&mut self) -> Result<Resource<streams::OutputStream>, anyhow::Error> {
        let stream = self.ctx_mut().stdout.stream();
        Ok(self.table_mut().push(stream)?)
    }
}

impl<T: WasiView> stderr::Host for T {
    fn get_stderr(&mut self) -> Result<Resource<streams::OutputStream>, anyhow::Error> {
        let stream = self.ctx_mut().stderr.stream();
        Ok(self.table_mut().push(stream)?)
    }
}

pub struct TerminalInput;
pub struct TerminalOutput;

impl<T: WasiView> terminal_input::Host for T {}
impl<T: WasiView> terminal_input::HostTerminalInput for T {
    fn drop(&mut self, r: Resource<TerminalInput>) -> anyhow::Result<()> {
        self.table_mut().delete(r)?;
        Ok(())
    }
}
impl<T: WasiView> terminal_output::Host for T {}
impl<T: WasiView> terminal_output::HostTerminalOutput for T {
    fn drop(&mut self, r: Resource<TerminalOutput>) -> anyhow::Result<()> {
        self.table_mut().delete(r)?;
        Ok(())
    }
}
impl<T: WasiView> terminal_stdin::Host for T {
    fn get_terminal_stdin(&mut self) -> anyhow::Result<Option<Resource<TerminalInput>>> {
        if self.ctx().stdin.isatty() {
            let fd = self.table_mut().push(TerminalInput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}
impl<T: WasiView> terminal_stdout::Host for T {
    fn get_terminal_stdout(&mut self) -> anyhow::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx().stdout.isatty() {
            let fd = self.table_mut().push(TerminalOutput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}
impl<T: WasiView> terminal_stderr::Host for T {
    fn get_terminal_stderr(&mut self) -> anyhow::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx().stderr.isatty() {
            let fd = self.table_mut().push(TerminalOutput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}

#[cfg(all(unix, test))]
mod test {
    use crate::preview2::HostInputStream;
    use libc;
    use std::fs::File;
    use std::io::{BufRead, BufReader, Write};
    use std::os::fd::FromRawFd;

    fn test_child_stdin<T, P>(child: T, parent: P)
    where
        T: FnOnce(File),
        P: FnOnce(File, BufReader<File>),
    {
        unsafe {
            // Make pipe for emulating stdin.
            let mut stdin_fds: [libc::c_int; 2] = [0; 2];
            assert_eq!(
                libc::pipe(stdin_fds.as_mut_ptr()),
                0,
                "Failed to create stdin pipe"
            );
            let [stdin_read, stdin_write] = stdin_fds;

            // Make pipe for getting results.
            let mut result_fds: [libc::c_int; 2] = [0; 2];
            assert_eq!(
                libc::pipe(result_fds.as_mut_ptr()),
                0,
                "Failed to create result pipe"
            );
            let [result_read, result_write] = result_fds;

            let child_pid = libc::fork();
            if child_pid == 0 {
                libc::close(stdin_write);
                libc::close(result_read);

                libc::close(libc::STDIN_FILENO);
                libc::dup2(stdin_read, libc::STDIN_FILENO);

                let result_write = File::from_raw_fd(result_write);
                child(result_write);
            } else {
                libc::close(stdin_read);
                libc::close(result_write);

                let stdin_write = File::from_raw_fd(stdin_write);
                let result_read = BufReader::new(File::from_raw_fd(result_read));
                parent(stdin_write, result_read);
            }
        }
    }

    // This could even be parameterized somehow to use the worker thread stdin vs the asyncfd
    // stdin.
    fn test_stdin_by_forking<S, T>(mk_stdin: T)
    where
        S: HostInputStream,
        T: Fn() -> S,
    {
        test_child_stdin(
            |mut result_write| {
                let mut child_running = true;
                while child_running {
                    tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(async {
                            'task: loop {
                                println!("child: creating stdin");
                                let mut stdin = mk_stdin();

                                println!("child: checking that stdin is not ready");
                                assert!(
                                    tokio::time::timeout(
                                        std::time::Duration::from_millis(100),
                                        stdin.ready()
                                    )
                                    .await
                                    .is_err(),
                                    "stdin available too soon"
                                );

                                writeln!(&mut result_write, "start").unwrap();

                                println!("child: started");

                                let mut buffer = String::new();
                                loop {
                                    println!("child: waiting for stdin to be ready");
                                    stdin.ready().await;

                                    println!("child: reading input");
                                    // We can't effectively test for the case where stdin was closed, so panic if it is...
                                    let bytes = stdin.read(1024).unwrap();

                                    println!("child got: {:?}", bytes);

                                    buffer.push_str(std::str::from_utf8(bytes.as_ref()).unwrap());
                                    if let Some((line, rest)) = buffer.split_once('\n') {
                                        if line == "all done" {
                                            writeln!(&mut result_write, "done").unwrap();
                                            println!("child: exiting...");
                                            child_running = false;
                                            break 'task;
                                        } else if line == "restart_runtime" {
                                            writeln!(&mut result_write, "restarting").unwrap();
                                            println!("child: restarting runtime...");
                                            break 'task;
                                        } else if line == "restart_task" {
                                            writeln!(&mut result_write, "restarting").unwrap();
                                            println!("child: restarting task...");
                                            continue 'task;
                                        } else {
                                            writeln!(&mut result_write, "{}", line).unwrap();
                                        }

                                        buffer = rest.to_owned();
                                    }
                                }
                            }
                        });
                    println!("runtime exited");
                }
                println!("child exited");
            },
            |mut stdin_write, mut result_read| {
                let mut line = String::new();
                result_read.read_line(&mut line).unwrap();
                assert_eq!(line, "start\n");

                for i in 0..5 {
                    let message = format!("some bytes {}\n", i);
                    stdin_write.write_all(message.as_bytes()).unwrap();
                    line.clear();
                    result_read.read_line(&mut line).unwrap();
                    assert_eq!(line, message);
                }

                writeln!(&mut stdin_write, "restart_task").unwrap();
                line.clear();
                result_read.read_line(&mut line).unwrap();
                assert_eq!(line, "restarting\n");
                line.clear();

                result_read.read_line(&mut line).unwrap();
                assert_eq!(line, "start\n");

                for i in 0..10 {
                    let message = format!("more bytes {}\n", i);
                    stdin_write.write_all(message.as_bytes()).unwrap();
                    line.clear();
                    result_read.read_line(&mut line).unwrap();
                    assert_eq!(line, message);
                }

                writeln!(&mut stdin_write, "restart_runtime").unwrap();
                line.clear();
                result_read.read_line(&mut line).unwrap();
                assert_eq!(line, "restarting\n");
                line.clear();

                result_read.read_line(&mut line).unwrap();
                assert_eq!(line, "start\n");

                for i in 0..17 {
                    let message = format!("even more bytes {}\n", i);
                    stdin_write.write_all(message.as_bytes()).unwrap();
                    line.clear();
                    result_read.read_line(&mut line).unwrap();
                    assert_eq!(line, message);
                }

                writeln!(&mut stdin_write, "all done").unwrap();

                line.clear();
                result_read.read_line(&mut line).unwrap();
                assert_eq!(line, "done\n");
            },
        )
    }

    // This test doesn't work under qemu because of the use of fork in the test helper.
    #[test]
    #[cfg_attr(not(target_arch = "x86_64"), ignore)]
    fn test_worker_thread_stdin() {
        test_stdin_by_forking(super::worker_thread_stdin::stdin);
    }
}

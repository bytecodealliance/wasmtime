use crate::preview2::pipe::AsyncWriteStream;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::{stdin, Stdin};

#[allow(dead_code)]
mod worker_thread_stdin;
#[cfg(windows)]
pub use self::worker_thread_stdin::{stdin, Stdin};

pub type Stdout = AsyncWriteStream;

pub fn stdout() -> Stdout {
    AsyncWriteStream::new(tokio::io::stdout())
}
pub type Stderr = AsyncWriteStream;

pub fn stderr() -> Stderr {
    AsyncWriteStream::new(tokio::io::stderr())
}

#[cfg(all(unix, test))]
mod test {
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
    #[test]
    fn test_stdin_by_forking() {
        test_child_stdin(
            |mut result_write| {
                //   in a tokio runtime:
                //     let stdin = super::stdin();
                //     // Make sure the initial state is that stdin is not ready:
                //     if timeout(stdin.ready().await).is_timeout() {
                //        send "start\n" on result pipe.
                //     }
                //     loop {
                //       match timeout(stdin.ready().await) {
                //         Ok => {
                //          let bytes = stdin.read();
                //          if bytes == ending sentinel:
                //            exit
                //          if bytes == some other sentinel:
                //            return and go back to the thing where we start the tokio runtime,
                //            testing that when creating a new super::stdin() it works correctly
                //          send "got: {bytes:?}\n" on result pipe.
                //         }
                //         Err => {
                //          send "timed out\n" on result pipe.
                //         }
                //       }
                //     }

                tokio::runtime::Runtime::new().unwrap().block_on(async {
                    use tokio::io::AsyncReadExt;

                    let mut stdin = tokio::io::stdin();

                    let mut buf = [0u8; 1024];
                    {
                        let r = tokio::time::timeout(
                            std::time::Duration::from_millis(100),
                            stdin.read(&mut buf[..1]),
                        )
                        .await;
                        assert!(r.is_err(), "stdin available too soon");
                    }

                    writeln!(&mut result_write, "start").unwrap();
                });
            },
            |mut stdin_write, mut result_read| {
                //   wait to recv "start\n" on result pipe (or the child process exits)
                let mut line = String::new();
                result_read.read_line(&mut line).unwrap();
                assert_eq!(line, "start\n");

                //   send some bytes to child stdin.
                //   make sure we get back "got {bytes:?}" on result pipe (or the child process exits)
                //   sleep for a while.
                //   make sure we get back "timed out" on result pipe (or the child process exits)
                //   send some bytes again. and etc.
            },
        )
    }
}

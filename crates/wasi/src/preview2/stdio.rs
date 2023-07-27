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
    use crate::preview2::{HostInputStream, StreamState};
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
                                    stdin.ready().await.unwrap();

                                    println!("child: reading input");
                                    let (bytes, status) = stdin.read(1024).unwrap();

                                    println!("child: {:?}, {:?}", bytes, status);

                                    // We can't effectively test for the case where stdin was closed.
                                    assert_eq!(status, StreamState::Open);

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

    #[test]
    fn test_async_fd_stdin() {
        test_stdin_by_forking(super::stdin);
    }

    #[test]
    fn test_worker_thread_stdin() {
        test_stdin_by_forking(super::worker_thread_stdin::stdin);
    }
}

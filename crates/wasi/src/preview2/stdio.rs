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
    // This could even be parameterized somehow to use the worker thread stdin vs the asyncfd
    // stdin.
    #[test]
    fn test_stdin_by_forking() {
        // Make pipe for emulating stdin.
        // Make pipe for getting results.
        // Fork.
        // When child:
        //   close stdin fd.
        //   use dup2 to turn the pipe recv end into the stdin fd.
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
        // When parent:
        //   wait to recv "start\n" on result pipe (or the child process exits)
        //   send some bytes to child stdin.
        //   make sure we get back "got {bytes:?}" on result pipe (or the child process exits)
        //   sleep for a while.
        //   make sure we get back "timed out" on result pipe (or the child process exits)
        //   send some bytes again. and etc.
        //
    }
}

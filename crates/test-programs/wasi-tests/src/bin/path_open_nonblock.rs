use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn try_path_open(dir_fd: wasi::Fd) {
    let _fd =
        wasi::path_open(dir_fd, 0, ".", 0, 0, 0, wasi::FDFLAGS_NONBLOCK).expect("opening the dir");
}

fn main() {
    let mut args = env::args();
    let prog = args.next().unwrap();
    let arg = if let Some(arg) = args.next() {
        arg
    } else {
        eprintln!("usage: {} <scratch directory>", prog);
        process::exit(1);
    };

    // Open scratch directory
    let dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe { try_path_open(dir_fd) }
}

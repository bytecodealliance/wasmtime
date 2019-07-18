mod fs;
mod misc;
mod sock;

pub use self::fs::*;
pub use self::misc::*;
pub use self::sock::*;

use crate::{host, memory, wasm32};

fn return_enc_errno(errno: host::__wasi_errno_t) -> wasm32::__wasi_errno_t {
    let errno = memory::enc_errno(errno);
    log::trace!("    -> errno={}", wasm32::strerror(errno));
    errno
}

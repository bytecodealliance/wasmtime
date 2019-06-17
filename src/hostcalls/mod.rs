mod fs;
mod misc;
mod sock;

pub use self::fs::*;
pub use self::misc::*;
pub use self::sock::*;

fn return_enc_errno(errno: super::host::__wasi_errno_t) -> super::wasm32::__wasi_errno_t {
    let errno = super::memory::enc_errno(errno);
    log::trace!("    -> errno={}", super::wasm32::strerror(errno));
    errno
}

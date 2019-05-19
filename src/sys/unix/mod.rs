pub mod host;
pub mod hostcalls;
pub mod fdmap;

pub mod memory {
    use crate::{host, wasm32};
    use crate::memory::*;

    #[cfg(target_os = "linux")]
    pub fn dirent_from_host(
        host_entry: &nix::libc::dirent,
    ) -> Result<wasm32::__wasi_dirent_t, host::__wasi_errno_t> {
        let mut entry = unsafe { std::mem::zeroed::<wasm32::__wasi_dirent_t>() };
        let d_namlen = unsafe { std::ffi::CStr::from_ptr(host_entry.d_name.as_ptr()) }
            .to_bytes()
            .len();
        if d_namlen > u32::max_value() as usize {
            return Err(host::__WASI_EIO);
        }
        entry.d_ino = enc_inode(host_entry.d_ino);
        entry.d_next = enc_dircookie(host_entry.d_off as u64);
        entry.d_namlen = enc_u32(d_namlen as u32);
        entry.d_type = enc_filetype(host_entry.d_type);
        Ok(entry)
    }

    #[cfg(not(target_os = "linux"))]
    pub fn dirent_from_host(
        host_entry: &nix::libc::dirent,
    ) -> Result<wasm32::__wasi_dirent_t, host::__wasi_errno_t> {
        let mut entry = unsafe { std::mem::zeroed::<wasm32::__wasi_dirent_t>() };
        entry.d_ino = enc_inode(host_entry.d_ino);
        entry.d_next = enc_dircookie(host_entry.d_seekoff);
        entry.d_namlen = enc_u32(u32::from(host_entry.d_namlen));
        entry.d_type = enc_filetype(host_entry.d_type);
        Ok(entry)
    }
}

pub fn dev_null() -> std::fs::File {
    std::fs::File::open("/dev/null").expect("failed to open /dev/null")
}

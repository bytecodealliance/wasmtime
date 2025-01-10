fn main() {
    #[link(wasm_import_module = "wasi_snapshot_preview1")]
    unsafe extern "C" {
        #[cfg_attr(target_arch = "wasm32", link_name = "adapter_open_badfd")]
        fn adapter_open_badfd(fd: *mut u32) -> wasi::Errno;

        #[cfg_attr(target_arch = "wasm32", link_name = "adapter_close_badfd")]
        fn adapter_close_badfd(fd: u32) -> wasi::Errno;
    }

    unsafe {
        let mut fd = 0;
        assert_eq!(adapter_open_badfd(&mut fd), wasi::ERRNO_SUCCESS);

        assert_eq!(wasi::fd_close(fd), Err(wasi::ERRNO_BADF));

        assert_eq!(wasi::fd_fdstat_get(fd).map(drop), Err(wasi::ERRNO_BADF));

        assert_eq!(wasi::fd_fdstat_set_rights(fd, 0, 0), Err(wasi::ERRNO_BADF));

        let mut buffer = [0_u8; 1];
        assert_eq!(
            wasi::fd_read(fd, &[wasi::Iovec {
                buf: buffer.as_mut_ptr(),
                buf_len: 1
            }]),
            Err(wasi::ERRNO_BADF)
        );

        assert_eq!(
            wasi::fd_write(fd, &[wasi::Ciovec {
                buf: buffer.as_ptr(),
                buf_len: 1
            }]),
            Err(wasi::ERRNO_BADF)
        );

        assert_eq!(adapter_close_badfd(fd), wasi::ERRNO_SUCCESS);
    }
}

fn main() {
    #[link(wasm_import_module = "wasi_snapshot_preview1")]
    unsafe extern "C" {
        #[cfg_attr(target_arch = "wasm32", link_name = "adapter_open_badfd")]
        fn adapter_open_badfd(fd: *mut u32) -> wasip1::Errno;

        #[cfg_attr(target_arch = "wasm32", link_name = "adapter_close_badfd")]
        fn adapter_close_badfd(fd: u32) -> wasip1::Errno;
    }

    unsafe {
        let mut fd = 0;
        assert_eq!(adapter_open_badfd(&mut fd), wasip1::ERRNO_SUCCESS);

        assert_eq!(wasip1::fd_close(fd), Err(wasip1::ERRNO_BADF));

        assert_eq!(wasip1::fd_fdstat_get(fd).map(drop), Err(wasip1::ERRNO_BADF));

        assert_eq!(
            wasip1::fd_fdstat_set_rights(fd, 0, 0),
            Err(wasip1::ERRNO_BADF)
        );

        let mut buffer = [0_u8; 1];
        assert_eq!(
            wasip1::fd_read(
                fd,
                &[wasip1::Iovec {
                    buf: buffer.as_mut_ptr(),
                    buf_len: 1
                }]
            ),
            Err(wasip1::ERRNO_BADF)
        );

        assert_eq!(
            wasip1::fd_write(
                fd,
                &[wasip1::Ciovec {
                    buf: buffer.as_ptr(),
                    buf_len: 1
                }]
            ),
            Err(wasip1::ERRNO_BADF)
        );

        assert_eq!(adapter_close_badfd(fd), wasip1::ERRNO_SUCCESS);
    }
}

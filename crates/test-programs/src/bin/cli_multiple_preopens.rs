use std::str;

fn main() {
    dbg!(wasip2::filesystem::preopens::get_directories());
    unsafe {
        let p3 = wasi::fd_prestat_get(3).unwrap();
        let p4 = wasi::fd_prestat_get(4).unwrap();
        let p5 = wasi::fd_prestat_get(5).unwrap();
        assert_eq!(wasi::fd_prestat_get(6).err().unwrap(), wasi::ERRNO_BADF);

        assert_eq!(p3.u.dir.pr_name_len, 2);
        assert_eq!(p4.u.dir.pr_name_len, 2);
        assert_eq!(p5.u.dir.pr_name_len, 2);

        let mut buf = [0; 100];

        wasi::fd_prestat_dir_name(3, buf.as_mut_ptr(), buf.len()).unwrap();
        assert_eq!(str::from_utf8(&buf[..2]).unwrap(), "/a");
        wasi::fd_prestat_dir_name(4, buf.as_mut_ptr(), buf.len()).unwrap();
        assert_eq!(str::from_utf8(&buf[..2]).unwrap(), "/b");
        wasi::fd_prestat_dir_name(5, buf.as_mut_ptr(), buf.len()).unwrap();
        assert_eq!(str::from_utf8(&buf[..2]).unwrap(), "/c");
        assert_eq!(
            wasi::fd_prestat_dir_name(6, buf.as_mut_ptr(), buf.len()),
            Err(wasi::ERRNO_BADF),
        );
    }
    // ..
}

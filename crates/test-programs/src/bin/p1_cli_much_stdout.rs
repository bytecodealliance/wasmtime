fn main() {
    let mut args = std::env::args().skip(1);
    let string_to_write = args.next().unwrap();
    let times_to_write = args.next().unwrap().parse::<u32>().unwrap();

    let bytes = string_to_write.as_bytes();
    for _ in 0..times_to_write {
        let mut remaining = bytes;
        while !remaining.is_empty() {
            let iovec = wasip1::Ciovec {
                buf: remaining.as_ptr(),
                buf_len: remaining.len(),
            };
            let amt = unsafe { wasip1::fd_write(1, &[iovec]).unwrap() };
            remaining = &remaining[amt..];
        }
    }
}

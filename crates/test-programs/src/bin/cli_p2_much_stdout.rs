fn main() {
    let mut args = std::env::args().skip(1);
    let string_to_write = args.next().unwrap();
    let times_to_write = args.next().unwrap().parse::<u32>().unwrap();

    let bytes = string_to_write.as_bytes();
    let stdout = wasip2::cli::stdout::get_stdout();
    for _ in 0..times_to_write {
        for chunk in bytes.chunks(4096) {
            stdout.blocking_write_and_flush(chunk).unwrap();
        }
    }
}

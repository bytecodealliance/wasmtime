fn main() {
    let mut args = std::env::args().skip(1);
    let amt_to_write = args.next().unwrap().parse().unwrap();
    let stdout = wasip2::cli::stdout::get_stdout();
    stdout.write_zeroes(amt_to_write).unwrap();
}

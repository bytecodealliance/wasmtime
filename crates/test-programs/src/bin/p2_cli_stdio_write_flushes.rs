use std::io::Write;

fn main() {
    print!("> ");
    std::io::stdout().flush().unwrap();

    let mut s = String::new();
    std::io::stdin().read_line(&mut s).unwrap();
    assert!(s.is_empty());
}

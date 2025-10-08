use std::io;

fn main() {
    assert_eq!(
        "So rested he by the Tumtum tree",
        &io::read_to_string(io::stdin().lock()).unwrap()
    );
}

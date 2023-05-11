use std::env;

fn main() {
    assert_eq!(env::args().collect::<Vec<_>>(), ["program", "/foo"]);
    assert_eq!(env::var("TEST").as_deref(), Ok("1"));
}

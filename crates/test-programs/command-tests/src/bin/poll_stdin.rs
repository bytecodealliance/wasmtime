/* FIXME once rustix can build on stable rust, rewrite
use rustix::io::{PollFd, PollFlags};

fn main() {
    let stdin = std::io::stdin();
    let mut pollfds = vec![PollFd::new(&stdin, PollFlags::IN)];
    let num = rustix::io::poll(&mut pollfds, -1).unwrap();
    assert_eq!(num, 1);
    assert_eq!(pollfds[0].revents(), PollFlags::IN);
}
*/
fn main() {
    println!("FIXME: this test does nothing for now");
}

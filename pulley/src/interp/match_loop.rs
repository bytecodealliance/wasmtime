use super::*;
use crate::decode::unwrap_uninhabited;

impl Interpreter<'_> {
    pub fn run(mut self) -> Done {
        let mut decoder = Decoder::new();
        loop {
            match unwrap_uninhabited(decoder.decode_one(&mut self)) {
                ControlFlow::Continue(()) => {}
                ControlFlow::Break(done) => break done,
            }
        }
    }
}

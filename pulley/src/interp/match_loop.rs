//! Implementation of the interpreter loop for Pulley with a simple `match`
//! statement.
//!
//! This module is notably in contrast to the `tail_loop.rs` which implements
//! the interpreter loop with tail calls. It's predicted that tail calls are a
//! more performant solution but that's also not available on stable Rust today,
//! so this module instead compiles on stable Rust.
//!
//! This interpreter loop is a simple `loop` with a "moral `match`" despite not
//! actually having one here. The `Decoder` API is used to dispatch to the
//! `OpVisitor` trait implementation on `Interpreter<'_>`. The literal `match`
//! is embedded within the `Decoder::decode_one` function.
//!
//! Note that as of the time of this writing there hasn't been much performance
//! analysis of this loop just yet. It's probably too simple to compile well and
//! will probably need tweaks to make it more performant.

use super::*;
use crate::decode::unwrap_uninhabited;

impl Interpreter<'_> {
    pub fn run(mut self) -> Done {
        let mut decoder = Decoder::new();
        loop {
            // Here `decode_one` will call the appropriate `OpVisitor` method on
            // `self` via the trait implementation in the module above this.
            // That'll return whether we should keep going or exit the loop,
            // which is then done here with a conditional `break`.
            //
            // This will then continue indefinitely until the bytecode says it's
            // done. Note that only trusted bytecode is interpreted here.
            match unwrap_uninhabited(decoder.decode_one(&mut self)) {
                ControlFlow::Continue(()) => {}
                ControlFlow::Break(done) => break done,
            }
        }
    }
}

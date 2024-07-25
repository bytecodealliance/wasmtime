#![no_main]

use libfuzzer_sys::fuzz_target;
use pulley_interpreter::{
    decode::{Decoder, SafeBytecodeStream},
    op::{MaterializeOpsVisitor, Op},
};

fuzz_target!(|ops: Vec<Op>| {
    let _ = env_logger::try_init();

    log::trace!("input: {ops:#?}");

    let mut encoded = vec![];
    for op in &ops {
        op.encode(&mut encoded);
    }
    log::trace!("encoded: {encoded:?}");

    let mut materializer = MaterializeOpsVisitor::new(SafeBytecodeStream::new(&encoded));
    let decoded = Decoder::decode_all(&mut materializer).expect("should decode okay");
    log::trace!("decoded: {decoded:#?}");

    assert_eq!(
        decoded, ops,
        "`decode(encode(ops))` should be equal to the original `ops`"
    );
});

use pulley_interpreter::{
    decode::{Decoder, SafeBytecodeStream},
    op::{MaterializeOpsVisitor, Op},
};

pub fn roundtrip(ops: Vec<Op>) {
    let _ = env_logger::try_init();

    log::trace!("input: {ops:#?}");

    let mut encoded = vec![];
    for op in &ops {
        let before = encoded.len();
        op.encode(&mut encoded);
        let size = encoded.len() - before;
        assert_eq!(size, op.width().into());
    }
    log::trace!("encoded: {encoded:?}");

    let mut materializer = MaterializeOpsVisitor::new(SafeBytecodeStream::new(&encoded));
    let decoded = Decoder::decode_all(&mut materializer).expect("should decode okay");
    log::trace!("decoded: {decoded:#?}");

    assert_eq!(
        decoded, ops,
        "`decode(encode(ops))` should be equal to the original `ops`"
    );
}

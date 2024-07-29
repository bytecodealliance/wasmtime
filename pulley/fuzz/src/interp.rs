use pulley_interpreter::{
    interp::Vm,
    op::{self, ExtendedOp, Op},
    *,
};
use std::ptr::NonNull;

pub fn interp(ops: Vec<Op>) {
    let _ = env_logger::try_init();

    log::trace!("input: {ops:#?}");

    let mut ops = ops;
    ops.retain(|op| op_is_safe_for_fuzzing(op));
    // Make sure that we end with a `ret` so that the interpreter returns
    // control to us instead of continuing off the end of the ops and into
    // undefined memory.
    ops.push(Op::Ret(op::Ret {}));

    log::trace!("filtered to only safe ops: {ops:#?}");

    let mut encoded = vec![];
    for op in &ops {
        op.encode(&mut encoded);
    }
    log::trace!("encoded: {encoded:?}");

    let mut vm = Vm::new();
    unsafe {
        let args = &[];
        let rets = &[];
        match vm.call(NonNull::from(&encoded[0]), args, rets.into_iter().copied()) {
            Ok(rets) => assert_eq!(rets.count(), 0),
            Err(pc) => {
                let pc = pc as usize;

                let start = &encoded[0] as *const u8 as usize;
                let end = encoded.last().unwrap() as *const u8 as usize;
                assert!(
                    start <= pc && pc < end,
                    "pc should be in range {start:#018x}..{end:#018x}, got {pc:#018x}"
                );

                let index = pc - start;
                assert_eq!(encoded[index], Opcode::ExtendedOp as u8);
                let [a, b] = (ExtendedOpcode::Trap as u16).to_le_bytes();
                assert_eq!(encoded[index + 1], a);
                assert_eq!(encoded[index + 2], b);
            }
        };
    }
}

fn op_is_safe_for_fuzzing(op: &Op) -> bool {
    match op {
        Op::Ret(_) => true,
        Op::Jump(_) => false,
        Op::BrIf(_) => false,
        Op::BrIfNot(_) => false,
        Op::BrIfXeq32(_) => false,
        Op::BrIfXneq32(_) => false,
        Op::BrIfXult32(_) => false,
        Op::BrIfXulteq32(_) => false,
        Op::BrIfXslt32(_) => false,
        Op::BrIfXslteq32(_) => false,
        Op::Xmov(op::Xmov { dst, .. }) => !dst.is_special(),
        Op::Fmov(_) => true,
        Op::Vmov(_) => true,
        Op::Xconst8(op::Xconst8 { dst, .. }) => !dst.is_special(),
        Op::Xconst16(op::Xconst16 { dst, .. }) => !dst.is_special(),
        Op::Xconst32(op::Xconst32 { dst, .. }) => !dst.is_special(),
        Op::Xconst64(op::Xconst64 { dst, .. }) => !dst.is_special(),
        Op::Xadd32(op::Xadd32 { dst, .. }) => !dst.is_special(),
        Op::Xadd64(op::Xadd64 { dst, .. }) => !dst.is_special(),
        Op::Load32U(_) => false,
        Op::Load32S(_) => false,
        Op::Load64(_) => false,
        Op::Load32UOffset8(_) => false,
        Op::Load32SOffset8(_) => false,
        Op::Load64Offset8(_) => false,
        Op::Store32(_) => false,
        Op::Store64(_) => false,
        Op::Store32SOffset8(_) => false,
        Op::Store64Offset8(_) => false,
        Op::BitcastIntFromFloat32(op::BitcastIntFromFloat32 { dst, .. }) => !dst.is_special(),
        Op::BitcastIntFromFloat64(op::BitcastIntFromFloat64 { dst, .. }) => !dst.is_special(),
        Op::BitcastFloatFromInt32(_) => true,
        Op::BitcastFloatFromInt64(_) => true,
        Op::ExtendedOp(op) => extended_op_is_safe_for_fuzzing(op),
        Op::Call(_) => false,
        Op::Xeq64(Xeq64 { dst, .. }) => !dst.is_special(),
        Op::Xneq64(Xneq64 { dst, .. }) => !dst.is_special(),
        Op::Xslt64(Xslt64 { dst, .. }) => !dst.is_special(),
        Op::Xslteq64(Xslteq64 { dst, .. }) => !dst.is_special(),
        Op::Xult64(Xult64 { dst, .. }) => !dst.is_special(),
        Op::Xulteq64(Xulteq64 { dst, .. }) => !dst.is_special(),
        Op::Xeq32(Xeq32 { dst, .. }) => !dst.is_special(),
        Op::Xneq32(Xneq32 { dst, .. }) => !dst.is_special(),
        Op::Xslt32(Xslt32 { dst, .. }) => !dst.is_special(),
        Op::Xslteq32(Xslteq32 { dst, .. }) => !dst.is_special(),
        Op::Xult32(Xult32 { dst, .. }) => !dst.is_special(),
        Op::Xulteq32(Xulteq32 { dst, .. }) => !dst.is_special(),
    }
}

fn extended_op_is_safe_for_fuzzing(op: &ExtendedOp) -> bool {
    match op {
        ExtendedOp::Trap(_) => true,
        ExtendedOp::Nop(_) => true,
        ExtendedOp::GetSp(GetSp { dst, .. }) => !dst.is_special(),
    }
}

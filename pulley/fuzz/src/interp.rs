use pulley_interpreter::{
    interp::{DoneReason, Vm},
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
            DoneReason::ReturnToHost(rets) => assert_eq!(rets.count(), 0),
            DoneReason::Trap(pc) => {
                let pc = pc.as_ptr() as usize;

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
            DoneReason::CallIndirectHost { .. } => unreachable!(),
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
        Op::BrIfXeq64(_) => false,
        Op::BrIfXneq64(_) => false,
        Op::BrIfXult64(_) => false,
        Op::BrIfXulteq64(_) => false,
        Op::BrIfXslt64(_) => false,
        Op::BrIfXslteq64(_) => false,
        Op::Xmov(op::Xmov { dst, .. }) => !dst.is_special(),
        Op::Fmov(_) => true,
        Op::Vmov(_) => true,
        Op::Xconst8(op::Xconst8 { dst, .. }) => !dst.is_special(),
        Op::Xconst16(op::Xconst16 { dst, .. }) => !dst.is_special(),
        Op::Xconst32(op::Xconst32 { dst, .. }) => !dst.is_special(),
        Op::Xconst64(op::Xconst64 { dst, .. }) => !dst.is_special(),
        Op::Load32U(_) => false,
        Op::Load32S(_) => false,
        Op::Load64(_) => false,
        Op::Load32UOffset8(_) => false,
        Op::Load32SOffset8(_) => false,
        Op::Load32UOffset64(_) => false,
        Op::Load32SOffset64(_) => false,
        Op::Load64Offset8(_) => false,
        Op::Load64Offset64(_) => false,
        Op::Store32(_) => false,
        Op::Store64(_) => false,
        Op::Store32SOffset8(_) => false,
        Op::Store32SOffset64(_) => false,
        Op::Store64Offset8(_) => false,
        Op::Store64Offset64(_) => false,
        Op::BitcastIntFromFloat32(op::BitcastIntFromFloat32 { dst, .. }) => !dst.is_special(),
        Op::BitcastIntFromFloat64(op::BitcastIntFromFloat64 { dst, .. }) => !dst.is_special(),
        Op::BitcastFloatFromInt32(_) => true,
        Op::BitcastFloatFromInt64(_) => true,
        Op::ExtendedOp(op) => extended_op_is_safe_for_fuzzing(op),
        Op::Call(_) => false,
        Op::CallIndirect(_) => false,
        Op::Xadd32(Xadd32 { operands, .. })
        | Op::Xadd64(Xadd64 { operands, .. })
        | Op::Xeq64(Xeq64 { operands, .. })
        | Op::Xneq64(Xneq64 { operands, .. })
        | Op::Xslt64(Xslt64 { operands, .. })
        | Op::Xslteq64(Xslteq64 { operands, .. })
        | Op::Xult64(Xult64 { operands, .. })
        | Op::Xulteq64(Xulteq64 { operands, .. })
        | Op::Xeq32(Xeq32 { operands, .. })
        | Op::Xneq32(Xneq32 { operands, .. })
        | Op::Xslt32(Xslt32 { operands, .. })
        | Op::Xslteq32(Xslteq32 { operands, .. })
        | Op::Xult32(Xult32 { operands, .. })
        | Op::Xulteq32(Xulteq32 { operands, .. }) => !operands.dst.is_special(),
        Op::PushFrame(_) | Op::PopFrame(_) => false,
        Op::XPush32(_) | Op::XPush64(_) => false,
        Op::XPop32(_) | Op::XPop64(_) => false,
        Op::XPush32Many(_) | Op::XPush64Many(_) => false,
        Op::XPop32Many(_) | Op::XPop64Many(_) => false,
        Op::BrTable32(_) => false,
        Op::StackAlloc32(_) => false,
        Op::StackFree32(_) => false,
        Op::Zext8(Zext8 { dst, .. })
        | Op::Zext16(Zext16 { dst, .. })
        | Op::Zext32(Zext32 { dst, .. })
        | Op::Sext8(Sext8 { dst, .. })
        | Op::Sext32(Sext32 { dst, .. })
        | Op::Sext16(Sext16 { dst, .. }) => !dst.is_special(),
    }
}

fn extended_op_is_safe_for_fuzzing(op: &ExtendedOp) -> bool {
    match op {
        ExtendedOp::Trap(_) => true,
        ExtendedOp::Nop(_) => true,
        ExtendedOp::CallIndirectHost(_) => false,
        ExtendedOp::Bswap32(Bswap32 { dst, .. }) | ExtendedOp::Bswap64(Bswap64 { dst, .. }) => {
            !dst.is_special()
        }
    }
}

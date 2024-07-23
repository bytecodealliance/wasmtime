#![no_main]

use libfuzzer_sys::fuzz_target;
use pulley_interpreter::{
    interp::Vm,
    op::{self, ExtendedOp, Op},
    ExtendedOpcode, Opcode,
};
use std::ptr::NonNull;

fuzz_target!(|ops: Vec<Op>| {
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
                    "pc should be in range {start}..{end}, got {pc}"
                );

                let index = pc - start;
                assert_eq!(encoded[index], Opcode::ExtendedOp as u8);
                let [a, b] = (ExtendedOpcode::Trap as u16).to_le_bytes();
                assert_eq!(encoded[index + 1], a);
                assert_eq!(encoded[index + 2], b);
            }
        };
    }
});

fn op_is_safe_for_fuzzing(op: &Op) -> bool {
    match op {
        Op::Ret(_) => true,
        Op::Jump(_) => false,
        Op::BrIf(_) => false,
        Op::BrIfNot(_) => false,
        Op::Xmov(op::Xmov { dst, .. }) => !dst.is_special(),
        Op::Fmov(_) => true,
        Op::Vmov(_) => true,
        Op::Xconst8(op::Xconst8 { dst, .. }) => !dst.is_special(),
        Op::Xconst16(op::Xconst16 { dst, .. }) => !dst.is_special(),
        Op::Xconst32(op::Xconst32 { dst, .. }) => !dst.is_special(),
        Op::Xconst64(op::Xconst64 { dst, .. }) => !dst.is_special(),
        Op::Xadd32(op::Xadd32 { dst, .. }) => !dst.is_special(),
        Op::Xadd64(op::Xadd64 { dst, .. }) => !dst.is_special(),
        Op::LoadU32(_) => false,
        Op::LoadS32(_) => false,
        Op::Load64(_) => false,
        Op::Load32UOffset8(_) => false,
        Op::Load32SOffset8(_) => false,
        Op::Load64Offset8(_) => false,
        Op::Load32UOffset16(_) => false,
        Op::Load32SOffset16(_) => false,
        Op::Load64Offset16(_) => false,
        Op::Load32UOffset32(_) => false,
        Op::Load32SOffset32(_) => false,
        Op::Load64Offset32(_) => false,
        Op::Load32UOffset64(_) => false,
        Op::Load32SOffset64(_) => false,
        Op::Load64Offset64(_) => false,
        Op::StoreS32(_) => false,
        Op::Store64(_) => false,
        Op::Store32SOffset8(_) => false,
        Op::Store64Offset8(_) => false,
        Op::Store32Offset16(_) => false,
        Op::Store64Offset16(_) => false,
        Op::Store32Offset32(_) => false,
        Op::Store64Offset32(_) => false,
        Op::Store32Offset64(_) => false,
        Op::Store64Offset64(_) => false,
        Op::BitcastIntFromFloat32(op::BitcastIntFromFloat32 { dst, .. }) => !dst.is_special(),
        Op::BitcastIntFromFloat64(op::BitcastIntFromFloat64 { dst, .. }) => !dst.is_special(),
        Op::BitcastFloatFromInt32(_) => true,
        Op::BitcastFloatFromInt64(_) => true,
        Op::ExtendedOp(op) => extended_op_is_safe_for_fuzzing(op),
    }
}

fn extended_op_is_safe_for_fuzzing(op: &ExtendedOp) -> bool {
    match op {
        ExtendedOp::Trap(_) => true,
        ExtendedOp::Nop(_) => true,
    }
}

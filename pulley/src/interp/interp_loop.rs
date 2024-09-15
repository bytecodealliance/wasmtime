use super::*;

#[derive(Copy, Clone)]
pub struct OpcodeHandler {
    /// The type of non tail-recursive opcode handlers: return
    /// `ControlFlow::Continue` with the next handler to call, or
    /// `ControlFlow::Done` with the reason to stop.
    #[cfg(not(pulley_tail_calls))]
    fun: fn(&mut MachineState, &mut UnsafeBytecodeStream) -> ControlFlow<Done, OpcodeHandler>,

    /// The type of tail-recursive opcode handlers: instead of returning
    /// `ControwFlow::Continue`, tail call the next handler directly; so
    /// `ControlFlow::Continue` is uninhabited.
    #[cfg(pulley_tail_calls)]
    fun: fn(&mut MachineState, &mut UnsafeBytecodeStream) -> Done,
}

#[cfg(not(pulley_tail_calls))]
pub fn interpreter_loop(vm: &mut Vm, bytecode: &mut UnsafeBytecodeStream) -> Done {
    let opcode = Opcode::decode(bytecode).unwrap();
    let mut handler = OPCODE_HANDLER_TABLE[opcode as usize];

    // As tight as we can get the interpreter loop without tail calls: while
    // the handlers keep returning the next handler to call, call it.
    loop {
        match (handler.fun)(&mut vm.state, bytecode) {
            ControlFlow::Continue(next_handler) => handler = next_handler,
            ControlFlow::Break(done) => return done,
        }
    }
}

/// The extra indirection through a macro is necessary to avoid a compiler error
/// when compiling without `#![feature(explicit_tail_calls)]` enabled (via
/// `--cfg pulley_tail_calls`).
///
/// It seems rustc first parses the the function, encounters `become` and emits
/// an error about using an unstable keyword on a stable compiler, then applies
/// `#[cfg(...)` after parsing to disable the function.
///
/// Macro bodies are just bags of tokens; the body is not parsed until after
/// they are expanded, and this macro is only expanded when `pulley_tail_calls`
/// is enabled.
#[cfg_attr(not(pulley_tail_calls), allow(unused_macros))]
macro_rules! tail_call {
    ($e:expr) => {
        become $e
    };
}

#[cfg(pulley_tail_calls)]
pub fn interpreter_loop(vm: &mut Vm, bytecode: &mut UnsafeBytecodeStream) -> Done {
    let opcode = Opcode::decode(bytecode).unwrap();
    let handler = OPCODE_HANDLER_TABLE[opcode as usize];

    // The ideal interpreter loop: a bunch of opcode handlers tail calling
    // each other!
    tail_call!((handler.fun)(&mut vm.state, bytecode));
}

/// Wrap the business logic of each handler with the boilerplate of decoding
/// operands and dispatching to next handler/exiting the loop.
macro_rules! define_opcode_handlers {
    (
    $(
        fn $name:ident (
            $state:ident : &mut MachineState,
            $bytecode:ident : &mut UnsafeBytecodeStream$(,)?
            $($field:ident : $field_ty:ty),*
        ) $body:block
    )*
    ) => {
        $(
            #[cfg(not(pulley_tail_calls))]
            pub fn $name($state: &mut MachineState, $bytecode: &mut UnsafeBytecodeStream) -> ControlFlow<Done, OpcodeHandler> {
                let ($($field,)*) = crate::decode::unwrap_uninhabited(crate::decode::operands::$name($bytecode));
                match $body {
                    ControlFlow::Continue(()) => {
                        // Decode the next handler and return it so that `run`
                        // can call it.
                        let next_opcode = Opcode::decode($bytecode).unwrap();
                        let next_handler = OPCODE_HANDLER_TABLE[next_opcode as usize];
                        ControlFlow::Continue(next_handler)
                    }
                    ControlFlow::Break(done) => ControlFlow::Break(done),
                }
            }

            #[cfg(pulley_tail_calls)]
            pub fn $name($state: &mut MachineState, $bytecode: &mut UnsafeBytecodeStream) -> Done {
                let ($($field,)*) = crate::decode::unwrap_uninhabited(crate::decode::operands::$name($bytecode));
                match $body {
                    ControlFlow::Continue(()) => {
                        // Decode the next handler and return it so that `run`
                        // can call it.
                        let next_opcode = Opcode::decode($bytecode).unwrap();
                        let next_handler = OPCODE_HANDLER_TABLE[next_opcode as usize];
                        tail_call!((next_handler.fun)($state, $bytecode));
                    }
                    ControlFlow::Break(done) => done,
                }
            }
        )*
    };
}

/// Define the table of opcode handlers.
macro_rules! opcode_handler_table_entry {
    (
        $(
            $( #[$attr:meta] )*
                $snake_name:ident = $name:ident $( {
                $(
                    $( #[$field_attr:meta] )*
                    $field:ident : $field_ty:ty
                ),*
            } )? ;
        )*
    ) => {[ $(OpcodeHandler { fun: $snake_name },)* OpcodeHandler { fun: extended }]};
}

/// Add one to account for `ExtendedOp`.
const NUM_OPCODES: usize = Opcode::MAX as usize + 1;
static OPCODE_HANDLER_TABLE: [OpcodeHandler; NUM_OPCODES] =
    for_each_op!(opcode_handler_table_entry);

#[inline]
fn pc_rel_jump(pc: &mut UnsafeBytecodeStream, offset: PcRelOffset, inst_size: isize) {
    let offset = isize::try_from(i32::from(offset)).unwrap();
    *pc = unsafe { pc.offset(offset - inst_size) };
}

define_opcode_handlers! {
    fn ret(state: &mut MachineState, pc: &mut UnsafeBytecodeStream) {
        if state[XReg::lr] == XRegVal::HOST_RETURN_ADDR {
            ControlFlow::Break(Done::ReturnToHost)
        } else {
            let return_addr = state[XReg::lr].get_ptr();
            *pc = unsafe { UnsafeBytecodeStream::new(NonNull::new_unchecked(return_addr)) };
            ControlFlow::Continue(())
        }
    }

    fn call(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, offset: PcRelOffset) {
        let return_addr = pc.as_ptr();
        state[XReg::lr].set_ptr(return_addr.as_ptr());
        pc_rel_jump(pc, offset, 5);
        ControlFlow::Continue(())
    }

    fn jump(_state: &mut MachineState, pc: &mut UnsafeBytecodeStream, offset: PcRelOffset) {
        pc_rel_jump(pc, offset, 5);
        ControlFlow::Continue(())
    }

    fn br_if(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, cond: XReg, offset: PcRelOffset) {
        let cond = state[cond].get_u64();
        if cond != 0 {
            pc_rel_jump(pc, offset, 6)
        }
        ControlFlow::Continue(())
    }

    fn br_if_not(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, cond: XReg, offset: PcRelOffset) {
        let cond = state[cond].get_u64();
        if cond == 0 {
            pc_rel_jump(pc, offset, 6)
        }
        ControlFlow::Continue(())
    }

    fn br_if_xeq32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, a: XReg, b: XReg, offset: PcRelOffset) {
        let a = state[a].get_u32();
        let b = state[b].get_u32();
        if a == b {
            pc_rel_jump(pc, offset, 7)
        }
        ControlFlow::Continue(())
    }

    fn br_if_xneq32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, a: XReg, b: XReg, offset: PcRelOffset) {
        let a = state[a].get_u32();
        let b = state[b].get_u32();
        if a != b {
            pc_rel_jump(pc, offset, 7)
        }
        ControlFlow::Continue(())
    }

    fn br_if_xslt32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, a: XReg, b: XReg, offset: PcRelOffset) {
        let a = state[a].get_i32();
        let b = state[b].get_i32();
        if a < b {
            pc_rel_jump(pc, offset, 7)
        }
        ControlFlow::Continue(())
    }

    fn br_if_xslteq32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, a: XReg, b: XReg, offset: PcRelOffset) {
        let a = state[a].get_i32();
        let b = state[b].get_i32();
        if a <= b {
            pc_rel_jump(pc, offset, 7)
        }
        ControlFlow::Continue(())
    }

    fn br_if_xult32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, a: XReg, b: XReg, offset: PcRelOffset) {
        let a = state[a].get_u32();
        let b = state[b].get_u32();
        if a < b {
            pc_rel_jump(pc, offset, 7)
        }
        ControlFlow::Continue(())
    }

    fn br_if_xulteq32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, a: XReg, b: XReg, offset: PcRelOffset) {
        let a = state[a].get_u32();
        let b = state[b].get_u32();
        if a <= b {
            pc_rel_jump(pc, offset, 7)
        }
        ControlFlow::Continue(())
    }

    fn br_if_xeq64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, a: XReg, b: XReg, offset: PcRelOffset) {
        let a = state[a].get_u64();
        let b = state[b].get_u64();
        if a == b {
            pc_rel_jump(pc, offset, 7)
        }
        ControlFlow::Continue(())
    }

    fn br_if_xneq64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, a: XReg, b: XReg, offset: PcRelOffset) {
        let a = state[a].get_u64();
        let b = state[b].get_u64();
        if a != b {
            pc_rel_jump(pc, offset, 7)
        }
        ControlFlow::Continue(())
    }

    fn br_if_xslt64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, a: XReg, b: XReg, offset: PcRelOffset) {
        let a = state[a].get_i64();
        let b = state[b].get_i64();
        if a < b {
            pc_rel_jump(pc, offset, 7)
        }
        ControlFlow::Continue(())
    }

    fn br_if_xslteq64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, a: XReg, b: XReg, offset: PcRelOffset) {
        let a = state[a].get_i64();
        let b = state[b].get_i64();
        if a <= b {
            pc_rel_jump(pc, offset, 7)
        }
        ControlFlow::Continue(())
    }

    fn br_if_xult64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, a: XReg, b: XReg, offset: PcRelOffset) {
        let a = state[a].get_u64();
        let b = state[b].get_u64();
        if a < b {
            pc_rel_jump(pc, offset, 7)
        }
        ControlFlow::Continue(())
    }

    fn br_if_xulteq64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, a: XReg, b: XReg, offset: PcRelOffset) {
        let a = state[a].get_u64();
        let b = state[b].get_u64();
        if a <= b {
            pc_rel_jump(pc, offset, 7)
        }
        ControlFlow::Continue(())
    }

    fn xmov(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, src: XReg) {
        let val = state[src];
        state[dst] = val;
        ControlFlow::Continue(())
    }

    fn fmov(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: FReg, src: FReg) {
        let val = state[src];
        state[dst] = val;
        ControlFlow::Continue(())
    }

    fn vmov(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: VReg, src: VReg) {
        let val = state[src];
        state[dst] = val;
        ControlFlow::Continue(())
    }

    fn xconst8(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, imm: i8) {
        state[dst].set_i64(i64::from(imm));
        ControlFlow::Continue(())
    }

    fn xconst16(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, imm: i16) {
        state[dst].set_i64(i64::from(imm));
        ControlFlow::Continue(())
    }

    fn xconst32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, imm: i32) {
        state[dst].set_i64(i64::from(imm));
        ControlFlow::Continue(())
    }

    fn xconst64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, imm: i64) {
        state[dst].set_i64(imm);
        ControlFlow::Continue(())
    }

    fn xadd32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_u32();
        let b = state[operands.src2].get_u32();
        state[operands.dst].set_u32(a.wrapping_add(b));
        ControlFlow::Continue(())
    }

    fn xadd64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_u64();
        let b = state[operands.src2].get_u64();
        state[operands.dst].set_u64(a.wrapping_add(b));
        ControlFlow::Continue(())
    }

    fn xeq64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_u64();
        let b = state[operands.src2].get_u64();
        state[operands.dst].set_u64(u64::from(a == b));
        ControlFlow::Continue(())
    }

    fn xneq64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_u64();
        let b = state[operands.src2].get_u64();
        state[operands.dst].set_u64(u64::from(a != b));
        ControlFlow::Continue(())
    }

    fn xslt64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_i64();
        let b = state[operands.src2].get_i64();
        state[operands.dst].set_u64(u64::from(a < b));
        ControlFlow::Continue(())
    }

    fn xslteq64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_i64();
        let b = state[operands.src2].get_i64();
        state[operands.dst].set_u64(u64::from(a <= b));
        ControlFlow::Continue(())
    }

    fn xult64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_u64();
        let b = state[operands.src2].get_u64();
        state[operands.dst].set_u64(u64::from(a < b));
        ControlFlow::Continue(())
    }

    fn xulteq64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_u64();
        let b = state[operands.src2].get_u64();
        state[operands.dst].set_u64(u64::from(a <= b));
        ControlFlow::Continue(())
    }

    fn xeq32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_u32();
        let b = state[operands.src2].get_u32();
        state[operands.dst].set_u64(u64::from(a == b));
        ControlFlow::Continue(())
    }

    fn xneq32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_u32();
        let b = state[operands.src2].get_u32();
        state[operands.dst].set_u64(u64::from(a != b));
        ControlFlow::Continue(())
    }

    fn xslt32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_i32();
        let b = state[operands.src2].get_i32();
        state[operands.dst].set_u64(u64::from(a < b));
        ControlFlow::Continue(())
    }

    fn xslteq32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_i32();
        let b = state[operands.src2].get_i32();
        state[operands.dst].set_u64(u64::from(a <= b));
        ControlFlow::Continue(())
    }

    fn xult32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_u32();
        let b = state[operands.src2].get_u32();
        state[operands.dst].set_u64(u64::from(a < b));
        ControlFlow::Continue(())
    }

    fn xulteq32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, operands: BinaryOperands<XReg>) {
        let a = state[operands.src1].get_u32();
        let b = state[operands.src2].get_u32();
        state[operands.dst].set_u64(u64::from(a <= b));
        ControlFlow::Continue(())
    }

    fn load32_u(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, ptr: XReg) {
        let ptr = state[ptr].get_ptr::<u32>();
        let val = unsafe { ptr::read_unaligned(ptr) };
        state[dst].set_u64(u64::from(val));
        ControlFlow::Continue(())
    }

    fn load32_s(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, ptr: XReg) {
        let ptr = state[ptr].get_ptr::<i32>();
        let val = unsafe { ptr::read_unaligned(ptr) };
        state[dst].set_i64(i64::from(val));
        ControlFlow::Continue(())
    }

    fn load64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, ptr: XReg) {
        let ptr = state[ptr].get_ptr::<u64>();
        let val = unsafe { ptr::read_unaligned(ptr) };
        state[dst].set_u64(val);
        ControlFlow::Continue(())
    }

    fn load32_u_offset8(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, ptr: XReg, offset: i8) {
        let val = unsafe {
            state[ptr]
                .get_ptr::<u32>()
                .byte_offset(offset.into())
                .read_unaligned()
        };
        state[dst].set_u64(u64::from(val));
        ControlFlow::Continue(())
    }

    fn load32_s_offset8(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, ptr: XReg, offset: i8) {
        let val = unsafe {
            state[ptr]
                .get_ptr::<i32>()
                .byte_offset(offset.into())
                .read_unaligned()
        };
        state[dst].set_i64(i64::from(val));
        ControlFlow::Continue(())
    }

    fn load32_u_offset64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, ptr: XReg, offset: i64) {
        let val = unsafe {
            state[ptr]
                .get_ptr::<u32>()
                .byte_offset(offset as isize)
                .read_unaligned()
        };
        state[dst].set_u64(u64::from(val));
        ControlFlow::Continue(())
    }

    fn load32_s_offset64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, ptr: XReg, offset: i64) {
        let val = unsafe {
            state[ptr]
                .get_ptr::<i32>()
                .byte_offset(offset as isize)
                .read_unaligned()
        };
        state[dst].set_i64(i64::from(val));
        ControlFlow::Continue(())
    }

    fn load64_offset8(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, ptr: XReg, offset: i8) {
        let val = unsafe {
            state[ptr]
                .get_ptr::<u64>()
                .byte_offset(offset.into())
                .read_unaligned()
        };
        state[dst].set_u64(val);
        ControlFlow::Continue(())
    }

    fn load64_offset64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, ptr: XReg, offset: i64) {
        let val = unsafe {
            state[ptr]
                .get_ptr::<u64>()
                .byte_offset(offset as isize)
                .read_unaligned()
        };
        state[dst].set_u64(val);
        ControlFlow::Continue(())
    }

    fn store32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, ptr: XReg, src: XReg) {
        let ptr = state[ptr].get_ptr::<u32>();
        let val = state[src].get_u32();
        unsafe {
            ptr::write_unaligned(ptr, val);
        }
        ControlFlow::Continue(())
    }

    fn store64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, ptr: XReg, src: XReg) {
        let ptr = state[ptr].get_ptr::<u64>();
        let val = state[src].get_u64();
        unsafe {
            ptr::write_unaligned(ptr, val);
        }
        ControlFlow::Continue(())
    }

    fn store32_offset8(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, ptr: XReg, offset: i8, src: XReg) {
        let val = state[src].get_u32();
        unsafe {
            state[ptr]
                .get_ptr::<u32>()
                .byte_offset(offset.into())
                .write_unaligned(val);
        }
        ControlFlow::Continue(())
    }

    fn store64_offset8(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, ptr: XReg, offset: i8, src: XReg) {
        let val = state[src].get_u64();
        unsafe {
            state[ptr]
                .get_ptr::<u64>()
                .byte_offset(offset.into())
                .write_unaligned(val);
        }
        ControlFlow::Continue(())
    }

    fn store32_offset64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, ptr: XReg, offset: i64, src: XReg) {
        let val = state[src].get_u32();
        unsafe {
            state[ptr]
                .get_ptr::<u32>()
                .byte_offset(offset as isize)
                .write_unaligned(val);
        }
        ControlFlow::Continue(())
    }

    fn store64_offset64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, ptr: XReg, offset: i64, src: XReg) {
        let val = state[src].get_u64();
        unsafe {
            state[ptr]
                .get_ptr::<u64>()
                .byte_offset(offset as isize)
                .write_unaligned(val);
        }
        ControlFlow::Continue(())
    }

    fn xpush32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, src: XReg) {
        state.push(state[src].get_u32());
        ControlFlow::Continue(())
    }

    fn xpush32_many(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, srcs: RegSet<XReg>) {
        for src in srcs {
            state.push(state[src].get_u32());
        }
        ControlFlow::Continue(())
    }

    fn xpush64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, src: XReg) {
        state.push(state[src].get_u64());
        ControlFlow::Continue(())
    }

    fn xpush64_many(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, srcs: RegSet<XReg>) {
        for src in srcs {
            state.push(state[src].get_u64());
        }
        ControlFlow::Continue(())
    }

    fn xpop32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg) {
        let val = state.pop();
        state[dst].set_u32(val);
        ControlFlow::Continue(())
    }

    fn xpop32_many(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dsts: RegSet<XReg>) {
        for dst in dsts.into_iter().rev() {
            let val = state.pop();
            state[dst].set_u32(val);
        }
        ControlFlow::Continue(())
    }

    fn xpop64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg) {
        let val = state.pop();
        state[dst].set_u64(val);
        ControlFlow::Continue(())
    }

    fn xpop64_many(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dsts: RegSet<XReg>) {
        for dst in dsts.into_iter().rev() {
            let val = state.pop();
            state[dst].set_u64(val);
        }
        ControlFlow::Continue(())
    }

    fn push_frame(state: &mut MachineState, pc: &mut UnsafeBytecodeStream) {
        state.push(state[XReg::lr].get_ptr::<u8>());
        state.push(state[XReg::fp].get_ptr::<u8>());
        state[XReg::fp] = state[XReg::sp];
        ControlFlow::Continue(())
    }

    fn pop_frame(state: &mut MachineState, pc: &mut UnsafeBytecodeStream) {
        state[XReg::sp] = state[XReg::fp];
        let fp = state.pop();
        let lr = state.pop();
        state[XReg::fp].set_ptr::<u8>(fp);
        state[XReg::lr].set_ptr::<u8>(lr);
        ControlFlow::Continue(())
    }

    fn bitcast_int_from_float_32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, src: FReg) {
        let val = state[src].get_f32();
        state[dst].set_u64(u32::from_ne_bytes(val.to_ne_bytes()).into());
        ControlFlow::Continue(())
    }

    fn bitcast_int_from_float_64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: XReg, src: FReg) {
        let val = state[src].get_f64();
        state[dst].set_u64(u64::from_ne_bytes(val.to_ne_bytes()));
        ControlFlow::Continue(())
    }

    fn bitcast_float_from_int_32(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: FReg, src: XReg) {
        let val = state[src].get_u32();
        state[dst].set_f32(f32::from_ne_bytes(val.to_ne_bytes()));
        ControlFlow::Continue(())
    }

    fn bitcast_float_from_int_64(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, dst: FReg, src: XReg) {
        let val = state[src].get_u64();
        state[dst].set_f64(f64::from_ne_bytes(val.to_ne_bytes()));
        ControlFlow::Continue(())
    }

    fn extended(state: &mut MachineState, pc: &mut UnsafeBytecodeStream, opcode: ExtendedOpcode) {
        match opcode {
            ExtendedOpcode::Nop => ControlFlow::Continue(()),
            ExtendedOpcode::Trap => ControlFlow::Break(Done::Trap),
            ExtendedOpcode::GetSp => {
                let (dst,) = crate::decode::unwrap_uninhabited(crate::decode::operands::get_sp(pc));
                let sp = state[XReg::sp].get_u64();
                state[dst].set_u64(sp);
                ControlFlow::Continue(())
            }
        }
    }
}

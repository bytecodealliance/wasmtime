use super::*;

type Handler = fn(&mut MachineState, &mut UnsafeBytecodeStream) -> Done;

/// The extra indirection through a macro is necessary to avoid a compiler error
/// when compiling without `#![feature(explicit_tail_calls)]` enabled (via
/// `--cfg pulley_tail_calls`).
///
/// It seems rustc first parses the function, encounters `become` and emits
/// an error about using an unstable keyword on a stable compiler, then applies
/// `#[cfg(...)` after parsing to disable the function.
///
/// Macro bodies are just bags of tokens; the body is not parsed until after
/// they are expanded, and this macro is only expanded when `pulley_tail_calls`
/// is enabled.
macro_rules! tail_call {
    ($e:expr) => {
        become $e
    };
}

pub fn run(vm: &mut Vm, bytecode: &mut UnsafeBytecodeStream) -> Done {
    run_one(&mut vm.state, bytecode)
}

fn run_one(state: &mut MachineState, bytecode: &mut UnsafeBytecodeStream) -> Done {
    let opcode = Opcode::decode(bytecode).unwrap();
    let handler = OPCODE_HANDLER_TABLE[opcode as usize];
    tail_call!(handler(state, bytecode));
}

macro_rules! define_opcode_handler_table {
    ($(
        $( #[$attr:meta] )*
        $snake_name:ident = $name:ident $( {
            $(
                $( #[$field_attr:meta] )*
                $field:ident : $field_ty:ty
            ),*
        } )?;
    )*) => {
        [
            $($snake_name,)*
            extended,
        ]
    };
}

/// Add one to account for `ExtendedOp`.
const NUM_OPCODES: usize = Opcode::MAX as usize + 1;
static OPCODE_HANDLER_TABLE: [Handler; NUM_OPCODES] = for_each_op!(define_opcode_handler_table);

macro_rules! define_opcode_handler {
    ($(
        $( #[$attr:meta] )*
        $snake_name:ident = $name:ident $( {
            $(
                $( #[$field_attr:meta] )*
                $field:ident : $field_ty:ty
            ),*
        } )?;
    )*) => {$(
        fn $snake_name(state: &mut MachineState, bytecode: &mut UnsafeBytecodeStream) -> Done {
            $($(
                let $field = unwrap_uninhabited(<$field_ty>::decode(bytecode));
            )*)?
            match super::$snake_name(state, bytecode, $($($field),*)?) {
                ControlFlow::Continue(()) => tail_call!(run_one(state, bytecode)),
                ControlFlow::Break(done) => done,
            }
        }
    )*};
}

for_each_op!(define_opcode_handler);

fn extended(state: &mut MachineState, bytecode: &mut UnsafeBytecodeStream) -> Done {
    let opcode = unwrap_uninhabited(ExtendedOpcode::decode(bytecode));
    match super::extended(state, bytecode, opcode) {
        ControlFlow::Continue(()) => tail_call!(run_one(state, bytecode)),
        ControlFlow::Break(done) => done,
    }
}

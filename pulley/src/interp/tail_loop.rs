use super::*;
use crate::decode::unwrap_uninhabited;
use crate::opcode::Opcode;

type Handler = fn(Interpreter<'_>) -> Done;

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

impl Interpreter<'_> {
    pub fn run(mut self) -> Done {
        // Perform a dynamic dispatch through a function pointer indexed by
        // opcode.
        let opcode = unwrap_uninhabited(Opcode::decode(self.bytecode()));
        let handler = OPCODE_HANDLER_TABLE[opcode as usize];
        tail_call!(handler(self));
    }
}

/// Same as `Interpreter::run`, except for extended opcodes.
fn run_extended(mut i: Interpreter<'_>) -> Done {
    let opcode = unwrap_uninhabited(ExtendedOpcode::decode(i.bytecode()));
    let handler = EXTENDED_OPCODE_HANDLER_TABLE[opcode as usize];
    tail_call!(handler(i));
}

static OPCODE_HANDLER_TABLE: [Handler; Opcode::MAX as usize + 1] = {
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
                $($snake_name,)* // refers to functions defined down below
                run_extended,
            ]
        };
    }

    for_each_op!(define_opcode_handler_table)
};

// same as above, but without a +1 for handling of extended ops as this is the
// extended ops.
static EXTENDED_OPCODE_HANDLER_TABLE: [Handler; ExtendedOpcode::MAX as usize] = {
    macro_rules! define_extended_opcode_handler_table {
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
                $($snake_name,)* // refers to functions defined down below
            ]
        };
    }

    for_each_extended_op!(define_extended_opcode_handler_table)
};

// Define a top-level function for each opcode. Each function here is the
// destination of the indirect return-call-indirect of above. Each function is
// also specialized to a single opcode and should be thoroughly inlined to
// ensure that everything "boils away".
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
        fn $snake_name(mut i: Interpreter<'_>) -> Done {
            $(
                let ($($field,)*) = unwrap_uninhabited(
                    crate::decode::operands::$snake_name(i.bytecode())
                );
            )?
            match OpVisitor::$snake_name(&mut i, $($($field),*)?) {
                ControlFlow::Continue(()) => tail_call!(i.run()),
                ControlFlow::Break(done) => done,
            }
        }
    )*};
}

for_each_op!(define_opcode_handler);

macro_rules! define_extended_opcode_handler {
    ($(
        $( #[$attr:meta] )*
        $snake_name:ident = $name:ident $( {
            $(
                $( #[$field_attr:meta] )*
                $field:ident : $field_ty:ty
            ),*
        } )?;
    )*) => {$(
        fn $snake_name(mut i: Interpreter<'_>) -> Done {
            $(
                let ($($field,)*) = unwrap_uninhabited(
                    crate::decode::operands::$snake_name(i.bytecode())
                );
            )?
            match ExtendedOpVisitor::$snake_name(&mut i, $($($field),*)?) {
                ControlFlow::Continue(()) => tail_call!(i.run()),
                ControlFlow::Break(done) => done,
            }
        }
    )*};
}
for_each_extended_op!(define_extended_opcode_handler);

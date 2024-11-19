use super::*;
use crate::decode::unwrap_uninhabited;

pub fn run(vm: &mut Vm, bytecode: &mut UnsafeBytecodeStream) -> Done {
    macro_rules! define_loop {
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
        ) => {
            loop {
                match unwrap_uninhabited(Opcode::decode(bytecode)) {
                    $(
                        Opcode::$name => {
                            $($(
                                let $field = unwrap_uninhabited(<$field_ty>::decode(bytecode));
                            )*)?
                            match super::$snake_name(&mut vm.state, bytecode, $($($field),*)?) {
                                ControlFlow::Continue(()) => {}
                                ControlFlow::Break(done) => break done,
                            }
                        }
                    )*

                    Opcode::ExtendedOp => {
                        let opcode = unwrap_uninhabited(ExtendedOpcode::decode(bytecode));
                        match super::extended(&mut vm.state, bytecode, opcode) {
                            ControlFlow::Continue(()) => {}
                            ControlFlow::Break(done) => break done,
                        }
                    }
                }
            }
        };
    }

    for_each_op!(define_loop)
}

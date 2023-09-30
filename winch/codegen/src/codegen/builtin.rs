//! Builtin function handling.

use crate::{
    abi::{ABISig, ABI},
    codegen::env::ptr_type_from_ptr_size,
    CallingConvention,
};
use wasmtime_environ::{BuiltinFunctionIndex, PtrSize, WasmType};

/// Metadata about a builtin function.
pub(crate) struct BuiltinFunction {
    /// The ABI specific signature of the function.
    pub sig: ABISig,
    /// The offset of the builtin function
    pub offset: u32,
    /// The builtin function base, relative to the VMContext.
    pub base: u32,
}

macro_rules! declare_function_sig {
    (
        $(
            $( #[$attr:meta] )*
            $name:ident( $( $pname:ident: $param:ident ),* ) $( -> $result:ident )?;
         )*
    ) => {
        /// Provides the ABI signatures for each builtin function
        /// signature.
        pub struct BuiltinFunctions {
            /// The target calling convention.
            call_conv: CallingConvention,
            /// The target pointer size.
            ptr_size: u8,
            /// The target pointer type, as a WebAssembly type.
            ptr_type: WasmType,
            /// The builtin functions base relative to the VMContext.
            base: u32,
            $(
                $name: Option<BuiltinFunction>,
            )*
        }

        // Until all the builtin functions are used.
        #[allow(dead_code)]
        impl BuiltinFunctions {
            pub fn new(ptr: impl PtrSize, call_conv: CallingConvention, base: u32) -> Self {
                let size = ptr.size();
                Self {
                    ptr_size: size,
                    call_conv,
                    base,
                    ptr_type: ptr_type_from_ptr_size(size),
                    $(
                        $name: None,
                    )*
                }
            }

            fn pointer(&self) -> WasmType {
                self.ptr_type
            }

            fn vmctx(&self) -> WasmType {
                self.pointer()
            }

            fn i32(&self) -> WasmType {
                WasmType::I32
            }

            fn i64(&self) -> WasmType {
                WasmType::I64
            }

            fn reference(&self) -> WasmType {
                self.pointer()
            }

            $(
                pub(crate) fn $name<A: ABI, P: PtrSize>(&mut self) -> &BuiltinFunction {
                    if self.$name.is_none() {
                        let params = vec![ $(self.$param() ),* ];
                        let result = vec![ $(self.$result() )?];
                        let sig = A::sig_from(&params, &result, &self.call_conv);
                        let index = BuiltinFunctionIndex::$name();
                        self.$name = Some(BuiltinFunction {
                            sig,
                            offset: index.index() * (self.ptr_size as u32),
                            base: self.base,
                        });
                    }

                    self.$name.as_ref().unwrap()
                }
             )*
        }
    }
}

wasmtime_environ::foreach_builtin_function!(declare_function_sig);

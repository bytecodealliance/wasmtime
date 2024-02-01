//! Builtin function handling.

use crate::{
    abi::{ABISig, ABI},
    codegen::env::ptr_type_from_ptr_size,
    CallingConvention,
};
use cranelift_codegen::ir::LibCall;
use std::sync::Arc;
use wasmtime_environ::{BuiltinFunctionIndex, PtrSize, VMOffsets, WasmValType};

#[derive(Copy, Clone)]
pub(crate) enum BuiltinType {
    /// Dynamic built-in function, derived from the VMContext.
    Dynamic {
        /// The index of the built-in function.
        index: u32,
        /// The built-in function base, relative to the VMContext.
        base: u32,
    },
    /// A known libcall.
    /// See [`cranelift_codegen::ir::LibCall`] for more details.
    Known(LibCall),
}

impl BuiltinType {
    /// Create a new dynamic built-in function type.
    pub fn dynamic(index: u32, base: u32) -> Self {
        Self::Dynamic { index, base }
    }

    /// Create a new known built-in function type.
    pub fn known(libcall: LibCall) -> Self {
        Self::Known(libcall)
    }
}

#[derive(Clone)]
pub struct BuiltinFunction {
    inner: Arc<BuiltinFunctionInner>,
}

impl BuiltinFunction {
    pub(crate) fn sig(&self) -> &ABISig {
        &self.inner.sig
    }

    pub(crate) fn ty(&self) -> BuiltinType {
        self.inner.ty
    }
}

/// Metadata about a builtin function.
pub struct BuiltinFunctionInner {
    /// The ABI specific signature of the function.
    sig: ABISig,
    /// The built-in function type.
    ty: BuiltinType,
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
            ptr_type: WasmValType,
            /// The builtin functions base relative to the VMContext.
            base: u32,
            /// F32 Ceil.
            ceil_f32: Option<BuiltinFunction>,
            /// F64 Ceil.
            ceil_f64: Option<BuiltinFunction>,
            /// F32 Floor.
            floor_f32: Option<BuiltinFunction>,
            /// F64 Floor.
            floor_f64: Option<BuiltinFunction>,
            /// F32 Trunc.
            trunc_f32: Option<BuiltinFunction>,
            /// F64 Trunc.
            trunc_f64: Option<BuiltinFunction>,
            /// F32 Nearest.
            nearest_f32: Option<BuiltinFunction>,
            /// F64 Nearest.
            nearest_f64: Option<BuiltinFunction>,
            $(
                $name: Option<BuiltinFunction>,
            )*
        }

        // Until all the builtin functions are used.
        #[allow(dead_code)]
        impl BuiltinFunctions {
            pub fn new<P: PtrSize>(vmoffsets: &VMOffsets<P>, call_conv: CallingConvention) -> Self {
                let size = vmoffsets.ptr.size();
                Self {
                    ptr_size: size,
                    call_conv,
                    base: vmoffsets.vmctx_builtin_functions(),
                    ptr_type: ptr_type_from_ptr_size(size),
                    ceil_f32: None,
                    ceil_f64: None,
                    floor_f32: None,
                    floor_f64: None,
                    trunc_f32: None,
                    trunc_f64: None,
                    nearest_f32: None,
                    nearest_f64: None,
                    $(
                        $name: None,
                    )*
                }
            }

            fn pointer(&self) -> WasmValType {
                self.ptr_type
            }

            fn vmctx(&self) -> WasmValType {
                self.pointer()
            }

            fn i32(&self) -> WasmValType {
                WasmValType::I32
            }

            fn f32(&self) -> WasmValType {
                WasmValType::F32
            }

            fn f64(&self) -> WasmValType {
                WasmValType::F64
            }

            fn i64(&self) -> WasmValType {
                WasmValType::I64
            }

            fn reference(&self) -> WasmValType {
                self.pointer()
            }

            fn over_f64<A: ABI>(&self) -> ABISig {
                A::sig_from(&[self.f64()], &[self.f64()], &self.call_conv)
            }

            fn over_f32<A: ABI>(&self) -> ABISig {
                A::sig_from(&[self.f64()], &[self.f64()], &self.call_conv)
            }

            pub(crate) fn ceil_f32<A: ABI>(&mut self) -> BuiltinFunction {
                if self.ceil_f32.is_none() {
                    let sig = self.over_f32::<A>();
                    let inner = Arc::new(BuiltinFunctionInner { sig, ty: BuiltinType::known(LibCall::CeilF32) });
                    self.ceil_f32 = Some(BuiltinFunction {
                        inner,
                    });
                }
                self.ceil_f32.as_ref().unwrap().clone()
            }

            pub(crate) fn ceil_f64<A: ABI>(&mut self) -> BuiltinFunction {
                if self.ceil_f64.is_none() {
                    let sig = self.over_f64::<A>();
                    let inner = Arc::new(BuiltinFunctionInner { sig, ty: BuiltinType::known(LibCall::CeilF64) });
                    self.ceil_f64 = Some(BuiltinFunction {
                        inner,
                    });
                }
                self.ceil_f64.as_ref().unwrap().clone()
            }

            pub(crate) fn floor_f32<A: ABI>(&mut self) -> BuiltinFunction {
                if self.floor_f32.is_none() {
                    let sig = self.over_f32::<A>();
                    let inner = Arc::new(BuiltinFunctionInner { sig, ty: BuiltinType::known(LibCall::FloorF32) });
                    self.floor_f32 = Some(BuiltinFunction {
                        inner,
                    });
                }
                self.floor_f32.as_ref().unwrap().clone()
            }

            pub(crate) fn floor_f64<A: ABI>(&mut self) -> BuiltinFunction {
                if self.floor_f64.is_none() {
                    let sig = self.over_f64::<A>();
                    let inner = Arc::new(BuiltinFunctionInner { sig, ty: BuiltinType::known(LibCall::FloorF64) });
                    self.floor_f64 = Some(BuiltinFunction {
                        inner,
                    });
                }
                self.floor_f64.as_ref().unwrap().clone()
            }

            pub(crate) fn trunc_f32<A: ABI>(&mut self) -> BuiltinFunction {
                if self.trunc_f32.is_none() {
                    let sig = self.over_f32::<A>();
                    let inner = Arc::new(BuiltinFunctionInner { sig, ty: BuiltinType::known(LibCall::TruncF32) });
                    self.trunc_f32 = Some(BuiltinFunction {
                        inner,
                    });
                }
                self.trunc_f32.as_ref().unwrap().clone()
            }

            pub(crate) fn trunc_f64<A: ABI>(&mut self) -> BuiltinFunction {
                if self.trunc_f64.is_none() {
                    let sig = self.over_f64::<A>();
                    let inner = Arc::new(BuiltinFunctionInner { sig, ty: BuiltinType::known(LibCall::TruncF64) });
                    self.trunc_f64 = Some(BuiltinFunction {
                        inner,
                    });
                }
                self.trunc_f64.as_ref().unwrap().clone()
            }

            pub(crate) fn nearest_f32<A: ABI>(&mut self) -> BuiltinFunction {
                if self.nearest_f32.is_none() {
                    let sig = self.over_f32::<A>();
                    let inner = Arc::new(BuiltinFunctionInner { sig, ty: BuiltinType::known(LibCall::NearestF32) });
                    self.nearest_f32 = Some(BuiltinFunction {
                        inner,
                    });
                }
                self.nearest_f32.as_ref().unwrap().clone()
            }

            pub(crate) fn nearest_f64<A: ABI>(&mut self) -> BuiltinFunction {
                if self.nearest_f64.is_none() {
                    let sig = self.over_f64::<A>();
                    let inner = Arc::new(BuiltinFunctionInner { sig, ty: BuiltinType::known(LibCall::NearestF64) });
                    self.nearest_f64 = Some(BuiltinFunction {
                        inner,
                    });
                }
                self.nearest_f64.as_ref().unwrap().clone()
            }

            $(
                pub(crate) fn $name<A: ABI, P: PtrSize>(&mut self) -> BuiltinFunction {
                    if self.$name.is_none() {
                        let params = vec![ $(self.$param() ),* ];
                        let result = vec![ $(self.$result() )?];
                        let sig = A::sig_from(&params, &result, &self.call_conv);
                        let index = BuiltinFunctionIndex::$name();
                        let inner = Arc::new(BuiltinFunctionInner { sig, ty: BuiltinType::dynamic(index.index() * (self.ptr_size as u32), self.base) });
                        self.$name = Some(BuiltinFunction {
                            inner,
                        });
                    }

                    self.$name.as_ref().unwrap().clone()
                }
             )*
        }
    }
}

wasmtime_environ::foreach_builtin_function!(declare_function_sig);

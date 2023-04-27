use crate::store::StoreOpaque;
use crate::{GlobalType, Mutability, Val};
use std::ptr;
use wasmtime_environ::GlobalInit;
use wasmtime_runtime::VMGlobalDefinition;

#[repr(C)]
pub struct VMHostGlobalContext {
    ty: GlobalType,
    global: VMGlobalDefinition,
}

impl Drop for VMHostGlobalContext {
    fn drop(&mut self) {
        match self.ty.content() {
            crate::ValType::I32
            | crate::ValType::I64
            | crate::ValType::F32
            | crate::ValType::F64
            | crate::ValType::V128
            | crate::ValType::FuncRef => {
                // Nothing to drop.
            }
            crate::ValType::ExternRef => unsafe {
                ptr::drop_in_place(self.global.as_externref_mut())
            },
        }
    }
}

pub fn generate_global_export(
    store: &mut StoreOpaque,
    ty: GlobalType,
    val: Val,
) -> wasmtime_runtime::ExportGlobal {
    let mut ctx = Box::new(VMHostGlobalContext {
        ty,
        global: VMGlobalDefinition::new(),
    });

    unsafe {
        match val {
            Val::I32(x) => *ctx.global.as_i32_mut() = x,
            Val::I64(x) => *ctx.global.as_i64_mut() = x,
            Val::F32(x) => *ctx.global.as_f32_bits_mut() = x,
            Val::F64(x) => *ctx.global.as_f64_bits_mut() = x,
            Val::V128(x) => *ctx.global.as_u128_mut() = x,
            Val::FuncRef(f) => {
                *ctx.global.as_anyfunc_mut() = f.map_or(ptr::null_mut(), |f| {
                    f.caller_checked_func_ref(store).as_ptr()
                })
            }
            Val::ExternRef(x) => *ctx.global.as_externref_mut() = x.map(|x| x.inner),
        }
    }

    let ret = wasmtime_runtime::ExportGlobal {
        definition: &mut ctx.global as *mut _,
        global: wasmtime_environ::Global {
            wasm_ty: ctx.ty.content().to_wasm_type(),
            mutability: match ctx.ty.mutability() {
                Mutability::Const => false,
                Mutability::Var => true,
            },
            // TODO: This is just a dummy value; nothing should actually read
            // this. We should probably remove this field from the struct.
            initializer: GlobalInit::I32Const(0),
        },
    };
    store.host_globals().push(ctx);
    ret
}

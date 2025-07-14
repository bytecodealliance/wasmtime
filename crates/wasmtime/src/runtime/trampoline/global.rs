use crate::runtime::vm::{ExportGlobalKind, StoreBox, VMGlobalDefinition};
use crate::store::{AutoAssertNoGc, StoreOpaque};
use crate::type_registry::RegisteredType;
use crate::{GlobalType, Mutability, Result, RootedGcRefImpl, Val};
use core::ptr::{self, NonNull};
use wasmtime_environ::{DefinedGlobalIndex, EntityRef, Global};

#[repr(C)]
pub struct VMHostGlobalContext {
    pub(crate) ty: Global,
    pub(crate) global: VMGlobalDefinition,

    _registered_type: Option<RegisteredType>,
}

pub fn generate_global_export(
    store: &mut StoreOpaque,
    ty: GlobalType,
    val: Val,
) -> Result<crate::runtime::vm::ExportGlobal> {
    let global = wasmtime_environ::Global {
        wasm_ty: ty.content().to_wasm_type(),
        mutability: match ty.mutability() {
            Mutability::Const => false,
            Mutability::Var => true,
        },
    };
    let ctx = StoreBox::new(VMHostGlobalContext {
        ty: global,
        global: VMGlobalDefinition::new(),
        _registered_type: ty.into_registered_type(),
    });

    let mut store = AutoAssertNoGc::new(store);
    let definition = unsafe {
        let global = &mut ctx.get().as_mut().global;
        match val {
            Val::I32(x) => *global.as_i32_mut() = x,
            Val::I64(x) => *global.as_i64_mut() = x,
            Val::F32(x) => *global.as_f32_bits_mut() = x,
            Val::F64(x) => *global.as_f64_bits_mut() = x,
            Val::V128(x) => global.set_u128(x.into()),
            Val::FuncRef(f) => {
                *global.as_func_ref_mut() =
                    f.map_or(ptr::null_mut(), |f| f.vm_func_ref(&store).as_ptr());
            }
            Val::ExternRef(x) => {
                let new = match x {
                    None => None,
                    Some(x) => Some(x.try_gc_ref(&store)?.unchecked_copy()),
                };
                let new = new.as_ref();
                global.write_gc_ref(store.gc_store_mut()?, new);
            }
            Val::AnyRef(a) => {
                let new = match a {
                    None => None,
                    Some(a) => Some(a.try_gc_ref(&store)?.unchecked_copy()),
                };
                let new = new.as_ref();
                global.write_gc_ref(store.gc_store_mut()?, new);
            }
        }
        global
    };

    let globals = store.host_globals();
    let index = DefinedGlobalIndex::new(globals.len());
    store.host_globals_mut().push(ctx);
    Ok(crate::runtime::vm::ExportGlobal {
        definition: NonNull::from(definition),
        kind: ExportGlobalKind::Host(index),
        global,
    })
}

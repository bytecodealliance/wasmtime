use crate::runtime::vm::{StoreBox, VMGlobalDefinition};
use crate::store::{AutoAssertNoGc, StoreOpaque};
use crate::type_registry::RegisteredType;
use crate::{GlobalType, Mutability, Result, Val};
use wasmtime_environ::Global;

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
) -> Result<crate::Global> {
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
    })?;

    let mut store = AutoAssertNoGc::new(store);
    // SAFETY: the global that this is pointing to is rooted in `ctx` above and
    // is safe to initialize.
    unsafe {
        let global = &mut ctx.get().as_mut().global;
        match val {
            Val::I32(x) => global.set_i32(x),
            Val::I64(x) => global.set_i64(x),
            Val::F32(x) => global.set_f32_bits(x),
            Val::F64(x) => global.set_f64_bits(x),
            Val::V128(x) => global.set_u128(x.into()),
            Val::FuncRef(f) => {
                global.set_func_ref(f.map(|f| f.vm_func_ref(&store)));
            }
            Val::ExternRef(x) => {
                let new = match x {
                    None => None,
                    Some(x) => Some(x.try_gc_ref(&store)?.unchecked_copy()),
                };
                let new = new.as_ref();
                global.write_gc_ref(&mut store, new)?;
            }
            Val::AnyRef(a) => {
                let new = match a {
                    None => None,
                    Some(a) => Some(a.try_gc_ref(&store)?.unchecked_copy()),
                };
                let new = new.as_ref();
                global.write_gc_ref(&mut store, new)?;
            }
            Val::ExnRef(e) => {
                let new = match e {
                    None => None,
                    Some(e) => Some(e.try_gc_ref(&store)?.unchecked_copy()),
                };
                let new = new.as_ref();
                global.write_gc_ref(&mut store, new)?;
            }
            Val::ContRef(None) => {
                // Allow null continuation references for trampoline globals - these are just placeholders
                global.write_gc_ref(&mut store, None)?;
            }
            Val::ContRef(Some(_)) => {
                // TODO(#10248): Implement non-null trampoline continuation reference handling
                return Err(crate::format_err!(
                    "non-null continuation references in trampoline globals not yet supported"
                ));
            }
        }
    }

    let index = store.host_globals_mut().push(ctx)?;
    Ok(crate::Global::from_host(store.id(), index))
}

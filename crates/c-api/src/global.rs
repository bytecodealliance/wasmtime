use crate::{wasm_extern_t, wasm_globaltype_t, wasm_store_t, wasm_val_t, ExternHost};
use wasmtime::{Global, HostRef};

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_global_t {
    ext: wasm_extern_t,
}

impl wasm_global_t {
    pub(crate) fn try_from(e: &wasm_extern_t) -> Option<&wasm_global_t> {
        match &e.which {
            ExternHost::Global(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    fn global(&self) -> &HostRef<Global> {
        match &self.ext.which {
            ExternHost::Global(g) => g,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_global_new(
    store: &wasm_store_t,
    gt: &wasm_globaltype_t,
    val: &wasm_val_t,
) -> Option<Box<wasm_global_t>> {
    let global =
        HostRef::new(Global::new(&store.store.borrow(), gt.ty().ty.clone(), val.val()).ok()?);
    Some(Box::new(wasm_global_t {
        ext: wasm_extern_t {
            which: ExternHost::Global(global),
        },
    }))
}

#[no_mangle]
pub extern "C" fn wasm_global_as_extern(g: &wasm_global_t) -> &wasm_extern_t {
    &g.ext
}

#[no_mangle]
pub extern "C" fn wasm_global_copy(g: &wasm_global_t) -> Box<wasm_global_t> {
    Box::new(g.clone())
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_same(
    g1: *const wasm_global_t,
    g2: *const wasm_global_t,
) -> bool {
    (*g1).global().ptr_eq(&(*g2).global())
}

#[no_mangle]
pub extern "C" fn wasm_global_type(g: &wasm_global_t) -> Box<wasm_globaltype_t> {
    let globaltype = g.global().borrow().ty().clone();
    Box::new(wasm_globaltype_t::new(globaltype))
}

#[no_mangle]
pub extern "C" fn wasm_global_get(g: &wasm_global_t, out: &mut wasm_val_t) {
    out.set(g.global().borrow().get());
}

#[no_mangle]
pub extern "C" fn wasm_global_set(g: &wasm_global_t, val: &wasm_val_t) {
    let result = g.global().borrow().set(val.val());
    drop(result); // TODO: should communicate this via the api somehow?
}

#[no_mangle]
pub extern "C" fn wasm_global_delete(_g: Box<wasm_global_t>) {}

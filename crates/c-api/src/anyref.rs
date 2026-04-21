use crate::{WasmtimeStoreContextMut, wasmtime_arrayref_t, wasmtime_eqref_t, wasmtime_structref_t};
use std::mem::MaybeUninit;
use wasmtime::{AnyRef, ArrayRef, EqRef, I31, OwnedRooted, RootScope, StructRef};

/// C-API representation of `anyref`.
///
/// This represented differently in the C API from the header to handle how
/// this is dispatched internally. Null anyref values are represented with a
/// `store_id` of zero, and otherwise the `rooted` field is valid.
///
/// Note that this relies on the Wasmtime definition of `OwnedRooted` to have
/// a 64-bit store_id first.
macro_rules! ref_wrapper {
    ({
        wasmtime: $wasmtime:ident,
        capi: $c:ident,
        clone: $clone:ident,
        unroot: $unroot:ident,
        $(
            to_raw: $to_raw:ident,
            from_raw: $from_raw:ident,
        )?
   }) => {
        pub struct $c {
            store_id: u64,
            a: u32,
            b: u32,
            c: *const (),
        }

        impl $c {
            pub unsafe fn as_wasmtime(&self) -> Option<wasmtime::OwnedRooted<$wasmtime>> {
                let store_id = std::num::NonZeroU64::new(self.store_id)?;
                Some(wasmtime::OwnedRooted::from_borrowed_raw_parts_for_c_api(
                    store_id, self.a, self.b, self.c,
                ))
            }

            pub unsafe fn into_wasmtime(self) -> Option<wasmtime::OwnedRooted<$wasmtime>> {
                std::mem::ManuallyDrop::new(self).to_owned()
            }

            unsafe fn to_owned(&self) -> Option<wasmtime::OwnedRooted<$wasmtime>> {
                let store_id = std::num::NonZeroU64::new(self.store_id)?;
                Some(wasmtime::OwnedRooted::from_owned_raw_parts_for_c_api(
                    store_id, self.a, self.b, self.c,
                ))
            }
        }

        impl Drop for $c {
            fn drop(&mut self) {
                unsafe {
                    let _ = self.to_owned();
                }
            }
        }

        impl From<Option<wasmtime::OwnedRooted<$wasmtime>>> for $c {
            fn from(rooted: Option<wasmtime::OwnedRooted<$wasmtime>>) -> $c {
                let mut ret = $c {
                    store_id: 0,
                    a: 0,
                    b: 0,
                    c: core::ptr::null(),
                };
                if let Some(rooted) = rooted {
                    let (store_id, a, b, c) = rooted.into_parts_for_c_api();
                    ret.store_id = store_id.get();
                    ret.a = a;
                    ret.b = b;
                    ret.c = c;
                }
                ret
            }
        }

        impl From<wasmtime::OwnedRooted<$wasmtime>> for $c {
            fn from(rooted: wasmtime::OwnedRooted<$wasmtime>) -> $c {
                Self::from(Some(rooted))
            }
        }

        // SAFETY: The `*const ()` comes from (and is converted back
        // into) an `Arc<()>`, and is only accessed as such, so this
        // type is both Send and Sync. These constraints are necessary
        // in the async machinery in this crate.
        unsafe impl Send for $c {}
        unsafe impl Sync for $c {}

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $clone(anyref: Option<&$c>, out: &mut std::mem::MaybeUninit<$c>) {
            let anyref = anyref.and_then(|a| a.as_wasmtime());
            out.write(anyref.into());
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $unroot(val: Option<&mut std::mem::ManuallyDrop<$c>>) {
            if let Some(val) = val {
                unsafe {
                    std::mem::ManuallyDrop::drop(val);
                }
            }
        }

        $(
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $to_raw(
                cx: crate::WasmtimeStoreContextMut<'_>,
                val: Option<&$c>,
            ) -> u32 {
                val.and_then(|v| v.as_wasmtime())
                    .and_then(|e| e.to_raw(cx).ok())
                    .unwrap_or_default()
            }

            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $from_raw(
                cx: crate::WasmtimeStoreContextMut<'_>,
                raw: u32,
                val: &mut std::mem::MaybeUninit<$c>,
            ) {
                let mut scope = wasmtime::RootScope::new(cx);
                let anyref = $wasmtime::from_raw(&mut scope, raw)
                    .map(|a| a.to_owned_rooted(&mut scope).expect("in scope"));
                crate::initialize(val, anyref.into());
            }
        )?
    };
}
pub(crate) use ref_wrapper;

ref_wrapper!({
    wasmtime: AnyRef,
    capi: wasmtime_anyref_t,
    clone: wasmtime_anyref_clone,
    unroot: wasmtime_anyref_unroot,
    to_raw: wasmtime_anyref_to_raw,
    from_raw: wasmtime_anyref_from_raw,
});

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_anyref_from_i31(
    cx: WasmtimeStoreContextMut<'_>,
    val: u32,
    out: &mut MaybeUninit<wasmtime_anyref_t>,
) {
    let mut scope = RootScope::new(cx);
    let anyref = AnyRef::from_i31(&mut scope, I31::wrapping_u32(val));
    let anyref = anyref.to_owned_rooted(&mut scope).expect("in scope");
    crate::initialize(out, Some(anyref).into())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_is_i31(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) => anyref.is_i31(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_i31_get_u(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
    dst: &mut MaybeUninit<u32>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) if anyref.is_i31(&cx).expect("OwnedRooted always in scope") => {
            let val = anyref
                .unwrap_i31(&cx)
                .expect("OwnedRooted always in scope")
                .get_u32();
            crate::initialize(dst, val);
            true
        }
        _ => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_i31_get_s(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
    dst: &mut MaybeUninit<i32>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) if anyref.is_i31(&cx).expect("OwnedRooted always in scope") => {
            let val = anyref
                .unwrap_i31(&cx)
                .expect("OwnedRooted always in scope")
                .get_i32();
            crate::initialize(dst, val);
            true
        }
        _ => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_is_eqref(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) => anyref.is_eqref(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_as_eqref(
    mut cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
    out: &mut MaybeUninit<wasmtime_eqref_t>,
) -> bool {
    if let Some(anyref) = anyref.and_then(|a| a.as_wasmtime()) {
        let mut scope = RootScope::new(&mut cx);
        let rooted = anyref.to_rooted(&mut scope);
        if let Ok(Some(eqref)) = rooted.as_eqref(&mut scope) {
            let owned = eqref.to_owned_rooted(&mut scope).expect("in scope");
            crate::initialize(out, Some(owned).into());
            return true;
        }
    }
    crate::initialize(out, None::<OwnedRooted<EqRef>>.into());
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_is_struct(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) => anyref.is_struct(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_as_struct(
    mut cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
    out: &mut MaybeUninit<wasmtime_structref_t>,
) -> bool {
    if let Some(anyref) = anyref.and_then(|a| a.as_wasmtime()) {
        let mut scope = RootScope::new(&mut cx);
        let rooted = anyref.to_rooted(&mut scope);
        if let Ok(Some(structref)) = rooted.as_struct(&scope) {
            let owned = structref.to_owned_rooted(&mut scope).expect("in scope");
            crate::initialize(out, Some(owned).into());
            return true;
        }
    }
    crate::initialize(out, None::<OwnedRooted<StructRef>>.into());
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_is_array(
    cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
) -> bool {
    match anyref.and_then(|a| a.as_wasmtime()) {
        Some(anyref) => anyref.is_array(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_anyref_as_array(
    mut cx: WasmtimeStoreContextMut<'_>,
    anyref: Option<&wasmtime_anyref_t>,
    out: &mut MaybeUninit<wasmtime_arrayref_t>,
) -> bool {
    if let Some(anyref) = anyref.and_then(|a| a.as_wasmtime()) {
        let mut scope = RootScope::new(&mut cx);
        let rooted = anyref.to_rooted(&mut scope);
        if let Ok(Some(arrayref)) = rooted.as_array(&scope) {
            let owned = arrayref.to_owned_rooted(&mut scope).expect("in scope");
            crate::initialize(out, Some(owned).into());
            return true;
        }
    }
    crate::initialize(out, None::<OwnedRooted<ArrayRef>>.into());
    false
}

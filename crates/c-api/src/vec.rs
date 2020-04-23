use crate::wasm_valtype_t;
use crate::{wasm_exporttype_t, wasm_extern_t, wasm_frame_t, wasm_val_t};
use crate::{wasm_externtype_t, wasm_importtype_t, wasm_memorytype_t};
use crate::{wasm_functype_t, wasm_globaltype_t, wasm_tabletype_t};
use std::mem;
use std::ptr;
use std::slice;

pub type wasm_name_t = wasm_byte_vec_t;

impl wasm_name_t {
    pub(crate) fn from_name(name: String) -> wasm_name_t {
        name.into_bytes().into()
    }
}

macro_rules! declare_vecs {
    (
        $((
            name: $name:ident,
            ty: $elem_ty:ty,
            new: $new:ident,
            empty: $empty:ident,
            uninit: $uninit:ident,
            copy: $copy:ident,
            delete: $delete:ident,
        ))*
    ) => {$(
        #[repr(C)]
        #[derive(Clone)]
        pub struct $name {
            size: usize,
            data: *mut $elem_ty,
        }

        impl $name {
            pub fn set_buffer(&mut self, buffer: Vec<$elem_ty>) {
                let mut vec = buffer.into_boxed_slice();
                self.size = vec.len();
                self.data = vec.as_mut_ptr();
                mem::forget(vec);
            }

            pub fn as_slice(&self) -> &[$elem_ty] {
                // Note that we're careful to not create a slice with a null
                // pointer as the data pointer, since that isn't defined
                // behavior in Rust.
                if self.size == 0 {
                    &[]
                } else {
                    assert!(!self.data.is_null());
                    unsafe { slice::from_raw_parts(self.data, self.size) }
                }
            }

            pub fn take(&mut self) -> Vec<$elem_ty> {
                if self.data.is_null() {
                    return Vec::new();
                }
                let vec = unsafe {
                    Vec::from_raw_parts(self.data, self.size, self.size)
                };
                self.data = ptr::null_mut();
                self.size = 0;
                return vec;
            }
        }

        impl From<Vec<$elem_ty>> for $name {
            fn from(mut vec: Vec<$elem_ty>) -> Self {
                assert_eq!(vec.len(), vec.capacity());
                let result = $name {
                    size: vec.len(),
                    data: vec.as_mut_ptr(),
                };
                mem::forget(vec);
                result
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                drop(self.take());
            }
        }

        #[no_mangle]
        pub extern "C" fn $empty(out: &mut $name) {
            out.size = 0;
            out.data = ptr::null_mut();
        }

        #[no_mangle]
        pub extern "C" fn $uninit(out: &mut $name, size: usize) {
            out.set_buffer(vec![Default::default(); size]);
        }

        #[no_mangle]
        pub unsafe extern "C" fn $new(
            out: &mut $name,
            size: usize,
            ptr: *const $elem_ty,
        ) {
            let slice = slice::from_raw_parts(ptr, size);
            out.set_buffer(slice.to_vec());
        }

        #[no_mangle]
        pub extern "C" fn $copy(out: &mut $name, src: &$name) {
            out.set_buffer(src.as_slice().to_vec());
        }

        #[no_mangle]
        pub extern "C" fn $delete(out: &mut $name) {
            out.take();
        }
    )*};
}

declare_vecs! {
    (
        name: wasm_byte_vec_t,
        ty: u8,
        new: wasm_byte_vec_new,
        empty: wasm_byte_vec_new_empty,
        uninit: wasm_byte_vec_new_uninitialized,
        copy: wasm_byte_vec_copy,
        delete: wasm_byte_vec_delete,
    )
    (
        name: wasm_valtype_vec_t,
        ty: Option<Box<wasm_valtype_t>>,
        new: wasm_valtype_vec_new,
        empty: wasm_valtype_vec_new_empty,
        uninit: wasm_valtype_vec_new_uninitialized,
        copy: wasm_valtype_vec_copy,
        delete: wasm_valtype_vec_delete,
    )
    (
        name: wasm_functype_vec_t,
        ty: Option<Box<wasm_functype_t>>,
        new: wasm_functype_vec_new,
        empty: wasm_functype_vec_new_empty,
        uninit: wasm_functype_vec_new_uninitialized,
        copy: wasm_functype_vec_copy,
        delete: wasm_functype_vec_delete,
    )
    (
        name: wasm_globaltype_vec_t,
        ty: Option<Box<wasm_globaltype_t>>,
        new: wasm_globaltype_vec_new,
        empty: wasm_globaltype_vec_new_empty,
        uninit: wasm_globaltype_vec_new_uninitialized,
        copy: wasm_globaltype_vec_copy,
        delete: wasm_globaltype_vec_delete,
    )
    (
        name: wasm_tabletype_vec_t,
        ty: Option<Box<wasm_tabletype_t>>,
        new: wasm_tabletype_vec_new,
        empty: wasm_tabletype_vec_new_empty,
        uninit: wasm_tabletype_vec_new_uninitialized,
        copy: wasm_tabletype_vec_copy,
        delete: wasm_tabletype_vec_delete,
    )
    (
        name: wasm_memorytype_vec_t,
        ty: Option<Box<wasm_memorytype_t>>,
        new: wasm_memorytype_vec_new,
        empty: wasm_memorytype_vec_new_empty,
        uninit: wasm_memorytype_vec_new_uninitialized,
        copy: wasm_memorytype_vec_copy,
        delete: wasm_memorytype_vec_delete,
    )
    (
        name: wasm_externtype_vec_t,
        ty: Option<Box<wasm_externtype_t>>,
        new: wasm_externtype_vec_new,
        empty: wasm_externtype_vec_new_empty,
        uninit: wasm_externtype_vec_new_uninitialized,
        copy: wasm_externtype_vec_copy,
        delete: wasm_externtype_vec_delete,
    )
    (
        name: wasm_importtype_vec_t,
        ty: Option<Box<wasm_importtype_t>>,
        new: wasm_importtype_vec_new,
        empty: wasm_importtype_vec_new_empty,
        uninit: wasm_importtype_vec_new_uninitialized,
        copy: wasm_importtype_vec_copy,
        delete: wasm_importtype_vec_delete,
    )
    (
        name: wasm_exporttype_vec_t,
        ty: Option<Box<wasm_exporttype_t>>,
        new: wasm_exporttype_vec_new,
        empty: wasm_exporttype_vec_new_empty,
        uninit: wasm_exporttype_vec_new_uninitialized,
        copy: wasm_exporttype_vec_copy,
        delete: wasm_exporttype_vec_delete,
    )
    (
        name: wasm_val_vec_t,
        ty: wasm_val_t,
        new: wasm_val_vec_new,
        empty: wasm_val_vec_new_empty,
        uninit: wasm_val_vec_new_uninitialized,
        copy: wasm_val_vec_copy,
        delete: wasm_val_vec_delete,
    )
    (
        name: wasm_frame_vec_t,
        ty: Option<Box<wasm_frame_t>>,
        new: wasm_frame_vec_new,
        empty: wasm_frame_vec_new_empty,
        uninit: wasm_frame_vec_new_uninitialized,
        copy: wasm_frame_vec_copy,
        delete: wasm_frame_vec_delete,
    )
    (
        name: wasm_extern_vec_t,
        ty: Option<Box<wasm_extern_t>>,
        new: wasm_extern_vec_new,
        empty: wasm_extern_vec_new_empty,
        uninit: wasm_extern_vec_new_uninitialized,
        copy: wasm_extern_vec_copy,
        delete: wasm_extern_vec_delete,
    )
}
